//! A lossless path loses nothing at full speed: RX drains its legs fairly, so one
//! lagging leg cannot push the reorder buffer past its memory cap and force a
//! false loss declaration (design SM-D20 with the OD-5 reorder bounds).

mod common;

use common::{bin, start_rx};
use std::io::Read;
use std::net::UdpSocket;
use std::process::Stdio;

const LEGS: u16 = 16;
const SIZE: usize = 64 * 1024 * 1024;

/// A base port whose `LEGS` UDP ports are free right now.
fn free_base_for_legs() -> u16 {
    let mut base = 47000 + (std::process::id() % 100) as u16 * LEGS;
    loop {
        if (0..LEGS).all(|k| UdpSocket::bind(("127.0.0.1", base + k)).is_ok()) {
            return base;
        }
        base += LEGS;
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
fn full_speed_transfer_over_16_legs_is_lossless() {
    let input = input_bytes();
    let path = std::env::temp_dir().join(format!("socket-mux-bulk-{}.bin", std::process::id()));
    std::fs::write(&path, &input).unwrap();

    let base = free_base_for_legs().to_string();
    let globals = [
        "-n",
        "16",
        "-p",
        &base,
        "--udp-buffer",
        "8388608",
        "--timeout",
        "5",
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
    let gaps = stderr.matches("loss gap").count();
    assert_eq!(gaps, 0, "RX declared {gaps} loss gaps on a lossless path");
    assert_eq!(
        code,
        Some(0),
        "stderr: {}",
        &stderr[..stderr.len().min(2000)]
    );
    assert_eq!(output.len(), input.len(), "delivered byte count");
    assert!(output == input, "delivered bytes differ from the input");
}
