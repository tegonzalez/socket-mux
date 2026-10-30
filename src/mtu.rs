//! Default datagram size from the network adapter MTU.
//!
//! With no `--datagram-size`, the size is the relevant adapter's MTU less 48
//! (a 40-byte IPv6 header and an 8-byte UDP header), clamped to the valid
//! range, so a datagram fits one IP packet. TX uses the adapter that routes to
//! the destination; RX uses the adapter owning its bind address, or for a
//! wildcard bind the largest MTU among up, non-loopback adapters (loopback
//! only when nothing else exists). When the MTU cannot be read the size falls
//! back to 1452. An explicit `--datagram-size` always wins; if it exceeds the
//! adapter MTU less 48, one warning says the datagrams will be IP-fragmented.
//!
//! The selection and size logic is pure (it takes an adapter list) so tests
//! drive it with fake adapters; [`sys::adapters`] reads the real list:
//! `getifaddrs` + `ioctl(SIOCGIFMTU)` on Linux, Android and Apple platforms,
//! `GetAdaptersAddresses` on Windows.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs, UdpSocket};

use log::{debug, info, warn};

use crate::cli::{Global, UsageError};
use crate::header::HEADER_LEN;

/// IPv6 header (40) + UDP header (8): what a datagram adds to reach the MTU.
pub const IP_UDP_OVERHEAD: u32 = 48;
/// Datagram size used when the adapter MTU is unknown.
pub const FALLBACK_DATAGRAM_SIZE: usize = 1452;
/// Smallest valid datagram size: the header and one payload byte.
pub const MIN_DATAGRAM_SIZE: usize = HEADER_LEN + 1;
/// Largest valid datagram size: the largest IPv4 UDP payload.
pub const MAX_DATAGRAM_SIZE: usize = 65_507;

/// One network adapter as the selection logic sees it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Adapter {
    pub name: String,
    pub addresses: Vec<IpAddr>,
    /// None when the MTU could not be read for this adapter.
    pub mtu: Option<u32>,
    pub up: bool,
    pub loopback: bool,
}

/// The adapter whose MTU governs the datagram size.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdapterMtu {
    pub name: String,
    pub mtu: u32,
}

/// Where the datagrams go: TX sends to a remote host, RX binds an address.
#[derive(Clone, Debug)]
pub enum Path<'a> {
    /// TX: the remote host (routed as for the first lane's port).
    Destination(&'a str),
    /// RX: the bind address (possibly a wildcard).
    Bind(IpAddr),
}

/// The effective datagram size and the log lines that explain it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Choice {
    pub size: usize,
    /// `info`: the size and its source.
    pub info: String,
    /// `debug`: why the MTU is unknown, if it is.
    pub debug: Option<String>,
    /// `warn`: the size exceeds the adapter MTU less 48.
    pub warning: Option<String>,
}

/// The adapter that owns `address`.
pub fn owning(adapters: &[Adapter], address: IpAddr) -> Result<AdapterMtu, String> {
    let adapter = adapters
        .iter()
        .find(|adapter| adapter.addresses.contains(&address))
        .ok_or_else(|| format!("no adapter owns {address}"))?;
    with_mtu(adapter)
}

/// For a wildcard bind: the largest MTU among up, non-loopback adapters with
/// an address RX can receive on (IPv4 for 0.0.0.0, any for ::); loopback
/// adapters only if no other adapter qualifies.
pub fn widest(adapters: &[Adapter], wildcard: IpAddr) -> Result<AdapterMtu, String> {
    let receives = |adapter: &&Adapter| {
        adapter.up
            && adapter.mtu.is_some()
            && adapter
                .addresses
                .iter()
                .any(|address| wildcard.is_ipv6() || address.is_ipv4())
    };
    let pick = |loopback: bool| {
        adapters
            .iter()
            .filter(receives)
            .filter(|adapter| adapter.loopback == loopback)
            // Largest MTU; the first listed wins a tie.
            .fold(None::<&Adapter>, |best, adapter| match best {
                Some(best) if best.mtu >= adapter.mtu => Some(best),
                _ => Some(adapter),
            })
    };
    match pick(false).or_else(|| pick(true)) {
        Some(adapter) => with_mtu(adapter),
        None => Err(format!("no up adapter with a readable MTU for {wildcard}")),
    }
}

