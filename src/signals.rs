//! SIGINT/SIGTERM (Unix) and Ctrl-C / console close (Windows): stop the run
//! cleanly and exit with 128 + signal number (130 for SIGINT, 143 for
//! SIGTERM; always 130 on Windows).
//!
//! The handler only records the signal and sets the run's [`Stop`]; the
//! threads wind down, the reporter prints its summary, and `main` exits with
//! [`exit_status`]. If the run is still stuck after [`GRACE`] (for example
//! TX blocked reading a terminal on standard input), a watchdog runs the
//! registered finisher (the summary) and exits. A second signal exits at once.

use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use anyhow::Result;
use log::{info, warn};

use crate::stop::Stop;

/// How long a stopped run may take to wind down before the watchdog exits.
pub const GRACE: Duration = Duration::from_secs(2);

const SIGINT: i32 = 2;
#[cfg(unix)]
const SIGTERM: i32 = 15;

static RECEIVED: AtomicI32 = AtomicI32::new(0);

type Finisher = Box<dyn Fn() + Send + Sync>;
static FINISHER: Mutex<Option<Finisher>> = Mutex::new(None);

/// The exit status for the signal that stopped the run, if one did.
pub fn exit_status() -> Option<u8> {
    match RECEIVED.load(Ordering::SeqCst) {
        0 => None,
        signal => Some(status_for(signal)),
    }
}

fn status_for(signal: i32) -> u8 {
    (128 + signal).clamp(0, 255) as u8
}

fn name(signal: i32) -> &'static str {
    match signal {
        SIGINT => "SIGINT",
        #[cfg(unix)]
        SIGTERM => "SIGTERM",
        _ => "signal",
    }
}

/// Register what the watchdog runs before a forced exit (print the summary).
pub fn set_finisher(finisher: impl Fn() + Send + Sync + 'static) {
    *FINISHER.lock().unwrap_or_else(|e| e.into_inner()) = Some(Box::new(finisher));
}

/// Record `signal` and stop the run; a repeated signal exits immediately.
fn on_signal(signal: i32, stop: &Arc<Stop>) {
    if RECEIVED
        .compare_exchange(0, signal, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        std::process::exit(i32::from(exit_status().unwrap_or(status_for(signal))));
    }
    info!(target: "socket_mux", "received {}; stopping", name(signal));
    stop.set();
    let _ = thread::Builder::new()
        .name("stop-watchdog".into())
        .spawn(|| {
            thread::sleep(GRACE);
            warn!(target: "socket_mux", "did not stop within {GRACE:?}; exiting");
            if let Some(finish) = FINISHER.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
                finish();
            }
            log::logger().flush();
            std::process::exit(i32::from(exit_status().unwrap_or(130)));
        });
}

/// Install the handlers for this run.
#[cfg(unix)]
pub fn install(stop: Arc<Stop>) -> Result<()> {
    use anyhow::Context;
    use signal_hook::iterator::Signals;

    let mut signals =
        Signals::new([SIGINT, SIGTERM]).context("could not install signal handlers")?;
    thread::Builder::new()
        .name("signals".into())
        .spawn(move || {
            // Blocks in a read on signal-hook's self-pipe: no CPU while idle.
            for signal in signals.forever() {
                on_signal(signal, &stop);
            }
        })
        .context("could not start the signal thread")?;
    Ok(())
}

#[cfg(windows)]
pub fn install(stop: Arc<Stop>) -> Result<()> {
    use std::sync::OnceLock;
    use windows_sys::Win32::System::Console::SetConsoleCtrlHandler;

    static STOP: OnceLock<Arc<Stop>> = OnceLock::new();

    // Runs on a thread the OS creates per event. Returning ends the process
    // for console close, so wait (bounded) for main to exit on its own.
    unsafe extern "system" fn handler(_event: u32) -> i32 {
        if let Some(stop) = STOP.get() {
            on_signal(SIGINT, stop);
        }
        thread::sleep(GRACE + Duration::from_secs(1));
        1
    }

    let _ = STOP.set(stop);
    // SAFETY: registers a handler with the documented signature.
    if unsafe { SetConsoleCtrlHandler(Some(handler), 1) } == 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(())
}

#[cfg(not(any(unix, windows)))]
pub fn install(_stop: Arc<Stop>) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_status_is_128_plus_the_signal() {
        assert_eq!(status_for(2), 130);
        assert_eq!(status_for(15), 143);
    }
}
