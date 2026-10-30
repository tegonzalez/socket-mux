//! Shared wait policy for every hot-path wait that is not a blocking syscall.
//!
//! Socket waits stay blocking `recv`/`send`/`write` calls; nothing here polls a
//! socket. Waits *between threads* (a queue empty or full, a pacing due time)
//! follow one ladder, reset whenever work turns up:
//!
//! 1. bounded busy-wait: `spin_loop` hints, doubling 1, 2, .. 64 (about 127
//!    hints, a few microseconds);
//! 2. `yield_now`, [`YIELD_STEPS`] times;
//! 3. short sleeps doubling from [`SLEEP_START`] to [`SLEEP_CAP`] (~1 ms); they
//!    are timed parks, so a producer's [`Signal::notify`] cuts them short;
//! 4. park until notified or until the caller's deadline.
//!
//! An idle thread therefore reaches a plain park within about 2 ms and then
//! uses no CPU until it is notified.

use std::sync::atomic::{fence, AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::thread::{self, Thread};
use std::time::{Duration, Instant};

/// Spin steps; step k issues 2^k spin hints.
pub const SPIN_STEPS: u32 = 7;
/// Yield steps after spinning.
pub const YIELD_STEPS: u32 = 3;
/// First timed park after yielding; each further one doubles.
pub const SLEEP_START: Duration = Duration::from_micros(16);
/// Longest timed park; past it the waiter parks until notified.
pub const SLEEP_CAP: Duration = Duration::from_micros(1024);

/// What the next wait step does.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Phase {
    /// Issue this many `spin_loop` hints.
    Spin(u32),
    /// Yield the processor once.
    Yield,
    /// Park for at most this long (cut short by a notify).
    Sleep(Duration),
    /// Park until notified (or until the caller's deadline).
    Park,
}

/// Exponential backoff state; [`Backoff::reset`] on work restarts at spinning.
#[derive(Clone, Debug, Default)]
pub struct Backoff {
    step: u32,
}

impl Backoff {
    pub const fn new() -> Self {
        Self { step: 0 }
    }

    /// Restart the ladder at the cheapest spin (call when work turned up).
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn reset(&mut self) {
        self.step = 0;
    }

    pub fn phase(&self) -> Phase {
        if self.step < SPIN_STEPS {
            return Phase::Spin(1 << self.step);
        }
        let step = self.step - SPIN_STEPS;
        if step < YIELD_STEPS {
            return Phase::Yield;
        }
        let doublings = step - YIELD_STEPS;
        match SLEEP_START.checked_mul(1 << doublings.min(20)) {
            Some(sleep) if sleep <= SLEEP_CAP => Phase::Sleep(sleep),
            _ => Phase::Park,
        }
    }

    /// Move one rung up the ladder; stays at [`Phase::Park`].
    pub fn advance(&mut self) {
        if self.phase() != Phase::Park {
            self.step += 1;
        }
    }

    /// Whether the busy phases are over (the next step parks or sleeps).
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn is_parking(&self) -> bool {
        matches!(self.phase(), Phase::Sleep(_) | Phase::Park)
    }
}

/// A wake-up channel between threads (an event count). Waiters register,
/// re-check their condition and park; [`Signal::notify`] unparks registered
/// waiters. With no waiter registered, a notify costs one fence and one load,
/// so producers may call it after every push.
#[derive(Debug, Default)]
pub struct Signal {
    waiters: AtomicUsize,
    threads: Mutex<Vec<Thread>>,
}

impl Signal {
    pub fn new() -> Self {
        Self {
            waiters: AtomicUsize::new(0),
            threads: Mutex::new(Vec::with_capacity(4)),
        }
    }

    /// Wake every waiter. Call after making a waiter's condition true.
    pub fn notify(&self) {
        // Pairs with the fence in `park`: either we see the waiter registered,
        // or the waiter's re-check sees the state we published before this call.
        fence(Ordering::SeqCst);
        if self.waiters.load(Ordering::Relaxed) == 0 {
            return;
        }
        let threads = self.threads.lock().unwrap_or_else(|e| e.into_inner());
        for thread in threads.iter() {
            thread.unpark();
        }
    }

