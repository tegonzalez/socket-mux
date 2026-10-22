use std::future::poll_fn;
use std::io;
use std::net::SocketAddr;
use std::task::Poll;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use bytes::Bytes;
use log::{debug, info, warn};
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpStream, UdpSocket};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::{sleep_until, Instant as TokioInstant};

use crate::cli::RxConfig;
use crate::header::{Header, HEADER_LEN};
use crate::reorder::{Datagram, InsertOutcome, LossGap, Reorder};
use crate::udp::{self, BufferDirection};

const REORDER_MEMORY_CAP: usize = 8 * 1024 * 1024; // OD-22 open: provisional implementation value, not a requirement
const MAX_DATAGRAM_SIZE: usize = 65535;
// OD-7 open: provisional implementation value, not a requirement — bounds the
// output channel so a stalled TCP output cannot grow memory without limit
// (OUTPUT_CHANNEL_BOUND 1200-byte datagrams are about 0.3 MB).
const OUTPUT_CHANNEL_BOUND: usize = 256;

pub async fn run(config: RxConfig) -> Result<()> {
    let timeout = config.global.timeout; // SM-D19: the global timeout; only RX uses it.

    // SM-D20: the main loop owns every socket. It waits for readiness on any leg,
    // then drains every ready leg in round-robin order. No forwarding task or
    // bounded leg queue can leave one leg lagging while the others fill reorder.
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
    let (output_sender, output_receiver) = mpsc::channel(OUTPUT_CHANNEL_BOUND);
    let output_task = tokio::spawn(write_output(tcp, output_receiver));
    let mut reorder = Reorder::new();
    // SM-D21 PROPOSAL: rule B starts at the first datagram and restarts on every
    // one on any leg; before the first datagram RX waits without a limit.
    let mut last_datagram: Option<Instant> = None;

    loop {
        // PROPOSAL SM-D21: one deadline, the earlier of rule A's head-gap expiry
        // and rule B's last-arrival expiry. With neither, the timer is disabled.
        let deadline = reorder
            .next_deadline(timeout, REORDER_MEMORY_CAP, last_datagram)
            .map(TokioInstant::from_std);

        tokio::select! {
            first_leg = any_leg_readable(&sockets) => {
                let first_leg = first_leg.context("UDP receive failed")?;
                // A readiness event may cover a burst. Rotate through all legs,
                // taking every datagram currently available from each before the
                // next. Each try_recv loop ends at WouldBlock, then awaits fresh
                // readiness; there is no polling loop or per-leg task.
                drain_ready_legs(&sockets, first_leg, &mut reorder, &mut last_datagram)?;
                if deliver_ready(&mut reorder, &output_sender).await? {
                    return close_output(output_sender, output_task).await;
                }
            }
            () = wait_deadline(deadline) => {
                // OD-22: the memory cap can fire this deadline while the
                // "missing" datagram still waits in a leg's socket, and
                // select! picks randomly between two ready branches. Sweep
                // every leg before declaring anything lost so a lagging leg
                // is drained, not condemned (the bulk-transfer defect M03).
                // A truly lost datagram is in no socket, so the sweep is a
                // no-op for a real gap. Ends at WouldBlock, then re-enters
                // the select: no polling loop (SM-D20).
                drain_ready_legs(&sockets, 0, &mut reorder, &mut last_datagram)?;
                if deliver_ready(&mut reorder, &output_sender).await? {
                    return close_output(output_sender, output_task).await;
                }
                let now = Instant::now();
                if last_datagram.is_some_and(|last| now.saturating_duration_since(last) >= timeout) {
                    // SM-D19 rule B: inject end of stream, including when EOS was
                    // lost. A drop after the last received datagram is invisible.
                    let (gaps, deliveries) = reorder.flush();
                    for gap in &gaps {
                        log_gap(gap);
                    }
                    let total_lost_slices = reorder.total_lost_slices();
                    let total_lost_bytes = reorder.total_lost_bytes();
                    // PROPOSAL SM-D21: one info line naming the timeout and totals.
                    info!(target: "socket_mux::rx",
                        "timeout {timeout:?} expired with no datagrams; ending the stream: \
                         total_lost_slices={total_lost_slices} total_lost_bytes={total_lost_bytes}");
                    send_payloads(
                        deliveries.into_iter().map(|delivery| delivery.payload),
                        &output_sender,
                    )
                    .await?;
                    return close_output(output_sender, output_task).await;
                }

                // SM-D19 rule A: declare a head gap whose predecessor has aged
                // past the timeout. The OD-22 memory cap can declare it sooner.
                declare_overdue_gap(&mut reorder, now, timeout);
                if deliver_ready(&mut reorder, &output_sender).await? {
                    return close_output(output_sender, output_task).await;
                }
            }
        }
        if output_task.is_finished() {
            output_task
                .await
                .context("could not join the TCP output task")??;
            bail!("TCP output task stopped before end of stream");
        }
    }
}

