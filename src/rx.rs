//! RX: one blocking receive thread per lane, one reorder thread, one TCP
//! writer thread.
//!
//! ```text
//! lane k thread: recv() into its pool buffer ─┐
//!                                              ├─ arrivals (bounded, lock-free) ─> reorder (this thread)
//! lane k+1 ...                                ─┘                                   │ wheel, timeouts A/B,
//!                                                                                  │ memory/output limits
//!            free list (lock-free) <── writer thread <─ output (bounded) ─────────┘
//!                                     blocking TCP writes
//! ```
//!
//! Every lane thread owns one spare pool buffer and blocks in `recv` into it,
//! so a datagram lands in its final buffer with no user-space copy. After a
//! receive it swaps in a free buffer and hands the full one on; with no free
//! buffer (all W held: out of order, queued or awaiting the TCP write) it keeps
//! its buffer and hands on only the header, which the reorder records as an
//! output-limit loss of the newest datagram. The writer does blocking TCP
//! writes and returns buffers itself, so a stalled TCP consumer never blocks
//! receiving: it only fills RX memory, which the output limit then drops.
//! Waits between threads use the shared wait policy ([`crate::wait`]); the
//! reorder's timeout deadline is the policy's park deadline.

use std::io::{self, IoSlice, Write};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, Shutdown, SocketAddr, TcpStream, UdpSocket};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use log::{debug, info, warn};

use crate::cli::RxConfig;
use crate::header::{Header, HEADER_LEN};
use crate::queue::{Bounded, FreeList};
use crate::reorder::{Pool, Reorder};
use crate::report::{Meter, Reporter, Role};
use crate::signals;
use crate::stop::Stop;
use crate::udp::{self, BufferDirection};
use crate::wait::{self, Signal};

/// Upper bound on the lane -> reorder queue (entries are ~48 bytes). It never
/// needs more than W + N entries; past this bound lanes wait for room.
const ARRIVALS_MAX: usize = 1 << 16;
/// Upper bound on the reorder -> writer queue; further delivered datagrams
/// wait in the reorder's output list (they still count against W).
const OUTPUT_MAX: usize = 1 << 14;
/// Arrivals the reorder takes per turn before it hands output on.
const INGEST_BATCH: usize = 256;
/// Datagrams per vectored TCP write.
const WRITE_VECTOR: usize = 64;
/// Arrival with no buffer: dropped under the output limit.
const NO_BUFFER: u32 = u32::MAX;

/// A received datagram handed from a lane to the reorder.
struct Arrival {
    header: Header,
    buffer: u32,
    arrived: Instant,
}

/// A delivered datagram handed from the reorder to the writer.
#[derive(Clone, Copy)]
struct Delivery {
    buffer: u32,
    payload_len: u16,
    eos: bool,
}

/// State shared by the RX threads.
struct Shared {
    pool: Arc<Pool>,
    free: Arc<FreeList>,
    arrivals: Bounded<Arrival>,
    output: Bounded<Delivery>,
    /// Wakes the reorder: arrivals pushed, output popped, a failure.
    reorder_wake: Arc<Signal>,
    /// Datagrams the writer completed (their buffers are back on the list).
    retired: AtomicU64,
    /// Delivered bytes and datagrams, loss counters and events (reporter).
    meter: Arc<Meter>,
    /// Lanes stop receiving.
    lanes_quit: AtomicBool,
    /// A lane or the writer failed; `failure` holds the first error.
    failed: AtomicBool,
    failure: Mutex<Option<anyhow::Error>>,
}

impl Shared {
    fn fail(&self, error: anyhow::Error) {
        let mut failure = self.failure.lock().unwrap_or_else(|e| e.into_inner());
        if failure.is_none() {
            *failure = Some(error);
        }
        self.failed.store(true, Ordering::SeqCst);
        drop(failure);
        self.reorder_wake.notify();
    }

    fn take_failure(&self) -> Option<anyhow::Error> {
        self.failure
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
    }
}

/// How the reorder loop ended.
#[derive(Debug, Eq, PartialEq)]
enum End {
    /// The stream ended (EOS, rule B or a new session); output is closing.
    Stream,
    /// A stop or a failure ended it early.
    Aborted,
}

