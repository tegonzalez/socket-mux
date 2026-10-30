//! Loss and throughput reporting off the critical path.
//!
//! The hot threads only touch a [`Meter`]: relaxed atomic counters plus a
//! bounded lock-free event ring. Pushing never blocks; when the ring is full
//! the event is counted in `events_dropped` instead (totals stay exact because
//! they are counters, not events). A low-priority reporter thread drains the
//! ring, merges adjacent loss gaps of the same cause into stretches, and logs
//! iperf3-like lines:
//!
//! ```text
//! [ 3.00- 4.00 s]  1.17 GBytes  9.38 Gbit/s  lost 15 datagrams in 2 stretches (memory 0 output 15 timeout 0) longest 12
//! rx summary: [ 0.00-12.00 s]  13.9 GBytes  9.29 Gbit/s  delivered 13929000000 bytes in 9754201 datagrams  lost 15 datagrams 21420 bytes in 2 stretches (memory 0 output 15 timeout 0) longest 12  late 0 duplicate 0
//! ```
//!
//! Per-stretch detail (`loss gap: cause=.. first_seq=.. lost_slices=..
//! lost_bytes=..`) and late/duplicate drops are logged at debug only.

use std::fmt::Write as _;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use crossbeam_queue::ArrayQueue;
use log::{debug, info, log_enabled, Level};

use crate::header::SEQUENCE_MASK;
use crate::wait::{wait_until, Signal};

/// Events the ring holds; ~32 bytes each.
pub const EVENT_RING: usize = 4096;

/// Why RX declared slices lost (spec Semantics: timeout rules A/B, the RX
/// memory limit, the output limit).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Cause {
    Memory = 0,
    Output = 1,
    Timeout = 2,
}

impl Cause {
    pub fn name(self) -> &'static str {
        match self {
            Cause::Memory => "memory",
            Cause::Output => "output",
            Cause::Timeout => "timeout",
        }
    }
}

/// A run of consecutive lost slices with one cause.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Stretch {
    pub cause: Cause,
    pub first: u64,
    pub slices: u64,
    pub bytes: u64,
}

impl Stretch {
    /// Absorb `next` if it has the same cause and starts right after this
    /// stretch (serial arithmetic on the sequence width).
    pub fn try_merge(&mut self, next: &Stretch) -> bool {
        if next.cause != self.cause
            || next.first != self.first.wrapping_add(self.slices) & SEQUENCE_MASK
        {
            return false;
        }
        self.slices += next.slices;
        self.bytes = self.bytes.wrapping_add(next.bytes);
        true
    }
}

/// What the hot threads tell the reporter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Event {
    Loss(Stretch),
    /// Late datagram dropped (pushed only when debug logging is on).
    Late(u64),
    /// Duplicate datagram dropped (pushed only when debug logging is on).
    Duplicate(u64),
}

/// Counters and the event ring shared by the hot threads and the reporter.
pub struct Meter {
    start: OnceLock<Instant>,
    /// Nanoseconds from start to the last delivered (RX) or sent (TX) data.
    last_nanos: AtomicU64,
    /// Payload bytes delivered to TCP (RX) or sent (TX).
    pub bytes: AtomicU64,
    /// Datagrams delivered (RX) or sent (TX).
    pub datagrams: AtomicU64,
    lost: [AtomicU64; 3],
    lost_bytes: AtomicU64,
    late: AtomicU64,
    duplicate: AtomicU64,
    events_dropped: AtomicU64,
    ring: ArrayQueue<Event>,
    wake: Signal,
}

impl std::fmt::Debug for Meter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Meter")
            .field("totals", &self.snapshot())
            .finish()
    }
}

impl Default for Meter {
    fn default() -> Self {
        Self::with_ring(EVENT_RING)
    }
}

impl Meter {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn with_ring(capacity: usize) -> Self {
        Self {
            start: OnceLock::new(),
            last_nanos: AtomicU64::new(0),
            bytes: AtomicU64::new(0),
            datagrams: AtomicU64::new(0),
            lost: Default::default(),
            lost_bytes: AtomicU64::new(0),
            late: AtomicU64::new(0),
            duplicate: AtomicU64::new(0),
            events_dropped: AtomicU64::new(0),
            ring: ArrayQueue::new(capacity.max(2)),
            wake: Signal::new(),
        }
    }

    /// The stream started at `at` (first valid datagram, or first send); later
    /// calls are ignored. Interval lines count from here.
    pub fn start(&self, at: Instant) {
        if self.start.set(at).is_ok() {
            self.wake.notify();
        }
    }

    pub fn started(&self) -> Option<Instant> {
        self.start.get().copied()
    }

