//! `tx-pat --bytes N` sends exactly N pattern bytes then EOS and exits 0;
//! `--ipd-ns` paces it. Through RX the pattern checks clean with check-pat.

mod common;

use common::{bin, free_base, rx_summary, start_rx};
use std::io::Write;
use std::process::Stdio;
use std::time::{Duration, Instant};

#[test]
fn paced_tx_pat_with_bytes_delivers_exactly_n_checkable_bytes() {
    let base = free_base();
    let p = base.to_string();
    let bytes: u64 = 1_000_000;
    let ipd_ns: u64 = 200_000;
    // The pacing bound below counts 1428-byte payloads, so the datagram size
    // is fixed at 1452 rather than derived from the loopback MTU.
    let size = ["--datagram-size", "1452"];
    let mut rx = start_rx(&[&size[..], &["-n", "3", "-p", &p, "--timeout", "2", "rx"]].concat());
    let began = Instant::now();
    let tx = bin()
        .args(size)
        .args(["-n", "3", "-p", &p, "tx-pat", "--seed", "5"])
        .args([
            "--bytes",
            &bytes.to_string(),
            "--ipd-ns",
            &ipd_ns.to_string(),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    let took = began.elapsed();
    let tx_log = String::from_utf8_lossy(&tx.stderr);
    assert_eq!(tx.status.code(), Some(0), "{tx_log}");
    assert!(
        tx_log.contains("tx summary: ") && tx_log.contains(&format!("sent {bytes} bytes")),
        "{tx_log}"
    );
    // 701 data datagrams of 1428 bytes plus EOS, one every 200 us.
    let datagrams = bytes.div_ceil(1428) + 1;
    let paced = Duration::from_nanos((datagrams - 1) * ipd_ns);
    assert!(took >= paced, "paced run took {took:?}, below {paced:?}");

    let (output, _) = rx.read_to_end();
    assert_eq!(output.len() as u64, bytes);
    let (code, stderr) = rx.wait(Duration::from_secs(3));
    assert_eq!(code, Some(0), "{stderr}");
    assert!(rx_summary(&stderr).contains("lost 0 datagrams"), "{stderr}");

    let mut check = bin()
        .args(["check-pat", "--seed", "5"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    check.stdin.take().unwrap().write_all(&output).unwrap();
    let checked = check.wait_with_output().unwrap();
    let line = String::from_utf8_lossy(&checked.stdout);
    assert_eq!(checked.status.code(), Some(0), "{line}");
    assert!(
        line.contains("blocks_ok=244 blocks_bad=0 resyncs=0 lost_bytes=0"),
        "{line}"
    );
}