pub fn run(config: RxConfig, stop: Arc<Stop>) -> Result<()> {
    let lanes = config.global.lanes;
    let mut sockets = Vec::with_capacity(lanes);
    let mut addresses = Vec::with_capacity(lanes);
    for lane in 0..lanes {
        let address = SocketAddr::new(config.bind, config.global.base_port + lane as u16);
        let socket =
            udp::bind_blocking(address, config.global.udp_buffer, BufferDirection::Receive)
                .with_context(|| format!("could not bind RX UDP socket at {address}"))?;
        sockets.push(socket);
        addresses.push(address);
    }
    let tcp = TcpStream::connect(config.connect)
        .with_context(|| format!("could not connect TCP output to {}", config.connect))?;
    let meter = Meter::new();
    let mut reorder = Reorder::with_spares(
        config.global.datagram_size,
        config.global.hold_capacity,
        lanes,
        meter.clone(),
    )
    .context("could not allocate RX memory pool")?;
    let reporter = Reporter::spawn(Role::Rx, meter, config.global.interval)?;
    {
        let reporter = reporter.clone();
        signals::set_finisher(move || reporter.finish());
    }
    let result = run_threads(
        &mut reorder,
        sockets,
        addresses,
        tcp,
        config.global.timeout,
        &stop,
    );
    // Drain the reporter whatever the outcome: the summary is always printed.
    reporter.finish();
    result
}

/// Start the lane and writer threads, run the reorder on this thread, and
/// join the writer. Exposed to tests through [`run`].
fn run_threads(
    reorder: &mut Reorder,
    sockets: Vec<UdpSocket>,
    addresses: Vec<SocketAddr>,
    tcp: TcpStream,
    timeout: Duration,
    stop: &Arc<Stop>,
) -> Result<()> {
    let lanes = sockets.len();
    let capacity = reorder.capacity();
    let reorder_wake = Arc::new(Signal::new());
    let shared = Arc::new(Shared {
        pool: reorder.pool().clone(),
        free: reorder.free_list().clone(),
        arrivals: Bounded::with_signals(
            (capacity + lanes).min(ARRIVALS_MAX),
            reorder_wake.clone(),
            Arc::new(Signal::new()),
        ),
        output: Bounded::with_signals(
            capacity.min(OUTPUT_MAX),
            Arc::new(Signal::new()),
            reorder_wake.clone(),
        ),
        reorder_wake,
        retired: AtomicU64::new(0),
        meter: reorder.meter().clone(),
        lanes_quit: AtomicBool::new(false),
        failed: AtomicBool::new(false),
        failure: Mutex::new(None),
    });
    stop.watch(shared.reorder_wake.clone());
    stop.watch(shared.output.not_empty().clone());
    stop.watch(shared.arrivals.not_full().clone());
    {
        // A stop unblocks the writer's blocking write and the lanes' recv.
        let tcp = tcp.try_clone().context("could not clone the TCP output")?;
        let shared = shared.clone();
        let addresses = addresses.clone();
        stop.on_stop(move || {
            let _ = tcp.shutdown(Shutdown::Both);
            quit_lanes(&shared, &addresses);
        });
    }

    for (lane, socket) in sockets.into_iter().enumerate() {
        let own = (capacity + lane) as u32; // this lane's spare buffer
        let (shared, stop) = (shared.clone(), stop.clone());
        thread::Builder::new()
            .name(format!("rx-lane-{lane}"))
            .spawn(move || lane_thread(lane, socket, own, &shared, &stop))
            .context("could not start an RX lane thread")?;
        // Lane threads are not joined: they leave recv when poked (quit_lanes)
        // and otherwise end with the process.
    }
    let writer: JoinHandle<Result<()>> = {
        let (shared, stop) = (shared.clone(), stop.clone());
        thread::Builder::new()
            .name("rx-writer".into())
            .spawn(move || writer_thread(tcp, &shared, &stop))
            .context("could not start the RX writer thread")?
    };

    let ended = reorder_loop(reorder, &shared, timeout, stop);
    if !matches!(ended, Ok(End::Stream)) {
        // Fatal error or stop: abandon the output and unblock the writer.
        stop.set();
    }
    quit_lanes(&shared, &addresses);
    let written = writer
        .join()
        .map_err(|_| anyhow!("the RX writer thread panicked"))
        .and_then(|r| r);
    ended?;
    if let Some(error) = shared.take_failure() {
        return Err(error);
    }
    written
}

/// Tell the lanes to stop and poke each socket with an empty datagram so a
/// blocking `recv` returns. Best effort: a lane that misses it ends with the
/// process.
fn quit_lanes(shared: &Shared, addresses: &[SocketAddr]) {
    if shared.lanes_quit.swap(true, Ordering::SeqCst) {
        return;
    }
    shared.arrivals.close();
    for address in addresses {
        let target = match address.ip() {
            IpAddr::V4(ip) if ip.is_unspecified() => {
                SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), address.port())
            }
            IpAddr::V6(ip) if ip.is_unspecified() => {
                SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), address.port())
            }
            _ => *address,
        };
        let local: SocketAddr = if target.is_ipv4() {
            (Ipv4Addr::UNSPECIFIED, 0).into()
        } else {
            (Ipv6Addr::UNSPECIFIED, 0).into()
        };
        if let Ok(poke) = UdpSocket::bind(local) {
            let _ = poke.send_to(&[], target);
        }
    }
}