/// Wait until any leg is readable, registering this task with every socket.
/// Returning `Pending` here is an event-driven wait, not a polling loop (SM-D20).
async fn any_leg_readable(sockets: &[UdpSocket]) -> io::Result<usize> {
    poll_fn(|cx| {
        for (leg, socket) in sockets.iter().enumerate() {
            match socket.poll_recv_ready(cx) {
                Poll::Ready(Ok(())) => return Poll::Ready(Ok(leg)),
                Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
                Poll::Pending => {}
            }
        }
        Poll::Pending
    })
    .await
}

/// Drain ready legs in round-robin order. Each leg reaches WouldBlock before the
/// next is visited, and the outer loop awaits readiness again when all are empty.
fn drain_ready_legs(
    sockets: &[UdpSocket],
    first_leg: usize,
    reorder: &mut Reorder,
    last_datagram: &mut Option<Instant>,
) -> Result<()> {
    let mut buffer = vec![0_u8; MAX_DATAGRAM_SIZE];
    for offset in 0..sockets.len() {
        let leg = (first_leg + offset) % sockets.len();
        loop {
            match sockets[leg].try_recv(&mut buffer) {
                Ok(length) => {
                    let arrived = Instant::now();
                    if let Some(datagram) = decode_datagram(&buffer[..length]) {
                        *last_datagram = Some(arrived);
                        insert_datagram(reorder, datagram, arrived);
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                Err(error) => return Err(error).context("UDP receive failed"),
            }
        }
    }
    Ok(())
}

/// Sleep until a deadline; with no deadline, wait forever (a disabled timer).
async fn wait_deadline(deadline: Option<TokioInstant>) {
    match deadline {
        Some(deadline) => sleep_until(deadline).await,
        None => std::future::pending().await,
    }
}

async fn write_output(mut tcp: TcpStream, mut receiver: mpsc::Receiver<Bytes>) -> Result<()> {
    while let Some(payload) = receiver.recv().await {
        tcp.write_all(&payload)
            .await
            .context("TCP output write failed")?;
    }
    // PROPOSAL SM-D14: close the TCP write side after delivering through EOS.
    tcp.shutdown().await.context("TCP output shutdown failed")?;
    Ok(())
}

/// Deliver every payload RX can now release, in order; true when end of stream arrived.
async fn deliver_ready(reorder: &mut Reorder, output: &mpsc::Sender<Bytes>) -> Result<bool> {
    let mut end_of_stream = false;
    let mut payloads = Vec::new();
    while let Some(delivery) = reorder.pop_ready() {
        end_of_stream |= delivery.end_of_stream;
        payloads.push(delivery.payload);
        if end_of_stream {
            break;
        }
    }
    send_payloads(payloads, output).await?;
    Ok(end_of_stream)
}

async fn send_payloads(
    payloads: impl IntoIterator<Item = Bytes>,
    output: &mpsc::Sender<Bytes>,
) -> Result<()> {
    for payload in payloads {
        if !payload.is_empty() && output.send(payload).await.is_err() {
            bail!("TCP output task stopped before end of stream");
        }
    }
    Ok(())
}

async fn close_output(sender: mpsc::Sender<Bytes>, task: JoinHandle<Result<()>>) -> Result<()> {
    drop(sender);
    task.await.context("could not join the TCP output task")?
}

/// SM-D19 and OD-22: declare the head-of-line gap if it is overdue. RX delivers
/// what this releases, then recomputes the single deadline for the next gap.
fn declare_overdue_gap(reorder: &mut Reorder, now: Instant, timeout: Duration) {
    if reorder.head_gap_overdue(now, timeout, REORDER_MEMORY_CAP) {
        if let Some(gap) = reorder.declare_loss() {
            log_gap(&gap);
        }
    }
}

// PROPOSAL SM-D13: report one gap with its and the running totals.
fn log_gap(gap: &LossGap) {
    warn!(target: "socket_mux::rx",
        "loss gap: first_seq={} lost_slices={} lost_bytes={} total_lost_slices={} total_lost_bytes={}",
        gap.first_sequence,
        gap.lost_slices,
        gap.lost_bytes,
        gap.total_lost_slices,
        gap.total_lost_bytes,
    );
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

fn insert_datagram(reorder: &mut Reorder, datagram: Datagram, arrived: Instant) {
    let sequence = datagram.header.sequence;
    match reorder.insert_at(datagram, arrived) {
        InsertOutcome::Inserted => {}
        InsertOutcome::Late => {
            debug!(target: "socket_mux::rx", "discarding late datagram: seq={sequence}")
        }
        InsertOutcome::Duplicate => {
            debug!(target: "socket_mux::rx", "discarding duplicate datagram: seq={sequence}")
        }
    }
}