    /// Count `datagrams` datagrams of `bytes` payload bytes delivered or sent.
    pub fn add_data(&self, datagrams: u64, bytes: u64) {
        self.datagrams.fetch_add(datagrams, Ordering::Relaxed);
        self.bytes.fetch_add(bytes, Ordering::Relaxed);
        if let Some(start) = self.started() {
            let nanos = start.elapsed().as_nanos().min(u128::from(u64::MAX)) as u64;
            self.last_nanos.fetch_max(nanos, Ordering::Relaxed);
        }
    }

    /// Record a loss gap: exact counters plus a stretch event for the detail.
    pub fn loss(&self, cause: Cause, first: u64, slices: u64, bytes: u64) {
        if slices == 0 {
            return;
        }
        self.lost[cause as usize].fetch_add(slices, Ordering::Relaxed);
        self.lost_bytes.fetch_add(bytes, Ordering::Relaxed);
        self.push(Event::Loss(Stretch {
            cause,
            first,
            slices,
            bytes,
        }));
    }

    pub fn late(&self, sequence: u64) {
        self.late.fetch_add(1, Ordering::Relaxed);
        if log_enabled!(target: "socket_mux::rx", Level::Debug) {
            self.push(Event::Late(sequence));
        }
    }

    pub fn duplicate(&self, sequence: u64) {
        self.duplicate.fetch_add(1, Ordering::Relaxed);
        if log_enabled!(target: "socket_mux::rx", Level::Debug) {
            self.push(Event::Duplicate(sequence));
        }
    }

    /// Never blocks: a full ring counts the event as dropped. Wakes the
    /// reporter early once the ring is half full.
    fn push(&self, event: Event) {
        if self.ring.push(event).is_err() {
            self.events_dropped.fetch_add(1, Ordering::Relaxed);
            return;
        }
        if self.ring.len() >= self.ring.capacity() / 2 {
            self.wake.notify();
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            bytes: self.bytes.load(Ordering::Relaxed),
            datagrams: self.datagrams.load(Ordering::Relaxed),
            lost: [0, 1, 2].map(|i| self.lost[i].load(Ordering::Relaxed)),
            lost_bytes: self.lost_bytes.load(Ordering::Relaxed),
            late: self.late.load(Ordering::Relaxed),
            duplicate: self.duplicate.load(Ordering::Relaxed),
            events_dropped: self.events_dropped.load(Ordering::Relaxed),
        }
    }
}

/// Counter values at one moment.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Snapshot {
    pub bytes: u64,
    pub datagrams: u64,
    pub lost: [u64; 3],
    pub lost_bytes: u64,
    pub late: u64,
    pub duplicate: u64,
    pub events_dropped: u64,
}

impl Snapshot {
    fn since(&self, earlier: &Snapshot) -> Snapshot {
        Snapshot {
            bytes: self.bytes - earlier.bytes,
            datagrams: self.datagrams - earlier.datagrams,
            lost: [0, 1, 2].map(|i| self.lost[i] - earlier.lost[i]),
            lost_bytes: self.lost_bytes - earlier.lost_bytes,
            late: self.late - earlier.late,
            duplicate: self.duplicate - earlier.duplicate,
            events_dropped: self.events_dropped - earlier.events_dropped,
        }
    }

    pub fn lost_total(&self) -> u64 {
        self.lost.iter().sum()
    }
}

/// Stretch counts for a span of time.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StretchStats {
    pub count: u64,
    pub longest: u64,
}

impl StretchStats {
    fn add(&mut self, stretch: &Stretch) {
        self.count += 1;
        self.longest = self.longest.max(stretch.slices);
    }
}

/// Merges loss events into stretches and counts them per interval and in
/// total. Adjacent gaps of one cause become one stretch.
#[derive(Debug, Default)]
pub struct Merger {
    open: Option<Stretch>,
    pub interval: StretchStats,
    pub total: StretchStats,
}

impl Merger {
    /// Feed one gap; returns the stretch it closed, if any.
    pub fn add(&mut self, stretch: Stretch) -> Option<Stretch> {
        if let Some(open) = &mut self.open {
            if open.try_merge(&stretch) {
                return None;
            }
        }
        let closed = self.close();
        self.open = Some(stretch);
        closed
    }

    /// Close the open stretch (end of interval or of stream).
    pub fn close(&mut self) -> Option<Stretch> {
        let closed = self.open.take()?;
        self.interval.add(&closed);
        self.total.add(&closed);
        debug!(target: "socket_mux::rx",
            "loss gap: cause={} first_seq={} lost_slices={} lost_bytes={}",
            closed.cause.name(), closed.first, closed.slices, closed.bytes);
        Some(closed)
    }