/// Whether `error` is a receive into a too-small buffer (Windows reports an
/// oversize datagram as WSAEMSGSIZE; elsewhere it is silently truncated).
fn is_oversize(error: &io::Error) -> bool {
    cfg!(windows) && error.raw_os_error() == Some(10040)
}

/// Lane thread: blocking `recv` straight into the owned pool buffer, decode
/// the header, swap the buffer for a free one and hand the datagram on.
fn lane_thread(lane: usize, socket: UdpSocket, mut own: u32, shared: &Shared, stop: &Stop) {
    loop {
        // SAFETY: this lane owns `own` until it hands it to the reorder.
        let buffer = unsafe { shared.pool.slice_mut(own as usize) };
        let received = socket.recv(buffer);
        if shared.lanes_quit.load(Ordering::Relaxed) || stop.is_set() {
            return;
        }
        let length = match received {
            Ok(length) => length,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) if is_oversize(&error) => {
                debug!(target: "socket_mux::rx", "discarding datagram larger than the datagram size");
                continue;
            }
            Err(error) => {
                shared.fail(anyhow!(error).context(format!("UDP receive failed on lane {lane}")));
                return;
            }
        };
        let arrived = Instant::now();
        let header = match Header::decode(&buffer[..length]) {
            Ok(header) => header,
            Err(reason) => {
                debug!(target: "socket_mux::rx", "discarding datagram: {reason}");
                continue;
            }
        };
        let handed = match shared.free.take() {
            Some(next) => std::mem::replace(&mut own, next),
            // Output limit: no RX memory is free, so drop this (the newest)
            // datagram; the reorder still learns its header.
            None => NO_BUFFER,
        };
        let arrival = Arrival {
            header,
            buffer: handed,
            arrived,
        };
        if shared.arrivals.push(arrival, stop.flag()).is_err() {
            return;
        }
    }
}

/// Writer thread: blocking, vectored TCP writes of delivered datagrams in
/// order; returns each buffer to the free list once written. After EOS, or
/// once the output queue is closed and drained, it shuts the TCP output.
fn writer_thread(mut tcp: TcpStream, shared: &Shared, stop: &Stop) -> Result<()> {
    let result = write_loop(&mut tcp, shared, stop);
    if let Err(error) = &result {
        if !stop.is_set() {
            shared.fail(anyhow!("{error:#}"));
        }
    }
    if stop.is_set() {
        return Ok(());
    }
    result?;
    tcp.shutdown(Shutdown::Write)
        .context("TCP output shutdown failed")
}

fn write_loop(tcp: &mut TcpStream, shared: &Shared, stop: &Stop) -> Result<()> {
    let mut batch = [Delivery {
        buffer: 0,
        payload_len: 0,
        eos: false,
    }; WRITE_VECTOR];
    loop {
        let Some(first) = shared.output.pop(stop.flag()) else {
            return Ok(()); // closed and drained, or stopped
        };
        batch[0] = first;
        let mut count = 1;
        while count < WRITE_VECTOR {
            match shared.output.try_pop() {
                Some(delivery) => {
                    batch[count] = delivery;
                    count += 1;
                }
                None => break,
            }
        }
        let eos = write_batch(tcp, &batch[..count], shared, stop)?;
        if eos || stop.is_set() {
            return Ok(());
        }
    }
}

/// Write `batch` completely; returns whether it held the EOS datagram.
fn write_batch(
    tcp: &mut TcpStream,
    batch: &[Delivery],
    shared: &Shared,
    stop: &Stop,
) -> Result<bool> {
    let mut done = 0; // datagrams of `batch` completed
    let mut partial = 0; // bytes of batch[done] already written
    let mut eos = false;
    let complete = |delivery: &Delivery, eos: &mut bool| {
        shared.free.give(delivery.buffer);
        shared.meter.add_data(1, u64::from(delivery.payload_len));
        shared.retired.fetch_add(1, Ordering::Release);
        *eos |= delivery.eos;
    };
    while done < batch.len() {
        let mut slices = [IoSlice::new(&[]); WRITE_VECTOR];
        let mut total = 0;
        for (i, delivery) in batch[done..].iter().enumerate() {
            // SAFETY: the writer owns delivered buffers until it gives them back.
            let buffer = unsafe { shared.pool.slice(delivery.buffer as usize) };
            let skip = if i == 0 { partial } else { 0 };
            let payload =
                &buffer[HEADER_LEN + skip..HEADER_LEN + usize::from(delivery.payload_len)];
            total += payload.len();
            slices[i] = IoSlice::new(payload);
        }
        let mut written = if total == 0 {
            0
        } else {
            match tcp.write_vectored(&slices[..batch.len() - done]) {
                Ok(0) => {
                    return Err(anyhow!(io::Error::new(
                        io::ErrorKind::WriteZero,
                        "TCP output write returned zero"
                    )))
                }
                Ok(written) => written,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => {
                    if stop.is_set() {
                        return Ok(false);
                    }
                    return Err(error).context("TCP output write failed");
                }
            }
        };
        // Retire every datagram the write covered, including empty ones.
        while done < batch.len() {
            let left = usize::from(batch[done].payload_len) - partial;
            if written < left {
                partial += written;
                break;
            }
            written -= left;
            partial = 0;
            complete(&batch[done], &mut eos);
            done += 1;
        }
    }
    Ok(eos)
}

