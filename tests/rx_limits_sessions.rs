mod common;

use common::{free_base, send, send_session, start_rx};
use std::time::Duration;

#[test]
fn deep_reorder_declares_memory_limit_loss() {
    let base = free_base();
    let mut rx = start_rx(&[
        "-n",
        "3",
        "-p",
        &base.to_string(),
        "--datagram-size",
        "128",
        "--rx-memory",
        "512",
        "--timeout",
        "1",
        "rx",
    ]);
    send(base, 0, 0, b"A", false);
    send(base, 4, 4, b"E", false);
    send(base, 5, 5, b"F", true);

    let (bytes, _) = rx.read_to_end();
    assert_eq!(bytes, b"AEF");
    let (code, stderr) = rx.wait(Duration::from_secs(3));
    assert_eq!(code, Some(0), "{stderr}");
    assert!(stderr.contains("cause=memory"), "{stderr}");
    assert!(stderr.contains("lost_slices=3 lost_bytes=3"), "{stderr}");
}

#[test]
fn duplicate_and_late_datagrams_are_dropped_and_logged() {
    let base = free_base();
    let mut rx = start_rx(&[
        "-n",
        "3",
        "-p",
        &base.to_string(),
        "--log-level",
        "debug",
        "--timeout",
        "1",
        "rx",
    ]);
    send(base, 2, 2, b"C", true);
    send(base, 2, 2, b"C", true);
    std::thread::sleep(Duration::from_millis(100));
    send(base, 0, 0, b"A", false);
    std::thread::sleep(Duration::from_millis(100));
    send(base, 0, 0, b"A", false);
    send(base, 1, 1, b"B", false);

    let (bytes, _) = rx.read_to_end();
    assert_eq!(bytes, b"ABC");
    let (code, stderr) = rx.wait(Duration::from_secs(3));
    assert_eq!(code, Some(0), "{stderr}");
    assert!(stderr.contains("discarding duplicate datagram"), "{stderr}");
    assert!(stderr.contains("discarding late datagram"), "{stderr}");
}

#[test]
fn session_change_flushes_current_stream_and_exits_successfully() {
    let base = free_base();
    let mut rx = start_rx(&["-n", "3", "-p", &base.to_string(), "--timeout", "2", "rx"]);
    send_session(base, 41, 0, 0, b"head", false);
    send_session(base, 41, 2, 8, b"tail", false);
    std::thread::sleep(Duration::from_millis(100));
    send_session(base, 42, 0, 0, b"new", false);

    let (bytes, _) = rx.read_to_end();
    assert_eq!(bytes, b"headtail");
    let (code, stderr) = rx.wait(Duration::from_secs(3));
    assert_eq!(code, Some(0), "{stderr}");
    assert!(
        stderr.contains("new session 42 (current 41); ending stream"),
        "{stderr}"
    );
    assert!(stderr.contains("cause=timeout"), "{stderr}");
}

#[test]
fn stalled_tcp_consumer_causes_output_limit_drops() {
    let base = free_base();
    let datagram_size = 65_507;
    let payload_len = datagram_size - 24;
    let rx_memory = datagram_size * 4;
    let mut rx = start_rx(&[
        "-n",
        "3",
        "-p",
        &base.to_string(),
        "--datagram-size",
        &datagram_size.to_string(),
        "--rx-memory",
        &rx_memory.to_string(),
        "--timeout",
        "0.5",
        "rx",
    ]);
    let payload = vec![b'X'; payload_len];
    for seq in 0..160_u64 {
        send(base, seq, seq * payload_len as u64, &payload, false);
        std::thread::sleep(Duration::from_millis(1));
    }
    let (bytes, _) = rx.read_to_end();
    assert!(
        bytes.len() > 1_000_000,
        "the unread TCP peer fills its send buffer"
    );
    let (code, stderr) = rx.wait(Duration::from_secs(3));
    assert_eq!(code, Some(0), "{stderr}");
    assert!(stderr.contains("cause=output"), "{stderr}");
}