/// RX: the adapter for bind address `bind`.
pub fn for_bind(adapters: &[Adapter], bind: IpAddr) -> Result<AdapterMtu, String> {
    if bind.is_unspecified() {
        widest(adapters, bind)
    } else {
        owning(adapters, bind)
    }
}

fn with_mtu(adapter: &Adapter) -> Result<AdapterMtu, String> {
    match adapter.mtu {
        Some(mtu) => Ok(AdapterMtu {
            name: adapter.name.clone(),
            mtu,
        }),
        None => Err(format!("could not read the MTU of {}", adapter.name)),
    }
}

/// The datagram size for adapter MTU `mtu`: MTU less 48, clamped to the
/// valid range. Also whether the clamp changed it.
pub fn size_for_mtu(mtu: u32) -> (usize, bool) {
    let fit = mtu.saturating_sub(IP_UDP_OVERHEAD) as usize;
    let size = fit.clamp(MIN_DATAGRAM_SIZE, MAX_DATAGRAM_SIZE);
    (size, size != fit)
}

/// Decide the datagram size from `--datagram-size` (if given) and the adapter
/// MTU (or why it is unknown).
pub fn choose(explicit: Option<usize>, adapter: Result<AdapterMtu, String>) -> Choice {
    let (adapter, debug) = match adapter {
        Ok(adapter) => (Some(adapter), None),
        Err(reason) => (None, Some(format!("adapter mtu unknown: {reason}"))),
    };
    let (size, info) = match (explicit, &adapter) {
        (Some(size), Some(a)) => (
            size,
            format!(
                "datagram size {size} (--datagram-size; {} mtu {})",
                a.name, a.mtu
            ),
        ),
        (Some(size), None) => (
            size,
            format!("datagram size {size} (--datagram-size; mtu unknown)"),
        ),
        (None, Some(a)) => {
            let (size, clamped) = size_for_mtu(a.mtu);
            let clamped = if clamped { "; clamped" } else { "" };
            (
                size,
                format!("datagram size {size} ({} mtu {}{clamped})", a.name, a.mtu),
            )
        }
        (None, None) => (
            FALLBACK_DATAGRAM_SIZE,
            format!("datagram size {FALLBACK_DATAGRAM_SIZE} (default; mtu unknown)"),
        ),
    };
    let warning = adapter.and_then(|a| {
        let fit = a.mtu.saturating_sub(IP_UDP_OVERHEAD) as usize;
        (size > fit).then(|| {
            format!(
                "datagram size {size} exceeds {} mtu {} less {IP_UDP_OVERHEAD}; datagrams will be \
                 IP-fragmented and a lost fragment loses the datagram",
                a.name, a.mtu
            )
        })
    });
    Choice {
        size,
        info,
        debug,
        warning,
    }
}

/// Find the adapter MTU for `path`, choose the datagram size, log it once and
/// store it (with the hold capacity it gives) in `global`. A size that gives
/// an invalid hold capacity is a usage error.
pub fn resolve(global: &mut Global, path: Path<'_>) -> Result<(), UsageError> {
    let adapter = adapter_for(path, global.base_port);
    let choice = choose(global.requested_datagram_size, adapter);
    if let Some(reason) = &choice.debug {
        debug!(target: "socket_mux", "{reason}");
    }
    info!(target: "socket_mux", "{}", choice.info);
    if let Some(warning) = &choice.warning {
        warn!(target: "socket_mux", "{warning}");
    }
    global.set_datagram_size(choice.size)
}

fn adapter_for(path: Path<'_>, port: u16) -> Result<AdapterMtu, String> {
    match path {
        Path::Bind(bind) => for_bind(&sys::adapters()?, bind),
        Path::Destination(remote) => {
            let local = route_source(remote, port)?;
            owning(&sys::adapters()?, local)
        }
    }
}

