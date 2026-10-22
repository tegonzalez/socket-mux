//! RX ends the stream when the global timeout expires (design SM-D19): it declares
//! outstanding gaps lost, delivers what it holds, closes its output and exits 0.
//! Timer rules are PROPOSAL SM-D21: the timer starts at the first datagram and
//! restarts on every datagram on any leg.

mod common;

use common::{free_base, send, start_rx};
use std::time::{Duration, Instant};

fn assert_timeout_ends_stream(args: &[&str], base: u16, timeout: Duration) {
    let mut rx = start_rx(args);
    send(base, 0, 0, &[b'a'; 100], false);
    send(base, 2, 200, &[b'c'; 100], false); // sequence 1 (leg 1) is lost; no end of stream
    let sent = Instant::now();
    let (bytes, closed) = rx.read_to_end();
    let after = closed - sent;
    assert_eq!(bytes, [[b'a'; 100], [b'c'; 100]].concat());
    assert!(after >= timeout.mul_f32(0.8), "closed after {after:?}, before the timeout");
    assert!(after <= timeout + Duration::from_secs(2), "closed after {after:?}");
    let (code, stderr) = rx.wait(Duration::from_secs(3));
    assert_eq!(code, Some(0), "stderr: {stderr}");
    assert!(
        stderr.contains("lost_slices=1 lost_bytes=100"),
        "loss of sequence 1 reported: {stderr}"
    );
}

#[test]
fn timeout_before_the_subcommand_ends_the_stream() {
    let base = free_base();
    let p = base.to_string();
    assert_timeout_ends_stream(
        &["--timeout", "0.5", "-n", "3", "-p", &p, "rx"],
        base,
        Duration::from_millis(500),
    );
}

#[test]
fn timeout_after_the_subcommand_ends_the_stream() {
    let base = free_base();
    let p = base.to_string();
    assert_timeout_ends_stream(
        &["rx", "-n", "3", "-p", &p, "--timeout", "0.5"],
        base,
        Duration::from_millis(500),
    );
}

#[test]
fn timeout_shorter_than_the_reorder_wait_still_reports_the_gap() {
    let base = free_base();
    let p = base.to_string();
    assert_timeout_ends_stream(
        &["-n", "3", "-p", &p, "--timeout", "0.1", "rx"],
        base,
        Duration::from_millis(100),
    );
}

#[test]
fn timer_waits_for_the_first_datagram() {
    let base = free_base();
    let p = base.to_string();
    let mut rx = start_rx(&["-n", "3", "-p", &p, "--timeout", "0.2", "rx"]);
    std::thread::sleep(Duration::from_secs(1));
    assert!(rx.child.try_wait().unwrap().is_none(), "RX ended before any datagram");
    send(base, 0, 0, b"only", true);
    let (bytes, _) = rx.read_to_end();
    assert_eq!(bytes, b"only");
    let (code, stderr) = rx.wait(Duration::from_secs(3));
    assert_eq!(code, Some(0), "stderr: {stderr}");
}
