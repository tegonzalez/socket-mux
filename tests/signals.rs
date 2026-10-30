//! SIGINT/SIGTERM stop RX and TX cleanly: the reporter is drained (its
//! summary is printed) and the exit status is 128 + signal number.
#![cfg(unix)]

mod common;

use common::{bin, free_base, rx_summary, send, start_rx};
use std::io::Read;
use std::process::{Child, Stdio};
use std::time::{Duration, Instant};

fn signal(child: &Child, name: &str) {
    let status = std::process::Command::new("kill")
        .args([&format!("-{name}"), &child.id().to_string()])
        .status()
        .unwrap();
    assert!(status.success(), "kill -{name}");
}

/// Wait for exit (bounded); returns the status code, the stderr and the time
/// it took after the signal.
fn wait_exit(child: &mut Child, limit: Duration) -> (Option<i32>, String, Duration) {
    let start = Instant::now();
    while start.elapsed() < limit {
        if let Some(status) = child.try_wait().unwrap() {
            let mut err = String::new();
            child
                .stderr
                .take()
                .unwrap()
                .read_to_string(&mut err)
                .unwrap();
            return (status.code(), err, start.elapsed());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    child.kill().ok();
    panic!("still running {limit:?} after the signal");
}

fn rx_stops_on(name: &str, status: i32) {
    let base = free_base();
    let p = base.to_string();
    let mut rx = start_rx(&["-n", "3", "-p", &p, "--timeout", "30", "rx"]);
    send(base, 0, 0, b"head", false);
    send(base, 2, 8, b"held", false); // held behind missing 1
    std::thread::sleep(Duration::from_millis(200));
    signal(&rx.child, name);
    let (code, stderr, took) = wait_exit(&mut rx.child, Duration::from_secs(5));
    assert_eq!(code, Some(status), "{stderr}");
    assert!(took < Duration::from_secs(1), "stopped after {took:?}");
    assert!(stderr.contains(&format!("received {name}")), "{stderr}");
    let summary = rx_summary(&stderr);
    assert!(
        summary.contains("delivered 4 bytes in 1 datagrams"),
        "{summary}"
    );
}

#[test]
fn rx_sigint_prints_the_summary_and_exits_130() {
    rx_stops_on("SIGINT", 130);
}

#[test]
fn rx_sigterm_prints_the_summary_and_exits_143() {
    rx_stops_on("SIGTERM", 143);
}

#[test]
fn idle_rx_sigint_still_prints_a_summary() {
    let base = free_base();
    let mut rx = start_rx(&["-n", "3", "-p", &base.to_string(), "rx"]);
    std::thread::sleep(Duration::from_millis(100));
    signal(&rx.child, "SIGINT");
    let (code, stderr, _) = wait_exit(&mut rx.child, Duration::from_secs(5));
    assert_eq!(code, Some(130), "{stderr}");
    assert!(rx_summary(&stderr).contains("delivered 0 bytes in 0 datagrams"));
}

fn tx_pat_stops_on(name: &str, status: i32, extra: &[&str]) {
    let base = free_base();
    let mut tx = bin()
        .args(["-n", "3", "-p", &base.to_string(), "tx-pat", "--seed", "7"])
        .args(extra)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    std::thread::sleep(Duration::from_millis(400));
    signal(&tx, name);
    let (code, stderr, took) = wait_exit(&mut tx, Duration::from_secs(5));
    assert_eq!(code, Some(status), "{stderr}");
    assert!(took < Duration::from_secs(1), "stopped after {took:?}");
    let summary = stderr
        .lines()
        .find(|l| l.contains("tx summary: "))
        .unwrap_or_else(|| panic!("no tx summary: {stderr}"));
    assert!(!summary.contains("sent 0 bytes"), "{summary}");
}

#[test]
fn tx_pat_sigint_prints_the_summary_and_exits_130() {
    tx_pat_stops_on("SIGINT", 130, &[]);
}

#[test]
fn tx_pat_sigterm_prints_the_summary_and_exits_143() {
    tx_pat_stops_on("SIGTERM", 143, &[]);
}

#[test]
fn paced_tx_pat_stops_promptly_between_due_times() {
    // 0.3 s between datagrams: the stop must not wait for the next due time.
    tx_pat_stops_on("SIGINT", 130, &["--ipd-ns", "300000000"]);
}

#[test]
fn tx_waiting_for_its_tcp_client_stops_on_sigint() {
    let base = free_base();
    let listen = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap();
    let mut tx = bin()
        .args([
            "-p",
            &base.to_string(),
            "tx",
            "--listen",
            &listen.to_string(),
        ])
        .stdin(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    std::thread::sleep(Duration::from_millis(300));
    signal(&tx, "SIGINT");
    let (code, stderr, took) = wait_exit(&mut tx, Duration::from_secs(5));
    assert_eq!(code, Some(130), "{stderr}");
    assert!(took < Duration::from_secs(1), "stopped after {took:?}");
    assert!(stderr.contains("tx summary: "), "{stderr}");
}
