use std::io;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use anyhow::{bail, Context, Result};
use log::{debug, info};
use tokio::io::{AsyncReadExt, Stdin};
use tokio::net::{lookup_host, TcpListener, TcpStream, UdpSocket};

use crate::cli::{Global, Input, TxConfig};
use crate::header::{Header, HEADER_LEN};
use crate::pattern::PatternStream;
use crate::udp::{self, BufferDirection};

pub const MAX_PAYLOAD_SIZE: usize = 1200; // OD-3 open: provisional implementation value, not a requirement

/// The one input TX reads (SM-P03, SM-P11), or the generated pattern (SM-P10).
pub enum Reader {
    Tcp(TcpStream),
    Stdin(Stdin),
    File(tokio::fs::File),
    Pattern(PatternStream),
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
    // PROPOSAL SM-D16: a file that cannot be opened is a runtime failure (exit 1);
    // open it before binding any socket.
    let file = match &config.input {
        Some(Input::File(path)) => Some(
            tokio::fs::File::open(path)
                .await
                .with_context(|| format!("could not open input file {}", path.to_string_lossy()))?,
        ),
        _ => None,
    };
    let (sockets, destinations) = udp_sockets(&config.global, &config.remote).await?;
    let mut reader = match (config.input, file) {
        (Some(Input::Stdin), _) => Reader::Stdin(tokio::io::stdin()),
        (Some(Input::File(_)), Some(file)) => Reader::File(file),
        (None, _) => Reader::Tcp(accept_tcp(&config.listen).await?),
        (Some(Input::File(_)), None) => unreachable!("the file was opened above"),
    };
    send_stream(&sockets, &destinations, &mut reader).await
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
    let remote_ip = remote.ip(); // PROPOSAL SM-D15: use the first resolved remote address.

    let mut sockets = Vec::with_capacity(global.streams);
    let mut destinations = Vec::with_capacity(global.streams);
    for stream in 0..global.streams {
        let port = global.base_port + stream as u16;
        let bind_ip = match remote_ip {
            IpAddr::V4(_) => IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            IpAddr::V6(_) => IpAddr::V6(Ipv6Addr::UNSPECIFIED),
        };
        let socket = udp::bind(
            SocketAddr::new(bind_ip, 0),
            global.udp_buffer,
            BufferDirection::Send,
        )
        .with_context(|| format!("could not create TX UDP socket for stream {stream}"))?;
        sockets.push(socket);
        destinations.push(SocketAddr::new(remote_ip, port)); // SM-D11: stream k uses base+k.
    }
    Ok((sockets, destinations))
}

pub async fn send_stream(
    sockets: &[UdpSocket],
    destinations: &[SocketAddr],
    reader: &mut Reader,
) -> Result<()> {
    let mut sequence = 0_u64;
    let mut stream_offset = 0_u64;
    let mut payload = vec![0_u8; MAX_PAYLOAD_SIZE];
    loop {
        let count = reader.read(&mut payload).await?;
        if count == 0 {
            break;
        }
        send_datagram(
            sockets,
            destinations,
            sequence,
            stream_offset,
            &payload[..count],
            false,
        )
        .await?;
        sequence += 1;
        stream_offset += count as u64;
    }

    // PROPOSAL SM-D10: send exactly one EOS datagram, including for an empty stream.
    // A pattern reader never reports end of stream, so tx-pat never reaches this.
    send_datagram(sockets, destinations, sequence, stream_offset, &[], true).await?;
    debug!(target: "socket_mux::tx", "sent EOS at sequence {sequence}, offset {stream_offset}");
    Ok(())
}

async fn send_datagram(
    sockets: &[UdpSocket],
    destinations: &[SocketAddr],
    sequence: u64,
    stream_offset: u64,
    payload: &[u8],
    eos: bool,
) -> Result<()> {
    let header = Header::new(payload.len() as u16, sequence, stream_offset, eos);
    let mut datagram = Vec::with_capacity(HEADER_LEN + payload.len());
    datagram.extend_from_slice(&header.encode());
    datagram.extend_from_slice(payload);
    // PROPOSAL SM-D9: assign global sequence s to stream s % N.
    let stream = sequence as usize % sockets.len();
    let sent = sockets[stream]
        .send_to(&datagram, destinations[stream])
        .await
        .with_context(|| format!("UDP send failed for sequence {sequence}"))?;
    if sent != datagram.len() {
        bail!(io::Error::new(
            io::ErrorKind::WriteZero,
            format!("short UDP send for sequence {sequence}"),
        ));
    }
    Ok(())
}
