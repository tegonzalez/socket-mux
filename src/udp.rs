use std::io;
use std::net::SocketAddr;

use socket2::{Domain, Protocol, Socket, Type};
use tokio::net::UdpSocket;

#[derive(Clone, Copy)]
pub enum BufferDirection {
    Send,
    Receive,
}

pub fn bind(
    address: SocketAddr,
    buffer_size: Option<usize>,
    direction: BufferDirection,
) -> io::Result<UdpSocket> {
    let socket = bind_inner(address, buffer_size, direction)?;
    socket.set_nonblocking(true)?;
    UdpSocket::from_std(socket)
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
        // PROPOSAL SM-D15: apply --udp-buffer independently to every UDP socket.
        match direction {
            BufferDirection::Send => socket.set_send_buffer_size(size)?,
            BufferDirection::Receive => socket.set_recv_buffer_size(size)?,
        }
    }
    socket.bind(&address.into())?;
    disable_connection_reset(&socket)?;
    Ok(socket.into())
}

#[cfg(windows)]
fn disable_connection_reset(socket: &Socket) -> io::Result<()> {
    use std::ffi::c_void;
    use std::mem::size_of;
    use std::os::windows::io::AsRawSocket;
    use std::ptr;

    use windows_sys::Win32::Networking::WinSock::{WSAIoctl, SIO_UDP_CONNRESET, SOCKET};

    // PROPOSAL SM-D8: disable SIO_UDP_CONNRESET on every Windows UDP socket.
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
