//! The inter-packet delay timeout (design SM-D19). RX delivers strictly in sequence. (A) A
//! missing sequence number, with the consecutive missing ones after it, expires once the
//! timeout has passed since the datagram before it arrived; RX then reports one gap and
//! resumes delivery. (B) RX ends the stream once the timeout has passed since the last
//! datagram. End of stream is processed only in sequence. Tests scale the default 10 s down.

mod common;

use common::{free_base, send, start_rx, Rx};
use std::io::Read;
use std::time::{Duration, Instant};

const LEN: usize = 100;

/// Payload of sequence `seq`: LEN bytes that name it, so delivery order is visible.
fn payload(seq: u64) -> Vec<u8> {
    vec![(seq % 251) as u8; LEN]
}

fn send_seq(base: u16, seq: u64, eos: bool) {
    send(base, seq, seq * LEN as u64, &payload(seq), eos);
}

fn expected(seqs: impl IntoIterator<Item = u64>) -> Vec<u8> {
    seqs.into_iter().flat_map(payload).collect()
}

/// Reads exactly `n` bytes of RX's output; returns them and when the last one arrived.
fn read_n(rx: &mut Rx, n: usize) -> (Vec<u8>, Instant) {
    let mut bytes = vec![0; n];
    rx.output.read_exact(&mut bytes).unwrap();
    (bytes, Instant::now())
}

fn gap_lines(stderr: &str) -> Vec<&str> {
    stderr.lines().filter(|l| l.contains("loss gap")).collect()
}

fn rx_with_timeout(base: u16, timeout: &str) -> Rx {
    start_rx(&[
        "-n",
        "3",
        "-p",
        &base.to_string(),
        "--timeout",
        timeout,
        "rx",
    ])
}

/// Worked example: datagrams 1, 2, 6 and 7 arrive at 0 s and 10005 and 10006 at 10 s, so the
/// inter-packet delay from 2 to 3 has expired, and 4 and 5 with it. Scaled to a 1 s timeout,
/// with sequence 0 sent first so the stream has no start-of-stream gap.
#[test]
fn worked_example_gap_expires_the_timeout_after_the_datagram_before_it() {
    let timeout = Duration::from_secs(1);
    let base = free_base();
    let mut rx = rx_with_timeout(base, "1");
    for seq in [0, 1, 2, 6, 7] {
        send_seq(base, seq, false);
    }
    let sent = Instant::now();
    let (head, _) = read_n(&mut rx, 3 * LEN);
    assert_eq!(head, expected(0..3), "0, 1 and 2 are delivered at once");

    // A steady stream keeps arriving past the timeout, so only rule A can release 6.
    let feeder = std::thread::spawn(move || {
        let mut seq = 8;
        while sent.elapsed() < timeout.mul_f32(1.3) {
            send_seq(base, seq, false);
            seq += 1;
            std::thread::sleep(Duration::from_millis(50));
        }
        send_seq(base, seq, true);
        seq
    });
    let (six, at) = read_n(&mut rx, LEN);
    let waited = at - sent;
    assert_eq!(six, payload(6), "delivery resumes at 6, in sequence");
    assert!(
        waited >= timeout.mul_f32(0.95),
        "6 delivered {waited:?} after 2 arrived, before the {timeout:?} timeout"
    );
    assert!(
        waited <= timeout + Duration::from_millis(500),
        "6 delivered {waited:?} after 2"
    );

    let last = feeder.join().unwrap();
    let (rest, _) = rx.read_to_end();
    assert_eq!(rest, expected(7..=last), "7 onward delivered in order");
    let (code, stderr) = rx.wait(Duration::from_secs(3));
    assert_eq!(code, Some(0), "stderr: {stderr}");
    let gaps = gap_lines(&stderr);
    assert_eq!(gaps.len(), 1, "one gap for 3, 4 and 5: {gaps:?}");
    assert!(
        gaps[0].contains("first_seq=3 lost_slices=3 lost_bytes=300"),
        "{}",
        gaps[0]
    );
}