    /// Take and reset the interval statistics.
    pub fn take_interval(&mut self) -> StretchStats {
        std::mem::take(&mut self.interval)
    }
}

/// "1.17 GBytes": decimal units, three significant digits (as iperf3 does).
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["KBytes", "MBytes", "GBytes", "TBytes", "PBytes"];
    if bytes < 1000 {
        return format!("{bytes} Bytes");
    }
    let mut value = bytes as f64 / 1000.0;
    let mut unit = 0;
    while value >= 999.5 && unit + 1 < UNITS.len() {
        value /= 1000.0;
        unit += 1;
    }
    format!("{} {}", three_digits(value), UNITS[unit])
}

/// "9.38 Gbit/s": decimal units, three significant digits.
pub fn format_rate(bytes: u64, span: Duration) -> String {
    const UNITS: [&str; 5] = ["bit/s", "Kbit/s", "Mbit/s", "Gbit/s", "Tbit/s"];
    let seconds = span.as_secs_f64();
    let mut value = if seconds > 0.0 {
        bytes as f64 * 8.0 / seconds
    } else {
        0.0
    };
    let mut unit = 0;
    while value >= 999.5 && unit + 1 < UNITS.len() {
        value /= 1000.0;
        unit += 1;
    }
    format!("{} {}", three_digits(value), UNITS[unit])
}

fn three_digits(value: f64) -> String {
    if value < 9.995 {
        format!("{value:.2}")
    } else if value < 99.95 {
        format!("{value:.1}")
    } else {
        format!("{value:.0}")
    }
}

fn span_label(from: Duration, to: Duration) -> String {
    format!("[{:5.2}-{:5.2} s]", from.as_secs_f64(), to.as_secs_f64())
}

fn losses(delta: &Snapshot, stretches: &StretchStats) -> String {
    format!(
        "lost {} datagrams in {} stretches (memory {} output {} timeout {}) longest {}",
        delta.lost_total(),
        stretches.count,
        delta.lost[0],
        delta.lost[1],
        delta.lost[2],
        stretches.longest
    )
}

/// Which side a reporter speaks for.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Role {
    Rx,
    Tx,
}

/// One interval line.
pub fn interval_line(
    role: Role,
    from: Duration,
    to: Duration,
    delta: &Snapshot,
    stretches: &StretchStats,
) -> String {
    let mut line = format!(
        "{}  {}  {}",
        span_label(from, to),
        format_bytes(delta.bytes),
        format_rate(delta.bytes, to.saturating_sub(from)),
    );
    match role {
        Role::Rx => write!(line, "  {}", losses(delta, stretches)),
        Role::Tx => write!(line, "  sent {} datagrams", delta.datagrams),
    }
    .expect("writing to a String");
    line
}

/// The final summary line; `span` runs from the stream start to the last
/// delivered (RX) or sent (TX) data.
pub fn summary_line(
    role: Role,
    span: Duration,
    total: &Snapshot,
    stretches: &StretchStats,
) -> String {
    let head = format!(
        "{}  {}  {}",
        span_label(Duration::ZERO, span),
        format_bytes(total.bytes),
        format_rate(total.bytes, span),
    );
    match role {
        Role::Rx => {
            let mut line = format!(
                "rx summary: {head}  delivered {} bytes in {} datagrams  lost {} datagrams {} bytes in {} stretches (memory {} output {} timeout {}) longest {}  late {} duplicate {}",
                total.bytes,
                total.datagrams,
                total.lost_total(),
                total.lost_bytes,
                stretches.count,
                total.lost[0],
                total.lost[1],
                total.lost[2],
                stretches.longest,
                total.late,
                total.duplicate,
            );
            if total.events_dropped > 0 {
                write!(line, "  events_dropped {}", total.events_dropped)
                    .expect("writing to a String");
            }
            line
        }
        Role::Tx => format!(
            "tx summary: {head}  sent {} bytes in {} datagrams",
            total.bytes, total.datagrams
        ),
    }
}

/// Lower the calling thread's scheduling priority: nice +10 for this thread
/// on Linux, below-normal priority on Windows, nothing elsewhere.
pub fn lower_thread_priority() {
    #[cfg(target_os = "linux")]
    // SAFETY: plain syscalls on the calling thread's own id.
    unsafe {
        let tid = libc::syscall(libc::SYS_gettid) as libc::id_t;
        libc::setpriority(libc::PRIO_PROCESS, tid, 10);
    }
    #[cfg(windows)]
    // SAFETY: GetCurrentThread returns a pseudo-handle for the calling thread.
    unsafe {
        use windows_sys::Win32::System::Threading::{
            GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_BELOW_NORMAL,
        };
        SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_BELOW_NORMAL);
    }
}

