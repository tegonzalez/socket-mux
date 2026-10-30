use std::io::{self, Read};
use std::net::{
    IpAddr, Ipv4Addr, Ipv6Addr, Shutdown, SocketAddr, TcpListener, TcpStream, ToSocketAddrs,
    UdpSocket,
};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use log::{debug, info};

use crate::cli::{Global, Input, TxConfig};
use crate::header::{Header, HEADER_LEN, SEQUENCE_MASK};
use crate::pattern::PatternStream;
use crate::queue::Bounded;
use crate::report::{Meter, Reporter, Role};
use crate::signals;
use crate::stop::Stop;
use crate::udp::{self, BufferDirection};
use crate::wait;

/// Datagrams handed to a lane thread per queue entry; amortises the hand-off.
const BATCH: usize = 32;
/// Batches queued per lane before the producer blocks (bounded memory, back-pressure).
const QUEUE_DEPTH: usize = 4;

/// The one input TX reads (spec S-03, spec S-11), or the generated pattern (spec S-10).
pub enum Reader {
    Tcp(TcpStream),
    Stdin(io::Stdin),
    File(std::fs::File),
    // Boxed: PatternStream holds a block buffer much larger than the other variants.
    // The limit, if any, is the pattern bytes still to send before end of stream.
    Pattern(Box<PatternStream>, Option<u64>),
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
                Reader::Pattern(pattern, None) => return Ok(pattern.fill(buf)),
                Reader::Pattern(pattern, Some(left)) => {
                    let take = (*left).min(buf.len() as u64) as usize;
                    *left -= take as u64;
                    return Ok(pattern.fill(&mut buf[..take]));
                }
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

pub fn run(config: TxConfig, stop: Arc<Stop>) -> Result<()> {
    // design TX Input Operand: a file that cannot be opened is a runtime failure (exit 1);
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
    with_report(&config.global, |meter| {
        let reader = match (config.input, file) {
            (Some(Input::Stdin), _) => Reader::Stdin(io::stdin()),
            (Some(Input::File(_)), Some(file)) => Reader::File(file),
            (None, _) => match accept_tcp(&config.listen, &stop)? {
                Some(tcp) => Reader::Tcp(tcp),
                None => return Ok(()), // stopped while waiting for the client
            },
            (Some(Input::File(_)), None) => unreachable!("the file was opened above"),
        };
        send_stream(
            lanes,
            reader,
            session,
            config.global.datagram_size,
            Pacer::unpaced(),
            &stop,
            meter,
        )
    })
}

/// Run `send` with a TX meter and reporter; the summary is printed however
/// the run ends (and by the signal watchdog if the run is stuck).
pub fn with_report(global: &Global, send: impl FnOnce(&Arc<Meter>) -> Result<()>) -> Result<()> {
    let meter = Meter::new();
    let reporter = Reporter::spawn(Role::Tx, meter.clone(), global.interval)?;
    {
        let reporter = reporter.clone();
        signals::set_finisher(move || reporter.finish());
    }
    let result = send(&meter);
    reporter.finish();
    result
}

/// Accept the one TCP client; None if `stop` is set first. A stop wakes the
/// blocking accept by connecting to the listener, and later unblocks reads by
/// shutting the accepted connection.
fn accept_tcp(listen: &SocketAddr, stop: &Arc<Stop>) -> Result<Option<TcpStream>> {
    let listener = TcpListener::bind(listen)
        .with_context(|| format!("could not bind TCP listener at {listen}"))?;
    info!(target: "socket_mux::tx", "listening on {listen}");
    let address = listener.local_addr().unwrap_or(*listen);
    stop.on_stop(move || {
        let _ = TcpStream::connect_timeout(&address, Duration::from_secs(1));
    });
    let (tcp, peer) = listener
        .accept()
        .context("could not accept the TCP client")?;
    drop(listener);
    if stop.is_set() {
        return Ok(None);
    }
    info!(target: "socket_mux::tx", "accepted TCP client {peer}");
    if let Ok(clone) = tcp.try_clone() {
        stop.on_stop(move || {
            let _ = clone.shutdown(Shutdown::Both);
        });
    }
    Ok(Some(tcp))
}

/// One blocking UDP socket per lane, connected to base+k so each send skips the
/// per-datagram route lookup of `send_to`.
pub fn udp_sockets(global: &Global, remote: &str) -> Result<Vec<UdpSocket>> {
    let remote_ip = (remote, global.base_port)
        .to_socket_addrs()
        .with_context(|| format!("could not resolve {remote}"))?
        .next()
        .with_context(|| format!("{remote} resolved to no addresses"))?
        .ip(); // design Command-Line Options: use the first resolved remote address.
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
            // design Round-Robin Slice Assignment: lane k uses base+k.
            let destination = SocketAddr::new(remote_ip, global.base_port + lane as u16);
            socket
                .connect(destination)
                .with_context(|| format!("could not address TX lane {lane} to {destination}"))?;
            Ok(socket)
        })
        .collect()
}

