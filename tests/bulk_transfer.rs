//! A lossless path loses nothing at full speed: RX drains its lanes fairly, so one
//! lagging lane cannot push the reorder buffer past its memory cap and force a
//! false loss declaration (design SM-D20 with the reorder bounds).

mod common;

use common::{bin, rx_summary, start_rx};
use std::io::Read;
use std::net::UdpSocket;
use std::process::Stdio;

const LANES: u16 = 16;
// This fixture queues the entire stream before reading TCP. Keep its input and
// holding pool below half the sequence space in reduced-width builds.
const SIZE: usize = if cfg!(feature = "sequence-8") {
    64 * 1024
} else if cfg!(feature = "sequence-16") {
    16 * 1024 * 1024
} else {
    64 * 1024 * 1024
};

/// A base port whose `LANES` UDP ports are free right now.
fn free_base_for_lanes() -> u16 {
    let mut base = 47000 + (std::process::id() % 100) as u16 * LANES;
    loop {
        if (0..LANES).all(|k| UdpSocket::bind(("127.0.0.1", base + k)).is_ok()) {
            return base;
        }
        base += LANES;
    }
}

/// Deterministic incompressible bytes (xorshift64).
fn input_bytes() -> Vec<u8> {
    let mut state = 0x9E37_79B9_7F4A_7C15_u64;
    let mut bytes = Vec::with_capacity(SIZE);
    while bytes.len() < SIZE {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        bytes.extend_from_slice(&state.to_le_bytes());
    }
    bytes
}

#[test]
fn full_speed_transfer_over_16_lanes_is_lossless() {
    let input = input_bytes();
    let path =
        std::path::Path::new("target").join(format!("socket-mux-bulk-{}.bin", std::process::id()));
    std::fs::write(&path, &input).unwrap();

    let base = free_base_for_lanes().to_string();
    let memory = if cfg!(feature = "sequence-8") {
        64 * 1452
    } else if cfg!(feature = "sequence-16") {
        16_384 * 1452
    } else {
        67_108_864
    }
    .to_string();
    let globals = [
        "-n",
        "16",
        "-p",
        &base,
        "--udp-buffer",
        "8388608",
        "--timeout",
        "5",
        "--rx-memory",
        &memory,
    ];
    let mut rx = start_rx(&[&globals[..], &["rx"]].concat());
    // Drain RX's log as it runs: a flood of loss lines must not block RX on a full pipe.
    let mut log = rx.child.stderr.take().unwrap();
    let log_reader = std::thread::spawn(move || {
        let mut text = String::new();
        log.read_to_string(&mut text).unwrap();
        text
    });
    let tx = bin()
        .args(globals)
        .arg("tx")
        .arg(&path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    std::fs::remove_file(&path).ok();
    assert!(
        tx.status.success(),
        "tx failed: {}",
        String::from_utf8_lossy(&tx.stderr)
    );

    let (output, _) = rx.read_to_end();
    let code = rx.child.wait().unwrap().code();
    let stderr = log_reader.join().unwrap();
    let summary = rx_summary(&stderr);
    assert!(
        summary.contains("lost 0 datagrams"),
        "RX declared loss on a lossless path: {summary}"
    );
    assert_eq!(
        code,
        Some(0),
        "stderr: {}",
        &stderr[..stderr.len().min(2000)]
    );
    assert_eq!(output.len(), input.len(), "delivered byte count");
    assert!(output == input, "delivered bytes differ from the input");
}
