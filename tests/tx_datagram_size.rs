mod common;

use common::bin;
use std::io::Write;
use std::net::UdpSocket;
use std::process::Stdio;
use std::time::Duration;

fn receive(socket: &UdpSocket, buffer: &mut [u8]) -> (usize, Vec<u8>) {
    let (length, _) = socket.recv_from(buffer).unwrap();
    (length, buffer[..length].to_vec())
}

#[test]
fn tx_uses_configured_datagram_size_and_emits_session_on_every_header() {
    let base = common::free_base();
    let receiver = UdpSocket::bind(("127.0.0.1", base)).unwrap();
    receiver
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let base_string = base.to_string();
    let mut child = bin()
        .args([
            "--lanes",
            "1",
            "--base-port",
            &base_string,
            "--datagram-size",
            "128",
            "--rx-memory",
            "8192",
            "tx",
            "-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let input: Vec<u8> = (0..220).map(|byte| byte as u8).collect();
    child.stdin.take().unwrap().write_all(&input).unwrap();

    let mut buffer = vec![0_u8; 128];
    let mut received = Vec::new();
    let mut session = None;
    for sequence in 0..3_u64 {
        let (length, datagram) = receive(&receiver, &mut buffer);
        assert_eq!(length, if sequence < 2 { 128 } else { 36 });
        assert_eq!(
            u16::from_be_bytes(datagram[4..6].try_into().unwrap()),
            if sequence < 2 { 104 } else { 12 }
        );
        assert_eq!(
            u64::from_be_bytes(datagram[8..16].try_into().unwrap()),
            sequence
        );
        assert_eq!(
            u64::from_be_bytes(datagram[16..24].try_into().unwrap()),
            sequence * 104
        );
        let packet_session = u16::from_be_bytes(datagram[6..8].try_into().unwrap());
        assert_eq!(session.get_or_insert(packet_session), &packet_session);
        received.extend_from_slice(&datagram[24..]);
    }
    let (length, eos) = receive(&receiver, &mut buffer);
    assert_eq!(length, 24);
    assert_eq!(eos[3] & 1, 1);
    assert_eq!(
        u16::from_be_bytes(eos[6..8].try_into().unwrap()),
        session.unwrap()
    );
    assert_eq!(u64::from_be_bytes(eos[8..16].try_into().unwrap()), 3);
    assert_eq!(received, input);
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(&format!("session={}", session.unwrap()))
    );
}

#[test]
fn tx_pat_uses_configured_datagram_size() {
    let base = common::free_base();
    let receiver = UdpSocket::bind(("127.0.0.1", base)).unwrap();
    receiver
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let base_string = base.to_string();
    let mut child = bin()
        .args([
            "--lanes",
            "1",
            "--base-port",
            &base_string,
            "--datagram-size",
            "128",
            "--rx-memory",
            "8192",
            "tx-pat",
            "--seed",
            "4",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut buffer = vec![0_u8; 128];
    let (length, datagram) = receive(&receiver, &mut buffer);
    assert_eq!(length, 128);
    assert_eq!(u16::from_be_bytes(datagram[4..6].try_into().unwrap()), 104);
    assert_eq!(u64::from_be_bytes(datagram[8..16].try_into().unwrap()), 0);
    child.kill().unwrap();
    let output = child.wait_with_output().unwrap();
    let session = u16::from_be_bytes(datagram[6..8].try_into().unwrap());
    assert!(String::from_utf8_lossy(&output.stderr).contains(&format!("session={session}")));
}
