use std::io;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use bytes::Bytes;
use log::{debug, warn};
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpStream, UdpSocket};
use tokio::sync::mpsc;

use crate::cli::RxConfig;
use crate::header::{Header, HEADER_LEN};
use crate::reorder::{Datagram, InsertOutcome, Reorder};
use crate::udp::{self, BufferDirection};

const REORDER_TIMEOUT: Duration = Duration::from_millis(250); // OD-5 open: provisional implementation value, not a requirement
const REORDER_MEMORY_CAP: usize = 8 * 1024 * 1024; // OD-5 open: provisional implementation value, not a requirement
const MAX_DATAGRAM_SIZE: usize = 65535;

pub async fn run(config: RxConfig) -> Result<()> {
    let mut sockets = Vec::with_capacity(config.global.streams);
    for stream in 0..config.global.streams {
        let address = SocketAddr::new(config.bind, config.global.base_port + stream as u16);
        let socket = udp::bind(address, config.global.udp_buffer, BufferDirection::Receive)
            .with_context(|| format!("could not bind RX UDP socket at {address}"))?;
        sockets.push(socket);
    }

    let tcp = TcpStream::connect(config.connect)
        .await
        .with_context(|| format!("could not connect TCP output to {}", config.connect))?;
    let (output_sender, output_receiver) = mpsc::unbounded_channel();
    let output_task = tokio::spawn(write_output(tcp, output_receiver));
    let mut reorder = Reorder::new();
    let mut buffer = vec![0_u8; MAX_DATAGRAM_SIZE];

    loop {
        drain_sockets(&sockets, &mut reorder, &mut buffer)?;

        let mut deliveries = Vec::new();
        let mut end_of_stream = collect_ready(&mut reorder, &mut deliveries);
        let now = Instant::now();
        // OD-5 open: after draining every socket, declare each overdue head-of-line
        // gap and deliver what follows, repeating in the same pass while the next
        // gap is overdue, so a dead leg's hundreds of gaps do not take minutes.
        while reorder.head_gap_overdue(now, REORDER_TIMEOUT, REORDER_MEMORY_CAP) {
            let Some(gap) = reorder.declare_loss() else {
                break;
            };
            // PROPOSAL SM-D13: report one gap with its and the running totals.
            warn!(target: "socket_mux::rx",
                "loss gap: first_seq={} lost_slices={} lost_bytes={} total_lost_slices={} total_lost_bytes={}",
                gap.first_sequence,
                gap.lost_slices,
                gap.lost_bytes,
                gap.total_lost_slices,
                gap.total_lost_bytes,
            );
            end_of_stream |= collect_ready(&mut reorder, &mut deliveries);
        }

        for payload in deliveries {
            if !payload.is_empty() && output_sender.send(payload).is_err() {
                bail!("TCP output task stopped before end of stream");
            }
        }
        if end_of_stream {
            drop(output_sender);
            output_task
                .await
                .context("could not join the TCP output task")??;
            return Ok(());
        }
        if output_task.is_finished() {
            output_task
                .await
                .context("could not join the TCP output task")??;
            bail!("TCP output task stopped before end of stream");
        }

        // Poll every socket while a relay delivers bursts; loss still uses the timeout above.
        tokio::task::yield_now().await;
        std::thread::yield_now();
    }
}

async fn write_output(
    mut tcp: TcpStream,
    mut receiver: mpsc::UnboundedReceiver<Bytes>,
) -> Result<()> {
    while let Some(payload) = receiver.recv().await {
        tcp.write_all(&payload)
            .await
            .context("TCP output write failed")?;
    }
    // PROPOSAL SM-D14: close the TCP write side after delivering through EOS.
    tcp.shutdown().await.context("TCP output shutdown failed")?;
    Ok(())
}

fn collect_ready(reorder: &mut Reorder, deliveries: &mut Vec<Bytes>) -> bool {
    let mut end_of_stream = false;
    while let Some(delivery) = reorder.pop_ready() {
        end_of_stream |= delivery.end_of_stream;
        deliveries.push(delivery.payload);
        if end_of_stream {
            break;
        }
    }
    end_of_stream
}

fn drain_sockets(
    sockets: &[UdpSocket],
    reorder: &mut Reorder,
    buffer: &mut [u8],
) -> io::Result<()> {
    loop {
        let mut received_any = false;
        for socket in sockets {
            match socket.try_recv(buffer) {
                Ok(length) => {
                    received_any = true;
                    if let Some(datagram) = decode_datagram(&buffer[..length]) {
                        insert_datagram(reorder, datagram);
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
                Err(error) => return Err(error),
            }
        }
        if !received_any {
            return Ok(());
        }
    }
}

fn decode_datagram(datagram: &[u8]) -> Option<Datagram> {
    let header = match Header::decode(datagram) {
        Ok(header) => header,
        Err(reason) => {
            debug!(target: "socket_mux::rx", "discarding datagram: {reason}");
            return None;
        }
    };
    Some(Datagram {
        header,
        payload: Bytes::copy_from_slice(&datagram[HEADER_LEN..]),
    })
}

fn insert_datagram(reorder: &mut Reorder, datagram: Datagram) {
    let sequence = datagram.header.sequence;
    match reorder.insert(datagram) {
        InsertOutcome::Inserted => {}
        InsertOutcome::Late => {
            debug!(target: "socket_mux::rx", "discarding late datagram: seq={sequence}")
        }
        InsertOutcome::Duplicate => {
            debug!(target: "socket_mux::rx", "discarding duplicate datagram: seq={sequence}")
        }
    }
}