    /// Park the current thread until notified, `timeout` passes or `ready()`
    /// already holds. Spurious returns are allowed; callers loop.
    pub fn park(&self, timeout: Option<Duration>, ready: &mut impl FnMut() -> bool) {
        let me = thread::current();
        let id = me.id();
        self.threads
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(me);
        self.waiters.fetch_add(1, Ordering::SeqCst);
        fence(Ordering::SeqCst);
        if !ready() {
            match timeout {
                Some(timeout) => thread::park_timeout(timeout),
                None => thread::park(),
            }
        }
        self.waiters.fetch_sub(1, Ordering::SeqCst);
        let mut threads = self.threads.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(at) = threads.iter().position(|t| t.id() == id) {
            threads.swap_remove(at);
        }
    }
}

/// Why [`wait_until`] returned.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Woke {
    Ready,
    Deadline,
}

/// Wait with the shared policy until `ready()` holds or `deadline` passes.
/// `ready` is re-checked after every step, so it should be cheap (a queue
/// emptiness check, a flag load). Producers must [`Signal::notify`] `signal`
/// after making `ready()` true.
pub fn wait_until(
    signal: &Signal,
    deadline: Option<Instant>,
    mut ready: impl FnMut() -> bool,
) -> Woke {
    let mut backoff = Backoff::new();
    loop {
        if ready() {
            return Woke::Ready;
        }
        let left = match deadline {
            Some(deadline) => match deadline.checked_duration_since(Instant::now()) {
                Some(left) if !left.is_zero() => Some(left),
                _ => return Woke::Deadline,
            },
            None => None,
        };
        match backoff.phase() {
            Phase::Spin(hints) => (0..hints).for_each(|_| std::hint::spin_loop()),
            Phase::Yield => thread::yield_now(),
            Phase::Sleep(sleep) => {
                signal.park(Some(left.map_or(sleep, |left| left.min(sleep))), &mut ready)
            }
            Phase::Park => signal.park(left, &mut ready),
        }
        backoff.advance();
    }
}

/// Spin only this close to a known due time.
pub const DUE_SPIN: Duration = Duration::from_micros(5);
/// Below this much time left a timed sleep is too coarse (OS timer slack and
/// wake-up latency), so the waiter yields instead.
#[cfg(target_os = "linux")]
pub const DUE_SLEEP_OVER: Duration = Duration::from_micros(60);
#[cfg(all(unix, not(target_os = "linux")))]
pub const DUE_SLEEP_OVER: Duration = Duration::from_micros(200);
#[cfg(not(unix))]
pub const DUE_SLEEP_OVER: Duration = Duration::from_millis(2);

/// Wait for a known due time (TX pacing): sleep in pieces of at most
/// [`SLEEP_CAP`] while more than [`DUE_SLEEP_OVER`] is left, then yield, then
/// spin for the last [`DUE_SPIN`]. Returns false early when `stop` is set,
/// which is checked at least every [`SLEEP_CAP`].
pub fn wait_due(due: Instant, stop: &AtomicBool) -> bool {
    loop {
        let now = Instant::now();
        let Some(left) = due.checked_duration_since(now).filter(|d| !d.is_zero()) else {
            return true;
        };
        if left <= DUE_SPIN {
            while Instant::now() < due {
                std::hint::spin_loop();
            }
            return true;
        }
        if stop.load(Ordering::Relaxed) {
            return false;
        }
        if left > DUE_SLEEP_OVER {
            thread::sleep((left - DUE_SLEEP_OVER).min(SLEEP_CAP));
        } else {
            thread::yield_now();
        }
    }
}