/// TX pacing (`tx-pat --ipd-ns`): datagram i (counted across all lanes) is
/// due at start + i * ipd. A sender ahead of schedule waits with the shared
/// due-time policy ([`wait::wait_due`]): it sleeps while far from the due
/// time, yields when close, and spins only for the last few microseconds. With
/// ipd 0 it never waits. It only runs while sending, so idle TX uses no CPU.
#[derive(Clone, Copy)]
pub struct Pacer {
    start: Instant,
    ipd_ns: u64,
}

impl Pacer {
    pub fn new(ipd_ns: u64) -> Self {
        Self {
            start: Instant::now(),
            ipd_ns,
        }
    }

    pub fn unpaced() -> Self {
        Self::new(0)
    }

    fn is_paced(&self) -> bool {
        self.ipd_ns > 0
    }

    /// Wait until datagram `index` is due; false if `stop` was set first.
    fn wait_for(&self, index: u64, stop: &AtomicBool) -> bool {
        if self.ipd_ns == 0 {
            return true;
        }
        let due = self.start + Duration::from_nanos(index.saturating_mul(self.ipd_ns));
        wait::wait_due(due, stop)
    }
}

/// A run of up to BATCH datagrams for one lane, stored at a fixed stride.
/// Datagram j of the batch is stream datagram `first + j * lanes`.
struct Batch {
    data: Vec<u8>,
    lens: Vec<usize>,
    first: u64,
}

