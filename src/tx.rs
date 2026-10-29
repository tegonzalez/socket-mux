use std::io::{self, Read};
use std::net::{
    IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, TcpListener, TcpStream, ToSocketAddrs, UdpSocket,
};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::thread;

use anyhow::{anyhow, Context, Result};
use log::{debug, info};

use crate::cli::{Global, Input, TxConfig};
use crate::header::{Header, HEADER_LEN, SEQUENCE_MASK};
use crate::pattern::PatternStream;
use crate::udp::{self, BufferDirection};

/// Datagrams handed to a lane thread per channel message; amortises the hand-off.
const BATCH: usize = 32;
/// Batches queued per lane before the producer blocks (bounded memory, back-pressure).
const QUEUE_DEPTH: usize = 4;

/// The one input TX reads (SM-P03, SM-P11), or the generated pattern (SM-P10).
pub enum Reader {
    Tcp(TcpStream),
    Stdin(io::Stdin),
    File(std::fs::File),
    // Boxed: PatternStream holds a block buffer much larger than the other variants.
    Pattern(Box<PatternStream>),
}

impl Reader {
    /// Read up to `buf.len()` bytes; 0 means end of stream. The pattern never ends.
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        loop {
            let result = match self {
                Reader::Tcp(stream) => stream.read(buf).context("TCP read failed"),
                Reader::Stdin(stdin) => {
                    stdin.lock().read(buf).context("standard input read failed")
                }
                Reader::File(file) => file.read(buf).context("input file read failed"),
                Reader::Pattern(pattern) => return Ok(pattern.fill(buf)),
            };
            match result {
                Err(error)
                    if error
                        .downcast_ref::<io::Error>()
                        .is_some_and(|e| e.kind() == io::ErrorKind::Interrupted) => {}
                other => return other,
            }
        }
    }
}

pub fn run(config: TxConfig) -> Result<()> {
    // SM-D16: a file that cannot be opened is a runtime failure (exit 1);
    // open it before binding any socket.
    let file = match &config.input {
        Some(Input::File(path)) => Some(
            std::fs::File::open(path)
                .with_context(|| format!("could not open input file {}", path.to_string_lossy()))?,
        ),
        _ => None,
    };
    let session = crate::pattern::random_session_id();
    info!(target: "socket_mux::tx", "session={session}");
    let lanes = udp_sockets(&config.global, &config.remote)?;
    let reader = match (config.input, file) {
        (Some(Input::Stdin), _) => Reader::Stdin(io::stdin()),
        (Some(Input::File(_)), Some(file)) => Reader::File(file),
        (None, _) => Reader::Tcp(accept_tcp(&config.listen)?),
        (Some(Input::File(_)), None) => unreachable!("the file was opened above"),
    };
    send_stream(lanes, reader, session, config.global.datagram_size)
}

fn accept_tcp(listen: &SocketAddr) -> Result<TcpStream> {
    let listener = TcpListener::bind(listen)
        .with_context(|| format!("could not bind TCP listener at {listen}"))?;
    info!(target: "socket_mux::tx", "listening on {listen}");
    let (tcp, peer) = listener
        .accept()
        .context("could not accept the TCP client")?;
    drop(listener);
    info!(target: "socket_mux::tx", "accepted TCP client {peer}");
    Ok(tcp)
}

/// One blocking UDP socket per lane, connected to base+k so each send skips the
/// per-datagram route lookup of `send_to`.
pub fn udp_sockets(global: &Global, remote: &str) -> Result<Vec<UdpSocket>> {
    let remote_ip = (remote, global.base_port)
        .to_socket_addrs()
        .with_context(|| format!("could not resolve {remote}"))?
        .next()
        .with_context(|| format!("{remote} resolved to no addresses"))?
        .ip(); // SM-D15: use the first resolved remote address.
    let bind_ip = match remote_ip {
        IpAddr::V4(_) => IpAddr::V4(Ipv4Addr::UNSPECIFIED),
        IpAddr::V6(_) => IpAddr::V6(Ipv6Addr::UNSPECIFIED),
    };
    (0..global.lanes)
        .map(|lane| {
            let socket = udp::bind_blocking(
                SocketAddr::new(bind_ip, 0),
                global.udp_buffer,
                BufferDirection::Send,
            )
            .with_context(|| format!("could not create TX UDP socket for lane {lane}"))?;
            // SM-D11: lane k uses base+k.
            let destination = SocketAddr::new(remote_ip, global.base_port + lane as u16);
            socket
                .connect(destination)
                .with_context(|| format!("could not address TX lane {lane} to {destination}"))?;
            Ok(socket)
        })
        .collect()
}

/// A run of up to BATCH datagrams for one lane, stored at a fixed stride.
struct Batch {
    data: Vec<u8>,
    lens: Vec<usize>,
}

impl Batch {
    fn new(datagram_size: usize) -> Self {
        Self {
            data: vec![0_u8; datagram_size * BATCH],
            lens: Vec::with_capacity(BATCH),
        }
    }
}

