use std::io;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use anyhow::{bail, Context, Result};
use log::{debug, info};
use tokio::io::{AsyncReadExt, Stdin};
use tokio::net::{lookup_host, TcpListener, TcpStream, UdpSocket};

use crate::cli::{Global, Input, TxConfig};
use crate::header::{Header, HEADER_LEN, SEQUENCE_MASK};
use crate::pattern::PatternStream;
use crate::udp::{self, BufferDirection};

/// The one input TX reads (SM-P03, SM-P11), or the generated pattern (SM-P10).
pub enum Reader {
    Tcp(TcpStream),
    Stdin(Stdin),
    File(tokio::fs::File),
    // Boxed: PatternStream holds a block buffer much larger than the other variants.
    Pattern(Box<PatternStream>),
}

impl Reader {
    /// Read up to `buf.len()` bytes; 0 means end of stream. The pattern never ends.
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        match self {
            Reader::Tcp(stream) => stream.read(buf).await.context("TCP read failed"),
            Reader::Stdin(stdin) => stdin.read(buf).await.context("standard input read failed"),
            Reader::File(file) => file.read(buf).await.context("input file read failed"),
            Reader::Pattern(pattern) => Ok(pattern.fill(buf)),
        }
    }
}

pub async fn run(config: TxConfig) -> Result<()> {
    // SM-D16: a file that cannot be opened is a runtime failure (exit 1);
    // open it before binding any socket.
    let file = match &config.input {
        Some(Input::File(path)) => Some(
            tokio::fs::File::open(path)
                .await
                .with_context(|| format!("could not open input file {}", path.to_string_lossy()))?,
        ),
        _ => None,
    };
    let session = crate::pattern::random_session_id();
    info!(target: "socket_mux::tx", "session={session}");
    let (sockets, destinations) = udp_sockets(&config.global, &config.remote).await?;
    let mut reader = match (config.input, file) {
        (Some(Input::Stdin), _) => Reader::Stdin(tokio::io::stdin()),
        (Some(Input::File(_)), Some(file)) => Reader::File(file),
        (None, _) => Reader::Tcp(accept_tcp(&config.listen).await?),
        (Some(Input::File(_)), None) => unreachable!("the file was opened above"),
    };
    send_stream(
        &sockets,
        &destinations,
        &mut reader,
        session,
        config.global.datagram_size,
    )
    .await
}

async fn accept_tcp(listen: &SocketAddr) -> Result<TcpStream> {
    let listener = TcpListener::bind(listen)
        .await
        .with_context(|| format!("could not bind TCP listener at {listen}"))?;
    info!(target: "socket_mux::tx", "listening on {listen}");
    let (tcp, peer) = listener
        .accept()
        .await
        .context("could not accept the TCP client")?;
    drop(listener);
    info!(target: "socket_mux::tx", "accepted TCP client {peer}");
    Ok(tcp)
}

pub async fn udp_sockets(
    global: &Global,
    remote: &str,
) -> Result<(Vec<UdpSocket>, Vec<SocketAddr>)> {
    let mut resolved = lookup_host((remote, global.base_port))
        .await
        .with_context(|| format!("could not resolve {remote}"))?;
    let remote = resolved
        .next()
        .with_context(|| format!("{remote} resolved to no addresses"))?;
    let remote_ip = remote.ip(); // SM-D15: use the first resolved remote address.

    let mut sockets = Vec::with_capacity(global.lanes);
    let mut destinations = Vec::with_capacity(global.lanes);
    for lane in 0..global.lanes {
        let port = global.base_port + lane as u16;
        let bind_ip = match remote_ip {
            IpAddr::V4(_) => IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            IpAddr::V6(_) => IpAddr::V6(Ipv6Addr::UNSPECIFIED),
        };
        let socket = udp::bind(
            SocketAddr::new(bind_ip, 0),
            global.udp_buffer,
            BufferDirection::Send,
        )
        .with_context(|| format!("could not create TX UDP socket for lane {lane}"))?;
        sockets.push(socket);
        destinations.push(SocketAddr::new(remote_ip, port)); // SM-D11: lane k uses base+k.
    }
    Ok((sockets, destinations))
}

pub async fn send_stream(
    sockets: &[UdpSocket],
    destinations: &[SocketAddr],
    reader: &mut Reader,
    session: u16,
    datagram_size: usize,
) -> Result<()> {
    let mut sequence = 0_u64;
    let mut stream_offset = 0_u64;
    let mut datagram = vec![0_u8; datagram_size];
    let max_payload = datagram_size - HEADER_LEN;
    loop {
        let count = reader
            .read(&mut datagram[HEADER_LEN..HEADER_LEN + max_payload])
            .await?;
        if count == 0 {
            break;
        }
        send_datagram(
            sockets,
            destinations,
            &mut datagram,
            SendInfo {
                session,
                sequence,
                stream_offset,
                payload_len: count,
                eos: false,
            },
        )
        .await?;
        sequence = sequence.wrapping_add(1) & SEQUENCE_MASK;
        stream_offset = stream_offset.wrapping_add(count as u64);
    }

    send_datagram(
        sockets,
        destinations,
        &mut datagram,
        SendInfo {
            session,
            sequence,
            stream_offset,
            payload_len: 0,
            eos: true,
        },
    )
    .await?;
    debug!(target: "socket_mux::tx", "sent EOS at sequence {sequence}, offset {stream_offset}");
    Ok(())
}

struct SendInfo {
    session: u16,
    sequence: u64,
    stream_offset: u64,
    payload_len: usize,
    eos: bool,
}

async fn send_datagram(
    sockets: &[UdpSocket],
    destinations: &[SocketAddr],
    datagram: &mut [u8],
    info: SendInfo,
) -> Result<()> {
    let header = Header::new(
        info.session,
        info.payload_len as u16,
        info.sequence,
        info.stream_offset,
        info.eos,
    );
    datagram[..HEADER_LEN].copy_from_slice(&header.encode());
    let datagram = &datagram[..HEADER_LEN + info.payload_len];
    let lane = info.sequence as usize % sockets.len();
    let sent = sockets[lane]
        .send_to(datagram, destinations[lane])
        .await
        .with_context(|| format!("UDP send failed for sequence {}", info.sequence))?;
    if sent != datagram.len() {
        bail!(io::Error::new(
            io::ErrorKind::WriteZero,
            format!("short UDP send for sequence {}", info.sequence),
        ));
    }
    Ok(())
}
