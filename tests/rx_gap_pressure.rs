//! One missing datagram must cost exactly one lost datagram: the datagrams held
//! behind its gap must not exhaust RX memory and turn into output-limit loss
//! while the TCP output keeps up.

mod common;

use common::{datagram, free_base, rx_summary, start_rx};
use std::net::UdpSocket;
use std::time::Duration;

const LANES: u64 = 2;
const DATAGRAM_SIZE: usize = 256;
const PAYLOAD: usize = DATAGRAM_SIZE - 24;
const W: usize = 64;

fn payload(seq: u64) -> Vec<u8> {
    (0..PAYLOAD).map(|i| (seq as usize * 7 + i) as u8).collect()
}

fn rx_args(base: u16, timeout: &str) -> Vec<String> {
    [
        "-n",
        &LANES.to_string(),
        "-p",
        &base.to_string(),
        "--datagram-size",
        &DATAGRAM_SIZE.to_string(),
        "--rx-memory",
        &(DATAGRAM_SIZE * W).to_string(),
        "--timeout",
        timeout,
        "-v",
        "rx",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

#[test]
fn one_missing_datagram_costs_exactly_one_memory_loss() {
    let base = free_base();
    let args = rx_args(base, "10");
    let mut rx = start_rx(&args.iter().map(String::as_str).collect::<Vec<_>>());
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    let send = |seq: u64, eos: bool| {
        let lane = base + (seq % LANES) as u16;
        let d = datagram(seq, seq * PAYLOAD as u64, &payload(seq), eos);
        socket.send_to(&d, ("127.0.0.1", lane)).unwrap();
    };

    let gap = 20_u64;
    let last = gap + 3 * W as u64;
    for seq in 0..gap {
        send(seq, false);
        std::thread::sleep(Duration::from_micros(300));
    }
    // After the gap, short bursts with pauses the TCP output easily keeps up
    // with. A burst spans the datagram at gap + W, so a release only at the
    // memory rule leaves no free buffer for the rest of that burst.
    let mut seq = gap + 1;
    while seq <= last {
        for _ in 0..6 {
            if seq > last {
                break;
            }
            send(seq, seq == last);
            seq += 1;
        }
        std::thread::sleep(Duration::from_millis(3));
    }

    let (bytes, _) = rx.read_to_end();
    let expected: Vec<u8> = (0..=last).filter(|&s| s != gap).flat_map(payload).collect();
    let (code, stderr) = rx.wait(Duration::from_secs(5));
    assert_eq!(code, Some(0), "{stderr}");
    assert!(
        rx_summary(&stderr).contains(&format!(
            "lost 1 datagrams {PAYLOAD} bytes in 1 stretches (memory 1 output 0 timeout 0)"
        )),
        "{stderr}"
    );
    assert_eq!(bytes.len(), expected.len(), "{stderr}");
    assert!(bytes == expected, "delivered bytes differ: {stderr}");
}

#[test]
fn stalled_output_behind_a_gap_still_drops_as_output_loss() {
    let base = free_base();
    let datagram_size = 65_507;
    let payload_len = datagram_size - 24;
    let rx_memory = datagram_size * 8;
    let mut rx = start_rx(&[
        "-n",
        &LANES.to_string(),
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
    // Nobody reads RX's TCP output until the stream ends: once the socket
    // buffers fill, the writer stalls and the output limit must drop, gap or
    // not.
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    let payload = vec![b'X'; payload_len];
    for seq in (0..160_u64).filter(|&seq| seq != 80) {
        let lane = base + (seq % LANES) as u16;
        let d = datagram(seq, seq * payload_len as u64, &payload, false);
        socket.send_to(&d, ("127.0.0.1", lane)).unwrap();
        std::thread::sleep(Duration::from_millis(1));
    }
    let (bytes, _) = rx.read_to_end();
    assert!(bytes.len() > 1_000_000, "the unread TCP peer fills its send buffer");
    let (code, stderr) = rx.wait(Duration::from_secs(5));
    assert_eq!(code, Some(0), "{stderr}");
    assert!(stderr.contains("cause=output"), "{stderr}");
    assert!(!rx_summary(&stderr).contains(" output 0 "), "{stderr}");
}