impl Batch {
    fn new(datagram_size: usize) -> Self {
        Self {
            data: vec![0_u8; datagram_size * BATCH],
            lens: Vec::with_capacity(BATCH),
            first: 0,
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

/// Read the next payload; Ok(None) when `stop` is set (a stop may surface
/// as a read error or an early end of a shut TCP input; neither is EOS).
fn read_payload(reader: &mut Reader, buf: &mut [u8], stop: &Stop) -> Result<Option<usize>> {
    match reader.read(buf) {
        _ if stop.is_set() => Ok(None),
        Ok(count) => Ok(Some(count)),
        Err(error) => Err(error),
    }
}

/// Payload bytes of datagrams with these lengths.
fn batch_payload(lens: &[usize]) -> u64 {
    lens.iter().map(|len| (len - HEADER_LEN) as u64).sum()
}

/// Send a batch; returns how many of its datagrams went out (fewer when a
/// stop interrupted it).
fn send_batch(
    socket: &UdpSocket,
    batch: &Batch,
    stride: usize,
    lanes: u64,
    pacer: &Pacer,
    stop: &Stop,
) -> io::Result<usize> {
    for (i, &len) in batch.lens.iter().enumerate() {
        if !pacer.wait_for(batch.first + i as u64 * lanes, stop.flag()) || stop.is_set() {
            return Ok(i);
        }
        send_one(socket, &batch.data[i * stride..i * stride + len])?;
    }
    Ok(batch.lens.len())
}

/// Lane thread: send every batch it is handed, then return the buffer. A send
/// error stops the whole run so the producer does not wait for this lane.
#[allow(clippy::too_many_arguments)]
fn lane_thread(
    lane: usize,
    socket: UdpSocket,
    work: &Bounded<Batch>,
    free: &Bounded<Batch>,
    stride: usize,
    lanes: u64,
    pacer: Pacer,
    stop: &Stop,
    meter: &Meter,
) -> Result<()> {
    if pacer.is_paced() {
        wait::fine_timer_slack();
    }
    while let Some(mut batch) = work.pop(stop.flag()) {
        match send_batch(&socket, &batch, stride, lanes, &pacer, stop) {
            Ok(sent) if sent == batch.lens.len() => {
                meter.add_data(sent as u64, batch_payload(&batch.lens))
            }
            Ok(sent) => {
                meter.add_data(sent as u64, batch_payload(&batch.lens[..sent]));
                return Ok(());
            }
            Err(error) => {
                stop.set();
                return Err(error).with_context(|| format!("UDP send failed on lane {lane}"));
            }
        }
        batch.lens.clear();
        // The free queue holds the whole pool, so this never finds it full.
        let _ = free.try_push(batch);
    }
    Ok(())
}

fn write_header(slot: &mut [u8], session: u16, sequence: u64, offset: u64, len: usize, eos: bool) {
    let header = Header::new(session, len as u16, sequence, offset, eos);
    slot[..HEADER_LEN].copy_from_slice(&header.encode());
}

/// Read the input into datagrams, number them, and send datagram `seq` on lane
/// `seq mod N` (design Round-Robin Slice Assignment). With N > 1 each lane is its own OS thread
/// with a blocking socket, so the per-datagram send cost spreads over N cores;
/// the producer (this thread) reads and frames. Ends with an EOS datagram,
/// unless `stop` is set first: then it returns Ok without sending EOS.
pub fn send_stream(
    sockets: Vec<UdpSocket>,
    mut reader: Reader,
    session: u16,
    datagram_size: usize,
    pacer: Pacer,
    stop: &Arc<Stop>,
    meter: &Arc<Meter>,
) -> Result<()> {
    let lanes = sockets.len();
    let max_payload = datagram_size - HEADER_LEN;

    if lanes == 1 {
        // One lane: no hand-off; send inline.
        if pacer.is_paced() {
            wait::fine_timer_slack();
        }
        let socket = &sockets[0];
        let mut datagram = vec![0_u8; datagram_size];
        let mut sequence = 0_u64;
        let mut offset = 0_u64;
        let mut index = 0_u64;
        // Sent counts reach the meter every 64 datagrams, not per datagram.
        let mut unmetered = (0_u64, 0_u64);
        let flush = |unmetered: &mut (u64, u64)| {
            meter.add_data(unmetered.0, unmetered.1);
            *unmetered = (0, 0);
        };
        loop {
            let Some(count) = read_payload(&mut reader, &mut datagram[HEADER_LEN..], stop)? else {
                flush(&mut unmetered);
                return Ok(());
            };
            if index == 0 {
                meter.start(Instant::now());
            }
            if !pacer.wait_for(index, stop.flag()) {
                flush(&mut unmetered);
                return Ok(());
            }
            index += 1;
            write_header(&mut datagram, session, sequence, offset, count, count == 0);
            let sent = send_one(socket, &datagram[..HEADER_LEN + count]);
            unmetered = (unmetered.0 + 1, unmetered.1 + count as u64);
            if sent.is_err() || count == 0 || unmetered.0 == 64 {
                flush(&mut unmetered);
            }
            sent.with_context(|| format!("UDP send failed for sequence {sequence}"))?;
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
    let free = Arc::new(Bounded::<Batch>::new(pool));
    stop.watch(free.not_empty().clone());
    for _ in 0..pool {
        if free.try_push(Batch::new(datagram_size)).is_err() {
            unreachable!("the pool fits its free queue");
        }
    }
    let mut work = Vec::with_capacity(lanes);
    let mut handles = Vec::with_capacity(lanes);
    for (lane, socket) in sockets.into_iter().enumerate() {
        let queue = Arc::new(Bounded::<Batch>::new(QUEUE_DEPTH));
        stop.watch(queue.not_empty().clone());
        stop.watch(queue.not_full().clone());
        work.push(queue.clone());
        let (free, stop) = (free.clone(), stop.clone());
        let meter = meter.clone();
        handles.push(
            thread::Builder::new()
                .name(format!("tx-lane-{lane}"))
                .spawn(move || {
                    lane_thread(
                        lane,
                        socket,
                        &queue,
                        &free,
                        datagram_size,
                        lanes as u64,
                        pacer,
                        &stop,
                        &meter,
                    )
                })
                .context("could not start a TX lane thread")?,
        );
    }

    let produced = produce(
        &mut reader,
        &work,
        &free,
        session,
        datagram_size,
        max_payload,
        stop,
        meter,
    );
    for queue in &work {
        queue.close(); // lane threads drain their queues and exit
    }
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

/// The producer: frames the input into per-lane batches. Returns Ok early
/// (no EOS) when `stop` is set.
#[allow(clippy::too_many_arguments)]
fn produce(
    reader: &mut Reader,
    work: &[Arc<Bounded<Batch>>],
    free: &Bounded<Batch>,
    session: u16,
    stride: usize,
    max_payload: usize,
    stop: &Stop,
    meter: &Meter,
) -> Result<()> {
    let lanes = work.len();
    let take = || free.pop(stop.flag());
    let mut filling = Vec::with_capacity(lanes);
    for _ in 0..lanes {
        match take() {
            Some(batch) => filling.push(batch),
            None => return Ok(()),
        }
    }
    let mut sequence = 0_u64;
    let mut offset = 0_u64;
    let mut index = 0_u64; // unmasked datagram count: lane and pacing schedule
    loop {
        let lane = (index % lanes as u64) as usize;
        let batch = &mut filling[lane];
        if batch.lens.is_empty() {
            batch.first = index;
        }
        let at = batch.lens.len() * stride;
        let slot = &mut batch.data[at..at + stride];
        let Some(count) = read_payload(
            reader,
            &mut slot[HEADER_LEN..HEADER_LEN + max_payload],
            stop,
        )?
        else {
            return Ok(());
        };
        if index == 0 {
            meter.start(Instant::now());
        }
        let eos = count == 0;
        write_header(slot, session, sequence, offset, count, eos);
        batch.lens.push(HEADER_LEN + count);
        if eos || batch.lens.len() == BATCH {
            let Some(next) = take() else {
                return Ok(());
            };
            let full = std::mem::replace(&mut filling[lane], next);
            if work[lane].push(full, stop.flag()).is_err() {
                return Ok(());
            }
        }
        if eos {
            // Flush the other lanes' partial batches; their datagrams precede EOS.
            for (other, batch) in filling.into_iter().enumerate() {
                if !batch.lens.is_empty() && work[other].push(batch, stop.flag()).is_err() {
                    return Ok(());
                }
            }
            debug!(target: "socket_mux::tx", "sent EOS at sequence {sequence}, offset {offset}");
            return Ok(());
        }
        sequence = sequence.wrapping_add(1) & SEQUENCE_MASK;
        offset = offset.wrapping_add(count as u64);
        index += 1;
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
        send_stream(
            senders,
            reader,
            7,
            datagram_size,
            Pacer::unpaced(),
            &Stop::new(),
            &Meter::new(),
        )
        .unwrap();
        std::fs::remove_file(&path).unwrap();
        let mut out = Vec::new();
        for (lane, r) in receivers.iter().enumerate() {
            let mut buf = vec![0_u8; 65536];
            while let Ok(n) = r.recv(&mut buf) {
                let h = Header::decode(&buf[..n]).unwrap();
                out.push((lane, h, buf[HEADER_LEN..n].to_vec()));
            }
        }
        // Stream offsets are unique and never wrap here; sequences do in
        // reduced-width builds.
        out.sort_by_key(|(_, h, _)| h.stream_offset);
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
            assert_eq!(h.sequence, i as u64 & SEQUENCE_MASK);
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
    fn pacer_spaces_datagrams_by_ipd() {
        let stop = AtomicBool::new(false);
        let pacer = Pacer::new(50_000); // 50 us
        let begin = Instant::now();
        for i in 0..200 {
            assert!(pacer.wait_for(i, &stop));
        }
        let elapsed = begin.elapsed();
        // 199 intervals of 50 us = 9.95 ms; allow scheduler slack upward only.
        assert!(elapsed >= Duration::from_micros(9_900), "{elapsed:?}");
        assert!(elapsed < Duration::from_millis(60), "{elapsed:?}");
        let unpaced = Pacer::unpaced();
        let begin = Instant::now();
        for i in 0..1_000_000 {
            unpaced.wait_for(i, &stop);
        }
        assert!(begin.elapsed() < Duration::from_millis(50));
    }

    #[test]
    fn limited_pattern_ends_with_eos_after_exact_bytes() {
        let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
        receiver
            .set_read_timeout(Some(Duration::from_millis(300)))
            .unwrap();
        let sender = UdpSocket::bind("127.0.0.1:0").unwrap();
        sender.connect(receiver.local_addr().unwrap()).unwrap();
        let limit = 10_000_u64;
        let reader = Reader::Pattern(Box::new(PatternStream::new(42)), Some(limit));
        send_stream(
            vec![sender],
            reader,
            9,
            1452,
            Pacer::unpaced(),
            &Stop::new(),
            &Meter::new(),
        )
        .unwrap();
        let mut total = 0_u64;
        let mut buf = vec![0_u8; 2048];
        let mut last = None;
        while let Ok(n) = receiver.recv(&mut buf) {
            let h = Header::decode(&buf[..n]).unwrap();
            total += (n - HEADER_LEN) as u64;
            last = Some(h);
        }
        assert_eq!(total, limit);
        assert_ne!(last.unwrap().flags & crate::header::FLAG_EOS, 0);
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