/// Send one datagram on a connected socket. ICMP port-unreachable surfaces on
/// a connected socket as ConnectionRefused; TX ignores it, as unconnected
/// `send_to` did, so TX may start before RX.
fn send_one(socket: &UdpSocket, datagram: &[u8]) -> io::Result<()> {
    loop {
        match socket.send(datagram) {
            Ok(sent) if sent == datagram.len() => return Ok(()),
            Ok(_) => return Err(io::Error::new(io::ErrorKind::WriteZero, "short UDP send")),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) if error.kind() == io::ErrorKind::ConnectionRefused => return Ok(()),
            Err(error) => return Err(error),
        }
    }
}

fn send_batch(socket: &UdpSocket, batch: &Batch, stride: usize) -> io::Result<()> {
    for (i, &len) in batch.lens.iter().enumerate() {
        send_one(socket, &batch.data[i * stride..i * stride + len])?;
    }
    Ok(())
}

/// Lane thread: send every batch it is handed, then return the buffer.
fn lane_thread(
    lane: usize,
    socket: UdpSocket,
    work: Receiver<Batch>,
    free: SyncSender<Batch>,
    stride: usize,
) -> Result<()> {
    for mut batch in work {
        send_batch(&socket, &batch, stride)
            .with_context(|| format!("UDP send failed on lane {lane}"))?;
        batch.lens.clear();
        // The producer may already be gone at end of stream; that is not an error.
        let _ = free.send(batch);
    }
    Ok(())
}

fn write_header(slot: &mut [u8], session: u16, sequence: u64, offset: u64, len: usize, eos: bool) {
    let header = Header::new(session, len as u16, sequence, offset, eos);
    slot[..HEADER_LEN].copy_from_slice(&header.encode());
}

