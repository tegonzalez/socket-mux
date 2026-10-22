//! Black-box helpers: drive the built `socket-mux` binary over real sockets.

#![allow(dead_code)]

use std::io::Read;
use std::net::{TcpListener, TcpStream, UdpSocket};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU16, Ordering};
use std::time::{Duration, Instant};

pub const N: u16 = 3;

pub fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_socket-mux"))
}

/// A base port whose N UDP ports are free right now. Each call starts its search
/// at a different offset so parallel tests do not collide.
pub fn free_base() -> u16 {
    static NEXT: AtomicU16 = AtomicU16::new(0);
    let start = 42000 + (std::process::id() % 500) as u16 * 16;
    loop {
        let base = start + NEXT.fetch_add(N, Ordering::Relaxed);
        let all_free = (0..N).all(|k| UdpSocket::bind(("127.0.0.1", base + k)).is_ok());
        if all_free {
            return base;
        }
    }
}

/// One datagram: the PROPOSAL SM-D10 header (big-endian magic, version, flags,
/// payload length, sequence, stream offset) followed by the payload.
pub fn datagram(seq: u64, offset: u64, payload: &[u8], eos: bool) -> Vec<u8> {
    let mut d = Vec::with_capacity(22 + payload.len());
    d.extend_from_slice(&0x534D_u16.to_be_bytes());
    d.push(1);
    d.push(u8::from(eos));
    d.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    d.extend_from_slice(&seq.to_be_bytes());
    d.extend_from_slice(&offset.to_be_bytes());
    d.extend_from_slice(payload);
    d
}

/// Sends sequence `seq` to its round-robin leg (seq % N).
pub fn send(base: u16, seq: u64, offset: u64, payload: &[u8], eos: bool) {
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    let leg = base + (seq % u64::from(N)) as u16;
    socket
        .send_to(&datagram(seq, offset, payload, eos), ("127.0.0.1", leg))
        .unwrap();
}

/// A running RX with its TCP output accepted.
pub struct Rx {
    pub child: Child,
    pub output: TcpStream,
}

/// Starts RX on `base` with `args` (global and RX options, any order the CLI
/// accepts) and accepts its TCP output connection.
pub fn start_rx(args: &[&str]) -> Rx {
    let sink = TcpListener::bind("127.0.0.1:0").unwrap();
    let connect = sink.local_addr().unwrap().to_string();
    let mut command = bin();
    command
        .args(args)
        .args(["--connect", &connect])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    let mut child = command.spawn().unwrap();
    let output = accept_from(&sink, &mut child);
    output
        .set_read_timeout(Some(Duration::from_secs(20)))
        .unwrap();
    // RX binds its UDP ports before it connects out, so datagrams sent now arrive.
    Rx { child, output }
}

/// Accepts RX's output connection, failing fast if RX exits or never connects.
fn accept_from(sink: &TcpListener, child: &mut Child) -> TcpStream {
    sink.set_nonblocking(true).unwrap();
    let end = Instant::now() + Duration::from_secs(10);
    loop {
        match sink.accept() {
            Ok((stream, _)) => {
                stream.set_nonblocking(false).unwrap();
                return stream;
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(error) => panic!("accept failed: {error}"),
        }
        if let Some(status) = child.try_wait().unwrap() {
            let mut err = String::new();
            child
                .stderr
                .take()
                .unwrap()
                .read_to_string(&mut err)
                .unwrap();
            panic!("RX exited with {status} before connecting: {err}");
        }
        assert!(
            Instant::now() < end,
            "RX did not connect its output within 10 s"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

impl Rx {
    /// Reads RX's output to end of stream; returns the bytes and when it closed.
    pub fn read_to_end(&mut self) -> (Vec<u8>, Instant) {
        let mut bytes = Vec::new();
        self.output.read_to_end(&mut bytes).unwrap();
        (bytes, Instant::now())
    }

    /// Waits up to `limit` for RX to exit; returns its status code and stderr.
    pub fn wait(mut self, limit: Duration) -> (Option<i32>, String) {
        let end = Instant::now() + limit;
        while Instant::now() < end {
            if let Some(status) = self.child.try_wait().unwrap() {
                let mut err = String::new();
                self.child
                    .stderr
                    .take()
                    .unwrap()
                    .read_to_string(&mut err)
                    .unwrap();
                return (status.code(), err);
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        self.child.kill().ok();
        panic!("RX still running after {limit:?}");
    }
}

impl Drop for Rx {
    fn drop(&mut self) {
        self.child.kill().ok();
        self.child.wait().ok();
    }
}

/// CPU time (user + system, all threads) of a process in clock ticks.
#[cfg(target_os = "linux")]
pub fn cpu_ticks(pid: u32) -> u64 {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).unwrap();
    let fields: Vec<&str> = stat[stat.rfind(')').unwrap() + 2..].split(' ').collect();
    fields[11].parse::<u64>().unwrap() + fields[12].parse::<u64>().unwrap()
}
