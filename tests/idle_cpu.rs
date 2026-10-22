//! No busy-waiting or polling (design SM-D20): an idle RX or TX uses close to no CPU, also
//! while RX holds datagrams behind a gap (design SM-D19: timestamps against one monotonic
//! clock, no timer per datagram).
#![cfg(target_os = "linux")]

mod common;

use common::{bin, cpu_ticks, free_base, send, start_rx};
use std::io::Read;
use std::net::TcpListener;
use std::process::Stdio;
use std::time::Duration;

const SAMPLE: Duration = Duration::from_secs(2);
/// 5% of one core over the sample, in clock ticks at the usual 100 per second.
const MAX_TICKS: u64 = 10;

fn idle_ticks(pid: u32) -> u64 {
    std::thread::sleep(Duration::from_millis(300));
    let before = cpu_ticks(pid);
    std::thread::sleep(SAMPLE);
    cpu_ticks(pid) - before
}

#[test]
fn idle_rx_does_not_spin() {
    let base = free_base();
    let p = base.to_string();
    let rx = start_rx(&["-n", "3", "-p", &p, "rx"]);
    let ticks = idle_ticks(rx.child.id());
    assert!(
        ticks <= MAX_TICKS,
        "idle RX used {ticks} ticks in {SAMPLE:?}"
    );
}

#[test]
fn idle_tx_does_not_spin() {
    let base = free_base();
    let listen = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap();
    let mut tx = bin()
        .args([
            "-n",
            "3",
            "-p",
            &base.to_string(),
            "tx",
            "--listen",
            &listen.to_string(),
        ])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let ticks = idle_ticks(tx.id());
    tx.kill().ok();
    tx.wait().ok();
    assert!(
        ticks <= MAX_TICKS,
        "idle TX used {ticks} ticks in {SAMPLE:?}"
    );
}

#[test]
fn rx_holding_datagrams_behind_a_gap_does_not_spin_or_deliver_past_it() {
    let base = free_base();
    let p = base.to_string();
    let mut rx = start_rx(&["-n", "3", "-p", &p, "--timeout", "30", "rx"]);
    send(base, 0, 0, &[0; 100], false);
    for seq in 2..=500_u64 {
        send(base, seq, seq * 100, &[1; 100], false); // held behind missing 1
    }
    let ticks = idle_ticks(rx.child.id());
    assert!(
        ticks <= MAX_TICKS,
        "RX holding 499 datagrams used {ticks} ticks in {SAMPLE:?}"
    );
    rx.output
        .set_read_timeout(Some(Duration::from_millis(200)))
        .unwrap();
    let mut delivered = vec![0; 200];
    let n = rx.output.read(&mut delivered).unwrap();
    assert_eq!(n, 100, "only datagram 0 delivered while 1 is missing");
    let more = rx.output.read(&mut delivered);
    assert!(more.is_err(), "nothing past the gap: {more:?}");
}