/// Read the input into datagrams, number them, and send datagram `seq` on lane
/// `seq mod N` (SM-D11 round-robin). With N > 1 each lane is its own OS thread
/// with a blocking socket, so the per-datagram send cost spreads over N cores;
/// the producer (this thread) reads and frames. Ends with an EOS datagram.
pub fn send_stream(
    sockets: Vec<UdpSocket>,
    mut reader: Reader,
    session: u16,
    datagram_size: usize,
) -> Result<()> {
    let lanes = sockets.len();
    let max_payload = datagram_size - HEADER_LEN;

    if lanes == 1 {
        // One lane: no hand-off; send inline.
        let socket = &sockets[0];
        let mut datagram = vec![0_u8; datagram_size];
        let mut sequence = 0_u64;
        let mut offset = 0_u64;
        loop {
            let count = reader.read(&mut datagram[HEADER_LEN..])?;
            write_header(&mut datagram, session, sequence, offset, count, count == 0);
            send_one(socket, &datagram[..HEADER_LEN + count])
                .with_context(|| format!("UDP send failed for sequence {sequence}"))?;
            if count == 0 {
                debug!(target: "socket_mux::tx", "sent EOS at sequence {sequence}, offset {offset}");
                return Ok(());
            }
            sequence = sequence.wrapping_add(1) & SEQUENCE_MASK;
            offset = offset.wrapping_add(count as u64);
        }
    }

    // Pool: each lane may have QUEUE_DEPTH queued, one in flight and one filling.
    let pool = lanes * (QUEUE_DEPTH + 2);
    let (free_tx, free_rx) = sync_channel::<Batch>(pool);
    let mut work = Vec::with_capacity(lanes);
    let mut handles = Vec::with_capacity(lanes);
    for (lane, socket) in sockets.into_iter().enumerate() {
        let (tx, rx) = sync_channel::<Batch>(QUEUE_DEPTH);
        let free = free_tx.clone();
        work.push(tx);
        handles.push(
            thread::Builder::new()
                .name(format!("tx-lane-{lane}"))
                .spawn(move || lane_thread(lane, socket, rx, free, datagram_size))
                .context("could not start a TX lane thread")?,
        );
    }
    for _ in 0..pool {
        free_tx
            .send(Batch::new(datagram_size))
            .expect("pool fits its channel");
    }
    drop(free_tx);

    let produced = produce(
        &mut reader,
        &work,
        &free_rx,
        session,
        datagram_size,
        max_payload,
    );
    drop(work); // lane threads drain their queues and exit
    let mut first_error = produced.err();
    for handle in handles {
        let result = handle
            .join()
            .map_err(|_| anyhow!("a TX lane thread panicked"))
            .and_then(|r| r);
        if let (None, Err(error)) = (&first_error, result) {
            first_error = Some(error);
        }
    }
    match first_error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

fn produce(
    reader: &mut Reader,
    work: &[SyncSender<Batch>],
    free: &Receiver<Batch>,
    session: u16,
    stride: usize,
    max_payload: usize,
) -> Result<()> {
    let lanes = work.len();
    let take = || free.recv().map_err(|_| anyhow!("a TX lane thread stopped"));
    let mut filling: Vec<Batch> = (0..lanes).map(|_| take()).collect::<Result<_>>()?;
    let mut sequence = 0_u64;
    let mut offset = 0_u64;
    loop {
        let lane = (sequence % lanes as u64) as usize;
        let batch = &mut filling[lane];
        let at = batch.lens.len() * stride;
        let slot = &mut batch.data[at..at + stride];
        let count = reader.read(&mut slot[HEADER_LEN..HEADER_LEN + max_payload])?;
        let eos = count == 0;
        write_header(slot, session, sequence, offset, count, eos);
        batch.lens.push(HEADER_LEN + count);
        if eos || batch.lens.len() == BATCH {
            let full = std::mem::replace(&mut filling[lane], take()?);
            work[lane]
                .send(full)
                .map_err(|_| anyhow!("TX lane {lane} stopped"))?;
        }
        if eos {
            // Flush the other lanes' partial batches; their datagrams precede EOS.
            for (other, batch) in filling.into_iter().enumerate() {
                if !batch.lens.is_empty() {
                    work[other]
                        .send(batch)
                        .map_err(|_| anyhow!("TX lane {other} stopped"))?;
                }
            }
            debug!(target: "socket_mux::tx", "sent EOS at sequence {sequence}, offset {offset}");
            return Ok(());
        }
        sequence = sequence.wrapping_add(1) & SEQUENCE_MASK;
        offset = offset.wrapping_add(count as u64);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::header::Header;
    use std::time::Duration;

    /// Run send_stream over `lanes` loopback sockets and collect every datagram.
    fn capture(lanes: usize, input: &[u8], datagram_size: usize) -> Vec<(usize, Header, Vec<u8>)> {
        let receivers: Vec<UdpSocket> = (0..lanes)
            .map(|_| {
                let s = UdpSocket::bind("127.0.0.1:0").unwrap();
                s.set_read_timeout(Some(Duration::from_millis(300)))
                    .unwrap();
                s
            })
            .collect();
        let senders: Vec<UdpSocket> = receivers
            .iter()
            .map(|r| {
                let s = UdpSocket::bind("127.0.0.1:0").unwrap();
                s.connect(r.local_addr().unwrap()).unwrap();
                s
            })
            .collect();
        let path = std::env::temp_dir().join(format!(
            "sm-tx-test-{}-{lanes}-{}",
            std::process::id(),
            input.len()
        ));
        std::fs::write(&path, input).unwrap();
        let reader = Reader::File(std::fs::File::open(&path).unwrap());
        send_stream(senders, reader, 7, datagram_size).unwrap();
        std::fs::remove_file(&path).unwrap();
        let mut out = Vec::new();
        for (lane, r) in receivers.iter().enumerate() {
            let mut buf = vec![0_u8; 65536];
            while let Ok(n) = r.recv(&mut buf) {
                let h = Header::decode(&buf[..n]).unwrap();
                out.push((lane, h, buf[HEADER_LEN..n].to_vec()));
            }
        }
        out.sort_by_key(|(_, h, _)| h.sequence);
        out
    }

    fn check(lanes: usize, len: usize) {
        let datagram_size = 100;
        let input: Vec<u8> = (0..len).map(|i| (i * 7 + 3) as u8).collect();
        let got = capture(lanes, &input, datagram_size);
        let payload = datagram_size - HEADER_LEN;
        let expected = len.div_ceil(payload) + 1; // + EOS
        assert_eq!(got.len(), expected, "lanes={lanes} len={len}");
        let mut rebuilt = Vec::new();
        for (i, (lane, h, body)) in got.iter().enumerate() {
            assert_eq!(h.sequence, i as u64);
            assert_eq!(*lane, i % lanes, "round-robin lane for seq {i}");
            assert_eq!(h.session, 7);
            assert_eq!(h.stream_offset, rebuilt.len() as u64);
            assert_eq!(h.flags & crate::header::FLAG_EOS != 0, i == expected - 1);
            rebuilt.extend_from_slice(body);
        }
        assert_eq!(rebuilt, input);
    }

    #[test]
    fn one_lane_sends_in_order_with_eos() {
        check(1, 1000);
    }

    #[test]
    fn lane_threads_keep_round_robin_offsets_and_eos() {
        for lanes in [2, 3, 4] {
            for len in [
                0,
                1,
                76,
                77,
                76 * BATCH * lanes,
                76 * BATCH * lanes + 5,
                20_000,
            ] {
                check(lanes, len);
            }
        }
    }

    #[test]
    fn send_to_absent_receiver_is_not_an_error() {
        let probe = UdpSocket::bind("127.0.0.1:0").unwrap();
        let dead = probe.local_addr().unwrap();
        drop(probe);
        let s = UdpSocket::bind("127.0.0.1:0").unwrap();
        s.connect(dead).unwrap();
        for _ in 0..4 {
            send_one(&s, b"x").unwrap();
        }
    }
}