/// The local address the OS routes `remote` from: connect a UDP socket (which
/// sends nothing) and read its local address.
fn route_source(remote: &str, port: u16) -> Result<IpAddr, String> {
    // SM-D15: the first resolved address, as the TX lanes use.
    let destination: SocketAddr = (remote, port)
        .to_socket_addrs()
        .map_err(|error| format!("could not resolve {remote}: {error}"))?
        .next()
        .ok_or_else(|| format!("{remote} resolved to no addresses"))?;
    let any: SocketAddr = match destination {
        SocketAddr::V4(_) => (Ipv4Addr::UNSPECIFIED, 0).into(),
        SocketAddr::V6(_) => (Ipv6Addr::UNSPECIFIED, 0).into(),
    };
    let probe = UdpSocket::bind(any).map_err(|error| format!("route probe: {error}"))?;
    probe
        .connect(destination)
        .map_err(|error| format!("no route to {destination}: {error}"))?;
    let local = probe
        .local_addr()
        .map_err(|error| format!("route probe: {error}"))?
        .ip();
    // An IPv4-mapped source belongs to the adapter's IPv4 address.
    Ok(match local {
        IpAddr::V6(v6) => v6.to_ipv4_mapped().map_or(local, IpAddr::V4),
        IpAddr::V4(_) => local,
    })
}

#[cfg(any(
    target_os = "linux",
    target_os = "android",
    target_os = "macos",
    target_os = "ios"
))]
mod sys {
    //! `getifaddrs` for names, flags and addresses; `ioctl(SIOCGIFMTU)` for
    //! each adapter's MTU.

    use std::ffi::CStr;
    use std::io;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    use super::Adapter;

    pub fn adapters() -> Result<Vec<Adapter>, String> {
        let mut head: *mut libc::ifaddrs = std::ptr::null_mut();
        // SAFETY: getifaddrs fills `head` with a list we free below.
        if unsafe { libc::getifaddrs(&mut head) } != 0 {
            return Err(format!("getifaddrs: {}", io::Error::last_os_error()));
        }
        let mut adapters: Vec<Adapter> = Vec::new();
        let mut entry = head;
        while !entry.is_null() {
            // SAFETY: a node of the list getifaddrs returned, not yet freed.
            let ifa = unsafe { &*entry };
            entry = ifa.ifa_next;
            if ifa.ifa_name.is_null() {
                continue;
            }
            // SAFETY: ifa_name is a NUL-terminated interface name.
            let name = unsafe { CStr::from_ptr(ifa.ifa_name) }
                .to_string_lossy()
                .into_owned();
            let flags = ifa.ifa_flags as libc::c_int;
            // SAFETY: ifa_addr is null or a sockaddr of its family.
            let address = unsafe { ip_of(ifa.ifa_addr) };
            let index = match adapters.iter().position(|a| a.name == name) {
                Some(index) => index,
                None => {
                    adapters.push(Adapter {
                        mtu: None,
                        up: flags & libc::IFF_UP != 0,
                        loopback: flags & libc::IFF_LOOPBACK != 0,
                        addresses: Vec::new(),
                        name,
                    });
                    adapters.len() - 1
                }
            };
            if let Some(address) = address {
                adapters[index].addresses.push(address);
            }
        }
        // SAFETY: `head` came from getifaddrs and is freed once.
        unsafe { libc::freeifaddrs(head) };
        for adapter in &mut adapters {
            adapter.mtu = mtu(&adapter.name);
        }
        Ok(adapters)
    }

    unsafe fn ip_of(address: *const libc::sockaddr) -> Option<IpAddr> {
        if address.is_null() {
            return None;
        }
        match i32::from((*address).sa_family) {
            libc::AF_INET => {
                let v4 = &*(address as *const libc::sockaddr_in);
                Some(IpAddr::V4(Ipv4Addr::from(u32::from_be(v4.sin_addr.s_addr))))
            }
            libc::AF_INET6 => {
                let v6 = &*(address as *const libc::sockaddr_in6);
                Some(IpAddr::V6(Ipv6Addr::from(v6.sin6_addr.s6_addr)))
            }
            _ => None,
        }
    }

