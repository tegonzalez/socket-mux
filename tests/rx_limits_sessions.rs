mod common;

use common::{free_base, rx_summary, send, send_session, start_rx};
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
        "--log-level",
        "debug",
        "rx",
    ]);
    // Lanes are received by concurrent threads, so datagrams sent back to back
    // on different lanes may reach the reorder in either order; space them.
    send(base, 0, 0, b"A", false);
    std::thread::sleep(Duration::from_millis(30));
    send(base, 4, 4, b"E", false);
    std::thread::sleep(Duration::from_millis(30));
    send(base, 5, 5, b"F", true);

    let (bytes, _) = rx.read_to_end();
    assert_eq!(bytes, b"AEF");
    let (code, stderr) = rx.wait(Duration::from_secs(3));
    assert_eq!(code, Some(0), "{stderr}");
    assert!(stderr.contains("cause=memory"), "{stderr}");
    assert!(stderr.contains("lost_slices=3 lost_bytes=3"), "{stderr}");
    assert!(
        rx_summary(&stderr)
            .contains("lost 3 datagrams 3 bytes in 1 stretches (memory 3 output 0 timeout 0)"),
        "{stderr}"
    );
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
    // Let the repeat of 0 be judged before 1 completes the stream (lanes are
    // received concurrently, so back-to-back sends may be seen in either order).
    std::thread::sleep(Duration::from_millis(100));
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
    let mut rx = start_rx(&[
        "-n",
        "3",
        "-p",
        &base.to_string(),
        "--timeout",
        "2",
        "-v",
        "rx",
    ]);
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
        "-v",
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
    assert!(rx_summary(&stderr).contains("timeout 0)"), "{stderr}");
}

#[test]
fn one_buffer_rx_delivers_a_datagram() {
    let base = free_base();
    let mut rx = start_rx(&[
        "-n",
        "3",
        "-p",
        &base.to_string(),
        "--datagram-size",
        "128",
        "--rx-memory",
        "128",
        "--timeout",
        "0.2",
        "rx",
    ]);
    send(base, 0, 0, b"one buffer", true);
    let (bytes, _) = rx.read_to_end();
    assert_eq!(bytes, b"one buffer");
    let (code, stderr) = rx.wait(Duration::from_secs(3));
    assert_eq!(code, Some(0), "{stderr}");
    assert!(rx_summary(&stderr).contains("lost 0 datagrams"), "{stderr}");
}

#[cfg(any(feature = "sequence-8", feature = "sequence-16"))]
#[test]
fn startup_reorders_older_datagrams_across_rollover() {
    let last = if cfg!(feature = "sequence-8") {
        255
    } else {
        65_535
    };
    let base = free_base();
    let mut rx = start_rx(&[
        "-n",
        "3",
        "-p",
        &base.to_string(),
        "--datagram-size",
        "128",
        "--rx-memory",
        "1024",
        "--timeout",
        "0.3",
        "rx",
    ]);
    send(base, last, 1, b"B", false);
    std::thread::sleep(Duration::from_millis(30));
    send(base, 0, 2, b"C", true);
    std::thread::sleep(Duration::from_millis(30));
    send(base, last - 1, 0, b"A", false);
    let (bytes, _) = rx.read_to_end();
    assert_eq!(bytes, b"ABC");
    let (code, stderr) = rx.wait(Duration::from_secs(3));
    assert_eq!(code, Some(0), "{stderr}");
    assert!(!stderr.contains("loss gap"), "{stderr}");
}
