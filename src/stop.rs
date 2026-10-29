//! Cooperative stop for one run: a flag, the signals of parked threads to wake,
//! and hooks that unblock blocking syscalls (shut a TCP socket, poke a UDP
//! lane). Setting it is idempotent; every waiter re-checks the flag.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::wait::Signal;

type Hook = Box<dyn Fn() + Send + Sync>;

#[derive(Default)]
pub struct Stop {
    flag: AtomicBool,
    signals: Mutex<Vec<Arc<Signal>>>,
    hooks: Mutex<Vec<Hook>>,
}

impl Stop {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn is_set(&self) -> bool {
        self.flag.load(Ordering::Relaxed)
    }

    /// The raw flag, for queue waits that take one.
    pub fn flag(&self) -> &AtomicBool {
        &self.flag
    }

    /// Notify `signal` when the stop is set, so its waiters re-check the flag.
    pub fn watch(&self, signal: Arc<Signal>) {
        self.signals
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(signal);
        if self.is_set() {
            self.wake();
        }
    }

    /// Run `hook` when the stop is set (at once if it already is). Hooks
    /// unblock threads stuck in blocking syscalls.
    pub fn on_stop(&self, hook: impl Fn() + Send + Sync + 'static) {
        if self.is_set() {
            hook();
        }
        self.hooks
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(Box::new(hook));
    }

    /// Set the stop: wake every watched signal and run every hook once.
    pub fn set(&self) {
        if self.flag.swap(true, Ordering::SeqCst) {
            return;
        }
        self.wake();
        for hook in self.hooks.lock().unwrap_or_else(|e| e.into_inner()).iter() {
            hook();
        }
    }

    fn wake(&self) {
        for signal in self.signals.lock().unwrap_or_else(|e| e.into_inner()).iter() {
            signal.notify();
        }
    }
}