    /// The MTU of adapter `name` by `ioctl(SIOCGIFMTU)`; None if unreadable.
    fn mtu(name: &str) -> Option<u32> {
        // SAFETY: plain socket(2) calls; the descriptor is closed below.
        let fd = unsafe {
            match libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 0) {
                -1 => libc::socket(libc::AF_INET6, libc::SOCK_DGRAM, 0),
                fd => fd,
            }
        };
        if fd < 0 {
            return None;
        }
        // SAFETY: an all-zero ifreq is valid; ifr_name is NUL-terminated.
        let mut request: libc::ifreq = unsafe { std::mem::zeroed() };
        let bytes = name.as_bytes();
        if bytes.len() >= request.ifr_name.len() {
            unsafe { libc::close(fd) };
            return None;
        }
        for (slot, byte) in request.ifr_name.iter_mut().zip(bytes) {
            *slot = *byte as libc::c_char;
        }
        // SAFETY: SIOCGIFMTU reads ifr_name and writes ifr_ifru.ifru_mtu.
        let result = unsafe { libc::ioctl(fd, libc::SIOCGIFMTU as _, &mut request) };
        // SAFETY: closes the descriptor opened above.
        unsafe { libc::close(fd) };
        if result != 0 {
            return None;
        }
        // SAFETY: a successful SIOCGIFMTU set the mtu member.
        let mtu = unsafe { request.ifr_ifru.ifru_mtu };
        u32::try_from(mtu).ok().filter(|&mtu| mtu > 0)
    }
}

#[cfg(windows)]
mod sys {
    //! `GetAdaptersAddresses`: friendly name, MTU, operational status, type
    //! and unicast addresses of every adapter.

    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    use windows_sys::Win32::NetworkManagement::IpHelper::{
        GetAdaptersAddresses, GAA_FLAG_SKIP_ANYCAST, GAA_FLAG_SKIP_DNS_SERVER,
        GAA_FLAG_SKIP_MULTICAST, IF_TYPE_SOFTWARE_LOOPBACK, IP_ADAPTER_ADDRESSES_LH,
    };
    use windows_sys::Win32::NetworkManagement::Ndis::IfOperStatusUp;
    use windows_sys::Win32::Networking::WinSock::{
        AF_INET, AF_INET6, AF_UNSPEC, SOCKADDR, SOCKADDR_IN, SOCKADDR_IN6,
    };

    use super::Adapter;

    const ERROR_SUCCESS: u32 = 0;
    const ERROR_BUFFER_OVERFLOW: u32 = 111;

    pub fn adapters() -> Result<Vec<Adapter>, String> {
        let flags = GAA_FLAG_SKIP_ANYCAST | GAA_FLAG_SKIP_MULTICAST | GAA_FLAG_SKIP_DNS_SERVER;
        let mut size: u32 = 16 * 1024;
        // u64 storage keeps the adapter records aligned.
        let mut buffer: Vec<u64>;
        let mut attempts = 0;
        loop {
            buffer = vec![0_u64; (size as usize).div_ceil(8)];
            // SAFETY: `buffer` holds `size` writable, aligned bytes.
            let result = unsafe {
                GetAdaptersAddresses(
                    u32::from(AF_UNSPEC),
                    flags,
                    std::ptr::null(),
                    buffer.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH,
                    &mut size,
                )
            };
            match result {
                ERROR_SUCCESS => break,
                ERROR_BUFFER_OVERFLOW if attempts < 3 => attempts += 1,
                error => return Err(format!("GetAdaptersAddresses failed: error {error}")),
            }
        }
        let mut adapters = Vec::new();
        let mut entry = buffer.as_ptr() as *const IP_ADAPTER_ADDRESSES_LH;
        while !entry.is_null() {
            // SAFETY: a record of the list GetAdaptersAddresses wrote to `buffer`.
            let record = unsafe { &*entry };
            entry = record.Next;
            let mut addresses = Vec::new();
            let mut unicast = record.FirstUnicastAddress;
            while !unicast.is_null() {
                // SAFETY: a unicast record of the same list.
                let address = unsafe { &*unicast };
                unicast = address.Next;
                // SAFETY: lpSockaddr is null or a sockaddr of its family.
                if let Some(ip) = unsafe { ip_of(address.Address.lpSockaddr) } {
                    addresses.push(ip);
                }
            }
            adapters.push(Adapter {
                // SAFETY: FriendlyName is null or a NUL-terminated UTF-16 string.
                name: unsafe { wide(record.FriendlyName) },
                addresses,
                mtu: (record.Mtu > 0).then_some(record.Mtu),
                up: record.OperStatus == IfOperStatusUp,
                loopback: record.IfType == IF_TYPE_SOFTWARE_LOOPBACK,
            });
        }
        Ok(adapters)
    }