/// The reporter thread; [`Reporter::finish`] drains it and prints the summary.
pub struct Reporter {
    meter: Arc<Meter>,
    finish: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl Reporter {
    /// Start the reporter. `interval` zero disables interval lines; the
    /// summary is printed either way.
    pub fn spawn(role: Role, meter: Arc<Meter>, interval: Duration) -> Result<Self> {
        let finish = Arc::new(AtomicBool::new(false));
        let handle = {
            let (meter, finish) = (meter.clone(), finish.clone());
            thread::Builder::new()
                .name(format!(
                    "{}-report",
                    if role == Role::Rx { "rx" } else { "tx" }
                ))
                .spawn(move || report_thread(role, &meter, &finish, interval))
                .context("could not start the reporter thread")?
        };
        Ok(Self {
            meter,
            finish,
            handle: Some(handle),
        })
    }

    /// Drain every pending event, print the summary and stop the thread.
    pub fn finish(mut self) {
        self.stop();
    }

    fn stop(&mut self) {
        self.finish.store(true, Ordering::SeqCst);
        self.meter.wake.notify();
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for Reporter {
    fn drop(&mut self) {
        self.stop();
    }
}

fn target(role: Role) -> &'static str {
    match role {
        Role::Rx => "socket_mux::rx",
        Role::Tx => "socket_mux::tx",
    }
}

fn drain(meter: &Meter, merger: &mut Merger) {
    while let Some(event) = meter.ring.pop() {
        match event {
            Event::Loss(stretch) => {
                merger.add(stretch);
            }
            Event::Late(sequence) => {
                debug!(target: "socket_mux::rx", "discarding late datagram: seq={sequence}")
            }
            Event::Duplicate(sequence) => {
                debug!(target: "socket_mux::rx", "discarding duplicate datagram: seq={sequence}")
            }
        }
    }
}

fn report_thread(role: Role, meter: &Meter, finish: &AtomicBool, interval: Duration) {
    lower_thread_priority();
    let mut merger = Merger::default();
    let mut previous = Snapshot::default();
    let mut ticks: u32 = 0;
    let half = meter.ring.capacity() / 2;
    loop {
        let next_tick = match meter.started() {
            Some(start) if !interval.is_zero() => interval
                .checked_mul(ticks + 1)
                .and_then(|offset| start.checked_add(offset)),
            _ => None,
        };
        let was_started = meter.started().is_some();
        wait_until(&meter.wake, next_tick, || {
            finish.load(Ordering::Relaxed)
                || meter.ring.len() >= half
                || meter.started().is_some() != was_started
        });
        drain(meter, &mut merger);
        if finish.load(Ordering::Relaxed) {
            break;
        }
        if let Some(tick) = next_tick {
            if Instant::now() >= tick {
                merger.close();
                let now = meter.snapshot();
                let from = interval * ticks;
                ticks += 1;
                info!(target: target(role), "{}",
                    interval_line(role, from, interval * ticks, &now.since(&previous), &merger.take_interval()));
                previous = now;
            }
        }
    }
    merger.close();
    let total = meter.snapshot();
    let span = Duration::from_nanos(meter.last_nanos.load(Ordering::Relaxed));
    info!(target: target(role), "{}", summary_line(role, span, &total, &merger.total));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stretch(cause: Cause, first: u64, slices: u64) -> Stretch {
        Stretch {
            cause,
            first,
            slices,
            bytes: slices * 10,
        }
    }

    #[test]
    fn adjacent_gaps_of_one_cause_merge_into_one_stretch() {
        let mut merger = Merger::default();
        assert_eq!(merger.add(stretch(Cause::Output, 10, 2)), None);
        assert_eq!(
            merger.add(stretch(Cause::Output, 12, 3)),
            None,
            "adjacent: merged"
        );
        // Same cause but not adjacent: closes the first stretch.
        assert_eq!(
            merger.add(stretch(Cause::Output, 20, 1)),
            Some(Stretch {
                cause: Cause::Output,
                first: 10,
                slices: 5,
                bytes: 50
            })
        );
        // Adjacent but a different cause: a new stretch.
        assert_eq!(
            merger.add(stretch(Cause::Timeout, 21, 4)),
            Some(stretch(Cause::Output, 20, 1))
        );
        assert_eq!(merger.close(), Some(stretch(Cause::Timeout, 21, 4)));
        assert_eq!(merger.close(), None);
        assert_eq!(
            merger.total,
            StretchStats {
                count: 3,
                longest: 5
            }
        );
        assert_eq!(merger.take_interval().count, 3);
        assert_eq!(merger.interval, StretchStats::default());
    }

    #[test]
    fn stretches_merge_across_sequence_rollover() {
        let mut open = stretch(Cause::Memory, SEQUENCE_MASK - 1, 2);
        assert!(open.try_merge(&stretch(Cause::Memory, 0, 3)));
        assert_eq!(open.slices, 5);
    }

    #[test]
    fn units_use_three_significant_digits() {
        assert_eq!(format_bytes(0), "0 Bytes");
        assert_eq!(format_bytes(999), "999 Bytes");
        assert_eq!(format_bytes(1_000), "1.00 KBytes");
        assert_eq!(format_bytes(1_172_500_000), "1.17 GBytes");
        assert_eq!(format_bytes(13_929_000_000), "13.9 GBytes");
        assert_eq!(format_bytes(999_999), "1.00 MBytes");
        assert_eq!(
            format_rate(1_172_500_000, Duration::from_secs(1)),
            "9.38 Gbit/s"
        );
        assert_eq!(format_rate(125, Duration::from_secs(1)), "1.00 Kbit/s");
        assert_eq!(format_rate(5, Duration::ZERO), "0.00 bit/s");
    }

    #[test]
    fn interval_and_summary_lines_have_the_iperf_like_shape() {
        let delta = Snapshot {
            bytes: 1_172_500_000,
            datagrams: 821_078,
            lost: [0, 15, 0],
            lost_bytes: 21_420,
            ..Snapshot::default()
        };
        let stretches = StretchStats {
            count: 2,
            longest: 12,
        };
        assert_eq!(
            interval_line(
                Role::Rx,
                Duration::from_secs(3),
                Duration::from_secs(4),
                &delta,
                &stretches
            ),
            "[ 3.00- 4.00 s]  1.17 GBytes  9.38 Gbit/s  lost 15 datagrams in 2 stretches (memory 0 output 15 timeout 0) longest 12"
        );
        assert_eq!(
            summary_line(Role::Rx, Duration::from_secs(1), &delta, &stretches),
            "rx summary: [ 0.00- 1.00 s]  1.17 GBytes  9.38 Gbit/s  delivered 1172500000 bytes in 821078 datagrams  lost 15 datagrams 21420 bytes in 2 stretches (memory 0 output 15 timeout 0) longest 12  late 0 duplicate 0"
        );
        let dropped = Snapshot {
            events_dropped: 7,
            ..delta
        };
        assert!(
            summary_line(Role::Rx, Duration::from_secs(1), &dropped, &stretches)
                .ends_with("  events_dropped 7")
        );
        assert_eq!(
            interval_line(
                Role::Tx,
                Duration::ZERO,
                Duration::from_secs(1),
                &delta,
                &stretches
            ),
            "[ 0.00- 1.00 s]  1.17 GBytes  9.38 Gbit/s  sent 821078 datagrams"
        );
        assert_eq!(
            summary_line(Role::Tx, Duration::from_millis(12_500), &delta, &stretches),
            "tx summary: [ 0.00-12.50 s]  1.17 GBytes  750 Mbit/s  sent 1172500000 bytes in 821078 datagrams"
        );
    }

    #[test]
    fn a_full_ring_counts_dropped_events_and_keeps_totals_exact() {
        let meter = Meter::with_ring(8);
        for i in 0..20 {
            meter.loss(Cause::Output, i * 2, 1, 10);
        }
        let snapshot = meter.snapshot();
        assert_eq!(snapshot.lost, [0, 20, 0]);
        assert_eq!(snapshot.lost_bytes, 200);
        assert_eq!(snapshot.events_dropped, 12);
        assert_eq!(meter.ring.len(), 8);
        meter.loss(Cause::Memory, 0, 0, 0);
        assert_eq!(
            meter.snapshot().events_dropped,
            12,
            "an empty gap is no event"
        );
    }

    #[test]
    fn reporter_drains_and_finishes_promptly() {
        let meter = Meter::new();
        let reporter = Reporter::spawn(Role::Rx, meter.clone(), Duration::from_millis(20)).unwrap();
        meter.start(Instant::now());
        meter.add_data(3, 300);
        meter.loss(Cause::Timeout, 5, 2, 20);
        std::thread::sleep(Duration::from_millis(70));
        let begin = Instant::now();
        reporter.finish();
        assert!(begin.elapsed() < Duration::from_secs(1));
        assert!(meter.ring.is_empty(), "every event drained");
    }
}
