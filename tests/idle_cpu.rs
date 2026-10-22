//! No busy-waiting or polling (design SM-D20): an idle RX or TX uses close to no CPU.
#![cfg(target_os = "linux")]

mod common;

use common::{bin, cpu_ticks, free_base, start_rx};
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
    assert!(ticks <= MAX_TICKS, "idle RX used {ticks} ticks in {SAMPLE:?}");
}

#[test]
fn idle_tx_does_not_spin() {
    let base = free_base();
    let listen = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap();
    let mut tx = bin()
        .args(["-n", "3", "-p", &base.to_string(), "tx", "--listen", &listen.to_string()])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let ticks = idle_ticks(tx.id());
    tx.kill().ok();
    tx.wait().ok();
    assert!(ticks <= MAX_TICKS, "idle TX used {ticks} ticks in {SAMPLE:?}");
}
