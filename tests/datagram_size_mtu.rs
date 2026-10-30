//! The default datagram size comes from the adapter MTU (less 48); an explicit
//! `--datagram-size` above it warns once; RX discards datagrams larger than
//! its own size.

mod common;

use common::{bin, bin_derived_size, free_base, rx_summary, start_rx};
use std::io::Write;
use std::net::TcpListener;
use std::process::{Output, Stdio};
use std::time::Duration;

/// `tx -` to loopback with `input` on standard input.
fn tx(args: &[&str], input: &[u8]) -> Output {
    let mut child = bin_derived_size()
        .args(args)
        .args(["tx", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    child.wait_with_output().unwrap()
}

fn size_lines(log: &str) -> Vec<&str> {
    log.lines()
        .filter(|line| line.contains("] datagram size "))
        .collect()
}

#[test]
fn tx_logs_the_derived_size_and_its_source_once() {
    let p = free_base().to_string();
    let out = tx(&["-p", &p], b"");
    let log = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "{log}");
    let lines = size_lines(&log);
    assert_eq!(lines.len(), 1, "{log}");
    assert!(lines[0].contains("INFO"), "{log}");
    assert!(!log.contains("WARN"), "{log}");
    // Linux loopback: MTU 65536, so 65536 - 48.
    #[cfg(target_os = "linux")]
    assert!(
        lines[0].ends_with("datagram size 65488 (lo mtu 65536)"),
        "{log}"
    );
}

#[test]
fn an_explicit_size_is_logged_as_such() {
    let p = free_base().to_string();
    let out = tx(&["-p", &p, "--datagram-size", "1452"], b"");
    let log = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "{log}");
    let lines = size_lines(&log);
    assert_eq!(lines.len(), 1, "{log}");
    assert!(
        lines[0].contains("datagram size 1452 (--datagram-size; "),
        "{log}"
    );
    assert!(!log.contains("exceeds"), "{log}");
}

// Loopback MTU is 65536 on Linux and 16384 on macOS, so 65507 exceeds it less 48.
#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn an_explicit_size_above_the_loopback_mtu_warns_once_and_proceeds() {
    let p = free_base().to_string();
    let input = vec![7_u8; 100_000];
    let out = tx(&["-p", &p, "--datagram-size", "65507"], &input);
    let log = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "{log}");
    let warnings: Vec<&str> = log.lines().filter(|l| l.contains("exceeds")).collect();
    assert_eq!(warnings.len(), 1, "{log}");
    assert!(warnings[0].contains("WARN"), "{log}");
    assert!(
        warnings[0].contains("datagram size 65507 exceeds lo"),
        "{log}"
    );
    assert!(
        warnings[0].ends_with(
            "less 48; datagrams will be IP-fragmented and a lost fragment loses the datagram"
        ),
        "{log}"
    );
    assert!(log.contains("sent 100000 bytes"), "{log}");

    // RX warns the same way, from the adapter owning its bind address.
    let sink = TcpListener::bind("127.0.0.1:0").unwrap();
    let connect = sink.local_addr().unwrap().to_string();
    drop(sink); // RX then fails to connect, after its start-up logging.
    let out = bin_derived_size()
        .args(["-p", &p, "rx", "--datagram-size", "65507", "-c", &connect])
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    let log = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{log}");
    assert_eq!(
        log.matches("datagram size 65507 exceeds lo").count(),
        1,
        "{log}"
    );
}

#[test]
fn rx_discards_datagrams_larger_than_its_own_size() {
    // TX sends 8000-byte datagrams; RX is fixed at 1452, so it discards every
    // data datagram and only the 24-byte EOS arrives. A file input (not a
    // pipe) makes every data datagram full-sized.
    let p = free_base().to_string();
    let path = std::env::temp_dir().join(format!("socket-mux-oversize-{}", std::process::id()));
    std::fs::write(&path, vec![1_u8; 20_000]).unwrap();
    let mut rx = start_rx(&[
        "-n",
        "1",
        "-p",
        &p,
        "--timeout",
        "0.5",
        "--datagram-size",
        "1452",
        "rx",
    ]);
    let out = bin()
        .args(["-n", "1", "-p", &p, "--datagram-size", "8000", "tx"])
        .arg(&path)
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    std::fs::remove_file(&path).ok();
    assert_eq!(out.status.code(), Some(0));
    let (output, _) = rx.read_to_end();
    assert!(output.is_empty(), "delivered {} bytes", output.len());
    let (code, stderr) = rx.wait(Duration::from_secs(5));
    assert_eq!(code, Some(0), "{stderr}");
    assert!(rx_summary(&stderr).contains("lost 3 datagrams"), "{stderr}");
}