    unsafe fn ip_of(address: *const SOCKADDR) -> Option<IpAddr> {
        if address.is_null() {
            return None;
        }
        match (*address).sa_family {
            AF_INET => {
                let v4 = &*(address as *const SOCKADDR_IN);
                Some(IpAddr::V4(Ipv4Addr::from(u32::from_be(
                    v4.sin_addr.S_un.S_addr,
                ))))
            }
            AF_INET6 => {
                let v6 = &*(address as *const SOCKADDR_IN6);
                Some(IpAddr::V6(Ipv6Addr::from(v6.sin6_addr.u.Byte)))
            }
            _ => None,
        }
    }

    unsafe fn wide(text: *const u16) -> String {
        if text.is_null() {
            return String::new();
        }
        let mut len = 0;
        while *text.add(len) != 0 {
            len += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(text, len))
    }
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "android",
    target_os = "macos",
    target_os = "ios",
    windows
)))]
mod sys {
    use super::Adapter;

    pub fn adapters() -> Result<Vec<Adapter>, String> {
        Err("reading adapter MTUs is not supported on this platform".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn adapter(
        name: &str,
        addresses: &[&str],
        mtu: Option<u32>,
        up: bool,
        loopback: bool,
    ) -> Adapter {
        Adapter {
            name: name.into(),
            addresses: addresses.iter().map(|a| a.parse().unwrap()).collect(),
            mtu,
            up,
            loopback,
        }
    }

    fn found(name: &str, mtu: u32) -> Result<AdapterMtu, String> {
        Ok(AdapterMtu {
            name: name.into(),
            mtu,
        })
    }

    fn host() -> Vec<Adapter> {
        vec![
            adapter("lo", &["127.0.0.1", "::1"], Some(65536), true, true),
            adapter(
                "eth0",
                &["192.0.2.10", "2001:db8::10"],
                Some(1500),
                true,
                false,
            ),
            adapter("jumbo", &["198.51.100.7"], Some(9000), true, false),
            adapter("down", &["203.0.113.1"], Some(16000), false, false),
            adapter("v6only", &["2001:db8::20"], Some(9216), true, false),
            adapter("nomtu", &["10.0.0.1"], None, true, false),
        ]
    }

    #[test]
    fn size_is_mtu_less_48_clamped_to_the_valid_range() {
        assert_eq!(size_for_mtu(1500), (1452, false));
        assert_eq!(size_for_mtu(9000), (8952, false));
        assert_eq!(size_for_mtu(1280), (1232, false));
        // Linux loopback: 65536 - 48 fits.
        assert_eq!(size_for_mtu(65536), (65488, false));
        // Larger MTUs (Windows loopback reports u32::MAX) clamp to 65507.
        assert_eq!(size_for_mtu(65556), (65507, true));
        assert_eq!(size_for_mtu(u32::MAX), (65507, true));
        // Tiny MTUs clamp up to header + 1.
        assert_eq!(size_for_mtu(68), (MIN_DATAGRAM_SIZE, true));
        assert_eq!(size_for_mtu(0), (MIN_DATAGRAM_SIZE, true));
        assert_eq!(size_for_mtu(73), (25, false));
    }

    #[test]
    fn a_bind_or_route_address_selects_its_owning_adapter() {
        let host = host();
        assert_eq!(
            for_bind(&host, "192.0.2.10".parse().unwrap()),
            found("eth0", 1500)
        );
        assert_eq!(
            for_bind(&host, "2001:db8::10".parse().unwrap()),
            found("eth0", 1500)
        );
        assert_eq!(
            for_bind(&host, "127.0.0.1".parse().unwrap()),
            found("lo", 65536)
        );
        assert_eq!(
            owning(&host, "198.51.100.7".parse().unwrap()),
            found("jumbo", 9000)
        );
        assert!(for_bind(&host, "192.0.2.99".parse().unwrap()).is_err());
        assert!(owning(&host, "10.0.0.1".parse().unwrap()).is_err());
    }

    #[test]
    fn a_wildcard_bind_selects_the_largest_up_non_loopback_mtu() {
        let host = host();
        // Down adapters (16000) and loopback (65536) do not count.
        assert_eq!(
            for_bind(&host, "0.0.0.0".parse().unwrap()),
            found("jumbo", 9000)
        );
        // :: also receives on IPv6-only adapters.
        assert_eq!(
            for_bind(&host, "::".parse().unwrap()),
            found("v6only", 9216)
        );
        // A tie keeps the first adapter listed.
        let tie = vec![
            adapter("a", &["192.0.2.1"], Some(1500), true, false),
            adapter("b", &["192.0.2.2"], Some(1500), true, false),
        ];
        assert_eq!(widest(&tie, "0.0.0.0".parse().unwrap()), found("a", 1500));
    }

    #[test]
    fn a_wildcard_bind_falls_back_to_loopback_then_to_unknown() {
        let only_loopback = vec![
            adapter("lo", &["127.0.0.1"], Some(65536), true, true),
            adapter("down", &["192.0.2.1"], Some(1500), false, false),
        ];
        assert_eq!(
            widest(&only_loopback, "0.0.0.0".parse().unwrap()),
            found("lo", 65536)
        );
        let nothing = vec![adapter("nomtu", &["192.0.2.1"], None, true, false)];
        assert!(widest(&nothing, "0.0.0.0".parse().unwrap()).is_err());
        assert!(widest(&[], "::".parse().unwrap()).is_err());
    }

    #[test]
    fn the_default_size_comes_from_the_adapter_or_falls_back_to_1452() {
        let choice = choose(None, found("enp191s0", 1500));
        assert_eq!(choice.size, 1452);
        assert_eq!(choice.info, "datagram size 1452 (enp191s0 mtu 1500)");
        assert_eq!((choice.debug, choice.warning), (None, None));

        let choice = choose(None, found("lo", 65536));
        assert_eq!(choice.size, 65488);
        assert_eq!(choice.warning, None);

        let choice = choose(None, found("Loopback Pseudo-Interface 1", u32::MAX));
        assert_eq!(choice.size, 65507);
        assert_eq!(
            choice.info,
            "datagram size 65507 (Loopback Pseudo-Interface 1 mtu 4294967295; clamped)"
        );
        assert_eq!(choice.warning, None);

        let choice = choose(None, Err("no adapter owns 192.0.2.99".into()));
        assert_eq!(choice.size, FALLBACK_DATAGRAM_SIZE);
        assert_eq!(choice.info, "datagram size 1452 (default; mtu unknown)");
        assert_eq!(
            choice.debug.as_deref(),
            Some("adapter mtu unknown: no adapter owns 192.0.2.99")
        );
        assert_eq!(choice.warning, None);
    }

    #[test]
    fn an_explicit_size_wins_and_warns_once_above_mtu_less_48() {
        let choice = choose(Some(8972), found("enp191s0", 1500));
        assert_eq!(choice.size, 8972);
        assert_eq!(
            choice.info,
            "datagram size 8972 (--datagram-size; enp191s0 mtu 1500)"
        );
        assert_eq!(
            choice.warning.as_deref(),
            Some(
                "datagram size 8972 exceeds enp191s0 mtu 1500 less 48; datagrams will be \
                 IP-fragmented and a lost fragment loses the datagram"
            )
        );
        // Exactly MTU - 48 fits; one more byte warns.
        assert_eq!(choose(Some(1452), found("eth0", 1500)).warning, None);
        assert!(choose(Some(1453), found("eth0", 1500)).warning.is_some());
        // Loopback: only sizes above 65488 warn.
        assert!(choose(Some(65507), found("lo", 65536)).warning.is_some());
        assert_eq!(choose(Some(65488), found("lo", 65536)).warning, None);
        // A derived size clamped up to the minimum still exceeds a tiny MTU.
        assert!(choose(None, found("tiny", 68)).warning.is_some());
    }

    #[test]
    fn no_warning_when_the_mtu_is_unknown() {
        let choice = choose(Some(65507), Err("getifaddrs: denied".into()));
        assert_eq!(choice.size, 65507);
        assert_eq!(
            choice.info,
            "datagram size 65507 (--datagram-size; mtu unknown)"
        );
        assert_eq!(choice.warning, None);
        assert!(choice.debug.is_some());
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn the_host_lists_a_loopback_adapter_with_an_mtu() {
        let adapters = sys::adapters().unwrap();
        let lo = owning(&adapters, "127.0.0.1".parse().unwrap()).unwrap();
        assert!(lo.mtu >= 1280, "{lo:?}");
        let route = route_source("127.0.0.1", 9).unwrap();
        assert_eq!(owning(&adapters, route), Ok(lo));
    }
}
