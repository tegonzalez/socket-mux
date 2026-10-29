use std::io;
use std::net::SocketAddr;
use std::sync::Once;

use log::{debug, info, warn};
use socket2::{Domain, Protocol, Socket, Type};

#[derive(Clone, Copy)]
pub enum BufferDirection {
    Send,
    Receive,
}

/// A blocking std UDP socket: TX lane threads send on it and RX lane threads
/// receive on it; neither side polls.
pub fn bind_blocking(
    address: SocketAddr,
    buffer_size: Option<usize>,
    direction: BufferDirection,
) -> io::Result<std::net::UdpSocket> {
    bind_inner(address, buffer_size, direction)
}

fn bind_inner(
    address: SocketAddr,
    buffer_size: Option<usize>,
    direction: BufferDirection,
) -> io::Result<std::net::UdpSocket> {
    let domain = if address.is_ipv4() {
        Domain::IPV4
    } else {
        Domain::IPV6
    };
    let socket = Socket::new(domain, Type::DGRAM, Some(Protocol::UDP))?;
    if let Some(size) = buffer_size {
        // SM-D15: apply --udp-buffer independently to every UDP socket.
        match direction {
            BufferDirection::Send => socket.set_send_buffer_size(size)?,
            BufferDirection::Receive => socket.set_recv_buffer_size(size)?,
        }
    }
    socket.bind(&address.into())?;
    disable_connection_reset(&socket)?;
    log_buffer_size(&socket, buffer_size, direction);
    Ok(socket.into())
}

fn buffer_size_status(requested: Option<usize>, reported: usize) -> (usize, bool) {
    // Linux includes bookkeeping overhead equal to the granted buffer size.
    #[cfg(target_os = "linux")]
    let effective = reported / 2;
    #[cfg(not(target_os = "linux"))]
    let effective = reported;

    (effective, requested.is_some_and(|size| effective < size))
}

fn log_buffer_size(socket: &Socket, requested: Option<usize>, direction: BufferDirection) {
    static SEND: Once = Once::new();
    static RECEIVE: Once = Once::new();

    let once = match direction {
        BufferDirection::Send => &SEND,
        BufferDirection::Receive => &RECEIVE,
    };
    once.call_once(|| {
        let reported = match direction {
            BufferDirection::Send => socket.send_buffer_size(),
            BufferDirection::Receive => socket.recv_buffer_size(),
        };
        let reported = match reported {
            Ok(size) => size,
            Err(error) => {
                debug!("could not read UDP buffer size: {error}");
                return;
            }
        };
        let (effective, capped) = buffer_size_status(requested, reported);
        if let Some(requested) = requested {
            info!("udp buffer requested={requested} effective={effective}");
            if capped {
                #[cfg(target_os = "linux")]
                {
                    let limit = match direction {
                        BufferDirection::Send => "net.core.wmem_max",
                        BufferDirection::Receive => "net.core.rmem_max",
                    };
                    warn!("OS capped UDP buffer: requested={requested} effective={effective}; check sysctl {limit}");
                }
                #[cfg(not(target_os = "linux"))]
                warn!("OS capped UDP buffer: requested={requested} effective={effective}");
            }
        } else {
            debug!("udp buffer OS default effective={effective}");
        }
    });
}

#[cfg(windows)]
fn disable_connection_reset(socket: &Socket) -> io::Result<()> {
    use std::ffi::c_void;
    use std::mem::size_of;
    use std::os::windows::io::AsRawSocket;
    use std::ptr;

    use windows_sys::Win32::Networking::WinSock::{WSAIoctl, SIO_UDP_CONNRESET, SOCKET};

    // SM-D8: disable SIO_UDP_CONNRESET on every Windows UDP socket.
    let mut enabled = 0_i32;
    let mut bytes_returned = 0_u32;
    let result = unsafe {
        WSAIoctl(
            socket.as_raw_socket() as SOCKET,
            SIO_UDP_CONNRESET,
            &mut enabled as *mut i32 as *mut c_void,
            size_of::<i32>() as u32,
            ptr::null_mut(),
            0,
            &mut bytes_returned,
            ptr::null_mut(),
            None,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(not(windows))]
fn disable_connection_reset(_socket: &Socket) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::buffer_size_status;

    #[test]
    fn buffer_cap_decision_uses_the_granted_size() {
        #[cfg(target_os = "linux")]
        let reported = 8_388_608;
        #[cfg(not(target_os = "linux"))]
        let reported = 4_194_304;

        assert_eq!(
            buffer_size_status(Some(8_388_608), reported),
            (4_194_304, true)
        );
        assert_eq!(
            buffer_size_status(Some(4_194_304), reported),
            (4_194_304, false)
        );
        assert_eq!(
            buffer_size_status(Some(2_097_152), reported),
            (4_194_304, false)
        );
        assert_eq!(buffer_size_status(None, reported), (4_194_304, false));
    }
}