/// The reorder thread: owns the wheel and every loss decision.
fn reorder_loop(
    reorder: &mut Reorder,
    shared: &Shared,
    timeout: Duration,
    stop: &Stop,
) -> Result<End> {
    let mut session: Option<u16> = None;
    let mut last_datagram: Option<Instant> = None;
    let mut end_after_flush = false;
    let mut retired_seen = 0_u64;
    loop {
        if stop.is_set() || shared.failed.load(Ordering::Relaxed) {
            return Ok(End::Aborted);
        }
        // 1. Forget datagrams the writer completed first, so a repeat of a
        //    written datagram is judged late rather than a duplicate.
        let retired = shared.retired.load(Ordering::Acquire);
        if retired != retired_seen {
            reorder.retire((retired - retired_seen) as usize);
            retired_seen = retired;
        }

        // 2. Arrivals, at most INGEST_BATCH per turn so output keeps moving.
        let mut ingested = 0;
        let mut drained = false;
        while ingested < INGEST_BATCH {
            let Some(arrival) = shared.arrivals.try_pop() else {
                drained = true;
                break;
            };
            ingested += 1;
            let buffer = (arrival.buffer != NO_BUFFER).then_some(arrival.buffer as usize);
            if end_after_flush || reorder.stream_ended() {
                if let Some(buffer) = buffer {
                    reorder.release_receive_buffer(buffer);
                }
                continue;
            }
            match session {
                None => {
                    session = Some(arrival.header.session);
                    shared.meter.start(arrival.arrived);
                    info!(target: "socket_mux::rx", "session={}", arrival.header.session);
                }
                Some(current) if current != arrival.header.session => {
                    warn!(target: "socket_mux::rx",
                        "new session {} (current {current}); ending stream",
                        arrival.header.session);
                    if let Some(buffer) = buffer {
                        reorder.release_receive_buffer(buffer);
                    }
                    reorder.flush();
                    end_after_flush = true;
                    continue;
                }
                Some(_) => {}
            }
            last_datagram = Some(arrival.arrived);
            reorder.accept(arrival.header, buffer, arrival.arrived)?;
        }

        // 3. Hand delivered datagrams to the writer, as far as its queue allows.
        let mut handed = 0;
        while let Some(output) = reorder.unhanded() {
            let delivery = Delivery {
                buffer: output.buffer as u32,
                payload_len: output.payload_len as u16,
                eos: output.eos,
            };
            if shared.output.try_push(delivery).is_err() {
                break;
            }
            reorder.mark_handed();
            handed += 1;
        }

        // 4. The stream ended and every delivered datagram is with the writer.
        if (end_after_flush || reorder.stream_ended()) && reorder.all_handed() {
            shared.output.close();
            return Ok(End::Stream);
        }

        // 5. Timeouts, judged only once every queued arrival has been seen.
        let deadline = if !end_after_flush && !reorder.stream_ended() {
            reorder.next_deadline(timeout, last_datagram)
        } else {
            None
        };
        if drained {
            if let Some(deadline) = deadline {
                let now = Instant::now();
                if now >= deadline {
                    if last_datagram
                        .is_some_and(|last| now.saturating_duration_since(last) >= timeout)
                    {
                        // Rule B: no datagram for T; deliver what is held, end.
                        reorder.flush();
                        info!(target: "socket_mux::rx",
                            "timeout {timeout:?} expired with no datagrams; ending the stream: total_lost_slices={} total_lost_bytes={}",
                            reorder.total_lost_slices(), reorder.total_lost_bytes());
                        end_after_flush = true;
                    } else {
                        // Rule A: the gap behind next expired.
                        reorder.expire_gap(now, timeout);
                    }
                    continue;
                }
            }
        }
        if ingested > 0 || handed > 0 {
            continue;
        }

        // 6. Nothing to do: wait for an arrival, writer room, a stop or the deadline.
        let pending = reorder.unhanded().is_some();
        wait::wait_until(&shared.reorder_wake, deadline, || {
            !shared.arrivals.is_empty()
                || pending && !shared.output.is_full()
                || stop.is_set()
                || shared.failed.load(Ordering::Relaxed)
        });
    }
}