#[test]
fn datagram_reordered_within_the_timeout_is_not_lost() {
    let base = free_base();
    let mut rx = rx_with_timeout(base, "1.5");
    send_seq(base, 0, false);
    send_seq(base, 2, false);
    std::thread::sleep(Duration::from_millis(700));
    send_seq(base, 1, false);
    send_seq(base, 3, true);
    let (bytes, _) = rx.read_to_end();
    assert_eq!(bytes, expected(0..4));
    let (code, stderr) = rx.wait(Duration::from_secs(3));
    assert_eq!(code, Some(0), "stderr: {stderr}");
    assert!(gap_lines(&stderr).is_empty(), "no loss: {stderr}");
}

#[test]
fn gap_is_timed_from_the_datagram_before_it_not_from_the_held_one() {
    let timeout = Duration::from_millis(1500);
    let base = free_base();
    let mut rx = rx_with_timeout(base, "1.5");
    send_seq(base, 0, false);
    let sent = Instant::now();
    let (zero, _) = read_n(&mut rx, LEN);
    assert_eq!(zero, payload(0));
    std::thread::sleep(Duration::from_millis(600));
    send_seq(base, 2, false); // held behind missing 1
    send_seq(base, 3, true);
    let (rest, closed) = rx.read_to_end();
    let after = closed - sent;
    assert_eq!(rest, expected(2..4));
    // From 0's arrival: about 1.5 s. Timed from 2's arrival it would be about 2.1 s.
    assert!(
        after >= timeout.mul_f32(0.95) && after <= timeout + Duration::from_millis(450),
        "gap 1 expired {after:?} after 0 arrived, expected about {timeout:?}"
    );
    let (code, stderr) = rx.wait(Duration::from_secs(3));
    assert_eq!(code, Some(0), "stderr: {stderr}");
    let gaps = gap_lines(&stderr);
    assert_eq!(gaps.len(), 1, "{gaps:?}");
    assert!(gaps[0].contains("first_seq=1 lost_slices=1"), "{}", gaps[0]);
}

#[test]
fn consecutive_gaps_expire_together_when_their_datagrams_arrived_together() {
    let timeout = Duration::from_millis(1500);
    let base = free_base();
    let mut rx = rx_with_timeout(base, "1.5");
    for seq in [0, 2, 4] {
        send_seq(base, seq, seq == 4);
    }
    let sent = Instant::now();
    let (bytes, closed) = rx.read_to_end();
    let after = closed - sent;
    assert_eq!(bytes, expected([0, 2, 4]));
    // Gap 3 is timed from 2's arrival, which is when 1 was sent: both expire at about 1.5 s.
    assert!(
        after >= timeout.mul_f32(0.95) && after <= timeout + Duration::from_millis(450),
        "stream ended {after:?} after the datagrams arrived, expected about {timeout:?}"
    );
    let (code, stderr) = rx.wait(Duration::from_secs(3));
    assert_eq!(code, Some(0), "stderr: {stderr}");
    let gaps = gap_lines(&stderr);
    assert_eq!(gaps.len(), 2, "{gaps:?}");
    assert!(gaps[0].contains("first_seq=1 lost_slices=1"), "{}", gaps[0]);
    assert!(gaps[1].contains("first_seq=3 lost_slices=1"), "{}", gaps[1]);
}

#[test]
fn end_of_stream_is_processed_only_in_sequence() {
    let base = free_base();
    let mut rx = rx_with_timeout(base, "5");
    send_seq(base, 2, true); // end of stream arrives first
    std::thread::sleep(Duration::from_millis(200));
    send_seq(base, 0, false);
    std::thread::sleep(Duration::from_millis(200));
    send_seq(base, 1, false);
    let started = Instant::now();
    let (bytes, closed) = rx.read_to_end();
    assert_eq!(bytes, expected(0..3));
    assert!(
        closed - started < Duration::from_secs(2),
        "ended at once once 1 arrived"
    );
    let (code, stderr) = rx.wait(Duration::from_secs(3));
    assert_eq!(code, Some(0), "stderr: {stderr}");
    assert!(gap_lines(&stderr).is_empty(), "no loss: {stderr}");
}