/// Ask the OS for fine timer resolution on this thread, so short pacing
/// sleeps do not overshoot by the default 50 µs Linux timer slack. A no-op
/// elsewhere.
pub fn fine_timer_slack() {
    #[cfg(target_os = "linux")]
    // SAFETY: PR_SET_TIMERSLACK only changes this thread's timer slack.
    unsafe {
        libc::prctl(libc::PR_SET_TIMERSLACK, 1 as libc::c_ulong, 0, 0, 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU64;
    use std::sync::Arc;

    #[test]
    fn backoff_spins_doubling_then_yields_then_sleeps_doubling_to_the_cap_then_parks() {
        let mut backoff = Backoff::new();
        let mut phases = Vec::new();
        for _ in 0..40 {
            phases.push(backoff.phase());
            backoff.advance();
        }
        let spins: Vec<_> = phases.iter().take(SPIN_STEPS as usize).copied().collect();
        assert_eq!(
            spins,
            (0..SPIN_STEPS)
                .map(|k| Phase::Spin(1 << k))
                .collect::<Vec<_>>()
        );
        let after_spin = &phases[SPIN_STEPS as usize..];
        assert!(after_spin[..YIELD_STEPS as usize]
            .iter()
            .all(|p| *p == Phase::Yield));
        let sleeps: Vec<Duration> = after_spin[YIELD_STEPS as usize..]
            .iter()
            .filter_map(|p| match p {
                Phase::Sleep(d) => Some(*d),
                _ => None,
            })
            .collect();
        assert_eq!(sleeps.first(), Some(&SLEEP_START));
        assert!(sleeps.windows(2).all(|w| w[1] == w[0] * 2), "{sleeps:?}");
        assert!(*sleeps.last().unwrap() <= SLEEP_CAP);
        assert_eq!(*phases.last().unwrap(), Phase::Park, "stays parked");
        assert!(backoff.is_parking());
        backoff.reset();
        assert_eq!(backoff.phase(), Phase::Spin(1));
        assert!(!backoff.is_parking());
    }

    #[test]
    fn wait_until_returns_at_the_deadline_without_a_notify() {
        let signal = Signal::new();
        let start = Instant::now();
        let woke = wait_until(&signal, Some(start + Duration::from_millis(30)), || false);
        assert_eq!(woke, Woke::Deadline);
        let waited = start.elapsed();
        assert!(waited >= Duration::from_millis(30), "{waited:?}");
        assert!(waited < Duration::from_millis(500), "{waited:?}");
    }

    #[test]
    fn notify_wakes_a_parked_waiter_promptly() {
        let signal = Arc::new(Signal::new());
        let flag = Arc::new(AtomicBool::new(false));
        let waiter = {
            let (signal, flag) = (signal.clone(), flag.clone());
            thread::spawn(move || {
                let start = Instant::now();
                let woke = wait_until(&signal, None, || flag.load(Ordering::Acquire));
                (woke, start.elapsed())
            })
        };
        // Long enough for the waiter to reach the untimed park.
        thread::sleep(Duration::from_millis(50));
        flag.store(true, Ordering::Release);
        signal.notify();
        let (woke, waited) = waiter.join().unwrap();
        assert_eq!(woke, Woke::Ready);
        assert!(waited < Duration::from_secs(2), "{waited:?}");
    }

    #[test]
    fn no_lost_wakeups_under_ping_pong() {
        // Two threads hand a counter back and forth 20k times; a lost wake-up
        // would hang (an untimed park) and time the test out.
        let signal = Arc::new(Signal::new());
        let turn = Arc::new(AtomicU64::new(0));
        let rounds = 20_000;
        let other = {
            let (signal, turn) = (signal.clone(), turn.clone());
            thread::spawn(move || {
                for i in 0..rounds {
                    wait_until(&signal, None, || turn.load(Ordering::Acquire) == 2 * i + 1);
                    turn.store(2 * i + 2, Ordering::Release);
                    signal.notify();
                }
            })
        };
        for i in 0..rounds {
            wait_until(&signal, None, || turn.load(Ordering::Acquire) == 2 * i);
            turn.store(2 * i + 1, Ordering::Release);
            signal.notify();
        }
        other.join().unwrap();
        assert_eq!(turn.load(Ordering::Acquire), 2 * rounds);
    }

    #[test]
    fn wait_due_hits_the_due_time_and_honours_stop() {
        let stop = AtomicBool::new(false);
        for micros in [3_u64, 40, 300, 2_500] {
            let due = Instant::now() + Duration::from_micros(micros);
            assert!(wait_due(due, &stop));
            let late = Instant::now() - due;
            assert!(
                late < Duration::from_millis(5),
                "{micros} us: late {late:?}"
            );
        }
        stop.store(true, Ordering::Relaxed);
        let start = Instant::now();
        assert!(!wait_due(start + Duration::from_secs(10), &stop));
        assert!(start.elapsed() < Duration::from_millis(100));
    }
}
