//! RX end to end over loopback with injected gaps: the reorder thread's loss
//! decisions reach the reporter as merged stretches, interval lines and one
//! summary, and a stalled TCP consumer does not stop RX from receiving.

mod common;

use common::{free_base, rx_summary, send, start_rx};
use std::io::Read;
use std::time::Duration;

/// The number after `key` in `line` (e.g. "in 12 datagrams" with key "in ").
fn number_after(line: &str, key: &str) -> u64 {
    let at = line
        .find(key)
        .unwrap_or_else(|| panic!("{key:?} in {line}"))
        + key.len();
    line[at..]
        .split(|c: char| !c.is_ascii_digit())
        .next()
        .unwrap()
        .parse()
        .unwrap()
}

#[test]
fn injected_gaps_are_reported_as_stretches_intervals_and_a_summary() {
    let base = free_base();
    let p = base.to_string();
    let mut rx = start_rx(&[
        "-n",
        "3",
        "-p",
        &p,
        "--timeout",
        "0.4",
        "--interval",
        "0.2",
        "-v",
        "rx",
    ]);
    // 5, 6, 7 and 12 never arrive; the stream ends by EOS at 20.
    for seq in (0..=20_u64).filter(|s| !(5..=7).contains(s) && *s != 12) {
        send(base, seq, seq * 10, &[seq as u8; 10], seq == 20);
        std::thread::sleep(Duration::from_millis(2));
    }
    let (bytes, _) = rx.read_to_end();
    assert_eq!(bytes.len(), 17 * 10);
    let (code, stderr) = rx.wait(Duration::from_secs(3));
    assert_eq!(code, Some(0), "{stderr}");

    // Debug detail: one line per stretch (5..7 is one stretch, not three).
    let gaps: Vec<&str> = stderr.lines().filter(|l| l.contains("loss gap:")).collect();
    assert_eq!(gaps.len(), 2, "{gaps:#?}");
    assert!(gaps[0].contains("cause=timeout first_seq=5 lost_slices=3 lost_bytes=30"));
    assert!(gaps[1].contains("cause=timeout first_seq=12 lost_slices=1 lost_bytes=10"));

    let summary = rx_summary(&stderr);
    assert!(
        summary.contains(
            "delivered 170 bytes in 17 datagrams  lost 4 datagrams 40 bytes in 2 stretches \
             (memory 0 output 0 timeout 4) longest 3"
        ),
        "{summary}"
    );
    // Rule A waits T = 0.4 s per gap, so at least one 0.2 s interval passed.
    let intervals: Vec<&str> = stderr
        .lines()
        .filter(|l| l.contains(" s]  ") && !l.contains("summary"))
        .collect();
    assert!(!intervals.is_empty(), "{stderr}");
    let lost: u64 = intervals.iter().map(|l| number_after(l, "lost ")).sum();
    assert!(lost <= 4, "{intervals:#?}");
}

#[test]
fn interval_zero_prints_only_the_summary() {
    let base = free_base();
    let p = base.to_string();
    let mut rx = start_rx(&[
        "-n",
        "3",
        "-p",
        &p,
        "--interval",
        "0",
        "--timeout",
        "2",
        "rx",
    ]);
    send(base, 0, 0, b"x", false);
    std::thread::sleep(Duration::from_millis(500));
    send(base, 1, 1, b"y", true);
    let (bytes, _) = rx.read_to_end();
    assert_eq!(bytes, b"xy");
    let (code, stderr) = rx.wait(Duration::from_secs(3));
    assert_eq!(code, Some(0), "{stderr}");
    let timed: Vec<&str> = stderr.lines().filter(|l| l.contains(" s]  ")).collect();
    assert_eq!(timed.len(), 1, "only the summary: {timed:#?}");
    assert!(rx_summary(&stderr).contains("delivered 2 bytes in 2 datagrams  lost 0 datagrams"));
}

/// The writer thread blocks on a TCP consumer that does not read; the lanes
/// must keep receiving meanwhile. Every datagram is then accounted for as
/// delivered or as an output-limit drop. Had receiving stalled, the small
/// kernel receive buffer would have overflowed and datagrams would be missing
/// from both counts (or appear as memory/timeout loss).
#[test]
fn stalled_output_does_not_block_receiving() {
    let base = free_base();
    let datagram_size = 65_507;
    let payload_len = datagram_size - 24;
    let sent = 200_u64;
    let mut rx = start_rx(&[
        "-n",
        "3",
        "-p",
        &base.to_string(),
        "--datagram-size",
        &datagram_size.to_string(),
        "--rx-memory",
        &(datagram_size * 4).to_string(),
        "--udp-buffer",
        "262144",
        "--timeout",
        "0.5",
        "rx",
    ]);
    let payload = vec![b'S'; payload_len];
    for seq in 0..sent {
        send(base, seq, seq * payload_len as u64, &payload, false);
        std::thread::sleep(Duration::from_millis(1));
    }
    // Keep the consumer stalled well past rule B (T = 0.5 s after the last
    // datagram): RX ends the stream while its writer is still blocked.
    std::thread::sleep(Duration::from_millis(1500));
    let mut bytes = Vec::new();
    rx.output.read_to_end(&mut bytes).unwrap();
    let (code, stderr) = rx.wait(Duration::from_secs(3));
    assert_eq!(code, Some(0), "{stderr}");
    let summary = rx_summary(&stderr);
    let delivered = number_after(summary, "bytes in ");
    let output_lost = number_after(summary, " output ");
    assert_eq!(bytes.len() as u64, delivered * payload_len as u64);
    assert!(
        output_lost > 0,
        "the stall forced output-limit drops: {summary}"
    );
    assert_eq!(
        delivered + output_lost,
        sent,
        "every datagram received: {summary}"
    );
    assert!(summary.contains("(memory 0 "), "{summary}");
    assert!(summary.contains(" timeout 0)"), "{summary}");
}
