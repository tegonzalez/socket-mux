use std::future::poll_fn;
use std::io;
use std::net::SocketAddr;
use std::task::Poll;
use std::time::Instant;

use anyhow::{bail, Context, Result};
use log::{debug, info, warn};
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpStream, UdpSocket};
use tokio::time::{sleep_until, Instant as TokioInstant};

use crate::cli::RxConfig;
use crate::header::Header;
use crate::reorder::{InsertOutcome, Reorder};
use crate::udp::{self, BufferDirection};

pub async fn run(config: RxConfig) -> Result<()> {
    let timeout = config.global.timeout;
    let mut sockets = Vec::with_capacity(config.global.lanes);
    for lane in 0..config.global.lanes {
        let address = SocketAddr::new(config.bind, config.global.base_port + lane as u16);
        let socket = udp::bind(address, config.global.udp_buffer, BufferDirection::Receive)
            .with_context(|| format!("could not bind RX UDP socket at {address}"))?;
        sockets.push(socket);
    }

    let mut tcp = TcpStream::connect(config.connect)
        .await
        .with_context(|| format!("could not connect TCP output to {}", config.connect))?;
    let mut reorder = Reorder::new(config.global.datagram_size, config.global.hold_capacity)
        .context("could not allocate RX memory pool")?;
    let mut last_datagram = None;
    let mut session = None;
    let mut end_after_flush = false;

    loop {
        if let Some(eos) = finish_empty_outputs(&mut reorder) {
            if eos {
                tcp.shutdown().await.context("TCP output shutdown failed")?;
                return Ok(());
            }
        }
        if end_after_flush && !reorder.has_output() {
            tcp.shutdown().await.context("TCP output shutdown failed")?;
            return Ok(());
        }

        let accepting = !end_after_flush && !reorder.stream_ended();
        let deadline = if accepting {
            reorder
                .next_deadline(timeout, last_datagram)
                .map(TokioInstant::from_std)
        } else {
            None
        };
        tokio::select! {
            first_lane = any_lane_readable(&sockets), if accepting => {
                let first_lane = first_lane.context("UDP receive failed")?;
                if drain_ready_lanes(
                    &sockets,
                    first_lane,
                    &mut reorder,
                    &mut last_datagram,
                    &mut session,
                )? {
                    reorder.flush();
                    end_after_flush = true;
                }
            }
            () = wait_deadline(deadline), if accepting => {
                if drain_ready_lanes(
                    &sockets,
                    0,
                    &mut reorder,
                    &mut last_datagram,
                    &mut session,
                )? {
                    reorder.flush();
                    end_after_flush = true;
                } else {
                    let now = Instant::now();
                    if last_datagram.is_some_and(|last| now.saturating_duration_since(last) >= timeout) {
                        reorder.flush();
                        info!(target: "socket_mux::rx",
                            "timeout {timeout:?} expired with no datagrams; ending the stream: total_lost_slices={} total_lost_bytes={}",
                            reorder.total_lost_slices(), reorder.total_lost_bytes());
                        end_after_flush = true;
                    } else {
                        reorder.expire_gap(now, timeout);
                    }
                }
            }
            ready = tcp.writable(), if reorder.has_output() => {
                ready.context("TCP output readiness failed")?;
                if write_ready(&mut tcp, &mut reorder)? {
                    tcp.shutdown().await.context("TCP output shutdown failed")?;
                    return Ok(());
                }
            }
        }
    }
}

async fn any_lane_readable(sockets: &[UdpSocket]) -> io::Result<usize> {
    poll_fn(|cx| {
        for (lane, socket) in sockets.iter().enumerate() {
            match socket.poll_recv_ready(cx) {
                Poll::Ready(Ok(())) => return Poll::Ready(Ok(lane)),
                Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
                Poll::Pending => {}
            }
        }
        Poll::Pending
    })
    .await
}

fn drain_ready_lanes(
    sockets: &[UdpSocket],
    first_lane: usize,
    reorder: &mut Reorder,
    last_datagram: &mut Option<Instant>,
    session: &mut Option<u16>,
) -> Result<bool> {
    let mut session_changed = false;
    for offset in 0..sockets.len() {
        let lane = (first_lane + offset) % sockets.len();
        loop {
            let buffer = reorder.take_receive_buffer();
            let receive_into_pool = buffer.unwrap_or_else(|| reorder.reserved_receive_buffer());
            let (length, arrived) =
                match sockets[lane].try_recv(reorder.buffer_mut(receive_into_pool)) {
                    Ok(length) => (length, Instant::now()),
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        if let Some(allocated) = buffer {
                            reorder.release_receive_buffer(allocated);
                        }
                        break;
                    }
                    Err(error) => {
                        if let Some(allocated) = buffer {
                            reorder.release_receive_buffer(allocated);
                        }
                        return Err(error).context("UDP receive failed");
                    }
                };
            let datagram = reorder.buffer(receive_into_pool);
            let header = match Header::decode(&datagram[..length]) {
                Ok(header) => header,
                Err(reason) => {
                    debug!(target: "socket_mux::rx", "discarding datagram: {reason}");
                    if let Some(index) = buffer {
                        reorder.release_receive_buffer(index);
                    }
                    continue;
                }
            };
            match *session {
                None => {
                    *session = Some(header.session);
                    info!(target: "socket_mux::rx", "session={}", header.session);
                }
                Some(current) if current != header.session => {
                    if !session_changed {
                        warn!(target: "socket_mux::rx",
                            "new session {} (current {}); ending stream",
                            header.session, current);
                        session_changed = true;
                    }
                    if let Some(index) = buffer {
                        reorder.release_receive_buffer(index);
                    }
                    continue;
                }
                Some(_) => {}
            }
            *last_datagram = Some(arrived);
            let outcome = reorder.accept(header, buffer, arrived)?;
            match outcome {
                InsertOutcome::Inserted | InsertOutcome::OutputDropped => {}
                InsertOutcome::Late | InsertOutcome::Duplicate => {}
            }
            if reorder.stream_ended() {
                return Ok(false);
            }
        }
    }
    Ok(session_changed)
}

async fn wait_deadline(deadline: Option<TokioInstant>) {
    match deadline {
        Some(deadline) => sleep_until(deadline).await,
        None => std::future::pending().await,
    }
}

fn finish_empty_outputs(reorder: &mut Reorder) -> Option<bool> {
    let mut finished_eos = None;
    while let Some(eos) = reorder.finish_empty_output() {
        if eos {
            return Some(true);
        }
        finished_eos = Some(false);
    }
    finished_eos
}

fn write_ready(tcp: &mut TcpStream, reorder: &mut Reorder) -> Result<bool> {
    loop {
        if let Some(eos) = reorder.finish_empty_output() {
            if eos {
                return Ok(true);
            }
            continue;
        }
        let Some(payload) = reorder.output_slice() else {
            return Ok(false);
        };
        match tcp.try_write(payload) {
            Ok(0) => bail!(io::Error::new(
                io::ErrorKind::WriteZero,
                "TCP output write returned zero"
            )),
            Ok(written) => {
                if reorder.complete_output(written) == Some(true) {
                    return Ok(true);
                }
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(false),
            Err(error) => return Err(error).context("TCP output write failed"),
        }
    }
}
