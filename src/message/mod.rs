//! Messaging primitives for discovering devices and services.

use std::io;
use std::net::{Ipv6Addr, SocketAddr};

use crate::net::connector::UdpConnector;
use crate::net::IpVersionMode;

pub mod listen;
pub mod multicast;
mod notify;
mod search;
mod ssdp;

pub use listen::Listen;
pub use multicast::Multicast;
pub use notify::{NotifyListener, NotifyMessage};
pub use search::{SearchListener, SearchRequest, SearchResponse};

/// Multicast Socket Information
pub const UPNP_MULTICAST_IPV4_ADDR: &str = "239.255.255.250";
pub const UPNP_MULTICAST_IPV6_LINK_LOCAL_ADDR: &str = "FF02::C";
pub const UPNP_MULTICAST_PORT: u16 = 1900;

/// Default TTL For Multicast
pub const UPNP_MULTICAST_TTL: u32 = 2;

/// Enumerates different types of SSDP messages.
#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub enum MessageType {
    /// A notify message.
    Notify,
    /// A search message.
    Search,
    /// A response to a search message.
    Response,
}

#[derive(Clone)]
pub struct Config {
    pub ipv4_addr: String,
    pub ipv6_addr: String,
    pub port: u16,
    pub ttl: u32,
    pub mode: IpVersionMode,
}

impl Config {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn set_ipv4_addr<S: Into<String>>(mut self, value: S) -> Self {
        self.ipv4_addr = value.into();
        self
    }

    pub fn set_ipv6_addr<S: Into<String>>(mut self, value: S) -> Self {
        self.ipv6_addr = value.into();
        self
    }

    pub fn set_port(mut self, value: u16) -> Self {
        self.port = value;
        self
    }

    pub fn set_ttl(mut self, value: u32) -> Self {
        self.ttl = value;
        self
    }

    pub fn set_mode(mut self, value: IpVersionMode) -> Self {
        self.mode = value;
        self
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            ipv4_addr: UPNP_MULTICAST_IPV4_ADDR.to_string(),
            ipv6_addr: UPNP_MULTICAST_IPV6_LINK_LOCAL_ADDR.to_string(),
            port: UPNP_MULTICAST_PORT,
            ttl: UPNP_MULTICAST_TTL,
            mode: IpVersionMode::Any,
        }
    }
}

/// Generate `UdpConnector` objects for all local `IPv4` interfaces.
fn all_local_connectors(multicast_ttl: Option<u32>, filter: &IpVersionMode) -> io::Result<Vec<UdpConnector>> {
    log::trace!("Fetching all local connectors");
    map_local(|&addr| match (filter, addr) {
        (&IpVersionMode::V4Only, SocketAddr::V4(n)) | (&IpVersionMode::Any, SocketAddr::V4(n)) => {
            Ok(Some(UdpConnector::new((*n.ip(), 0), multicast_ttl)?))
        }
        (&IpVersionMode::V6Only, SocketAddr::V6(n)) | (&IpVersionMode::Any, SocketAddr::V6(n)) => {
            Ok(Some(UdpConnector::new(n, multicast_ttl)?))
        }
        _ => Ok(None),
    })
}

/// Invoke the closure for every local address found on the system
///
/// This method filters out _loopback_ and _global_ addresses.
fn map_local<F, R>(mut f: F) -> io::Result<Vec<R>>
where
    F: FnMut(&SocketAddr) -> io::Result<Option<R>>,
{
    let addrs_iter = get_local_addrs()?;

    let mut obj_list = Vec::with_capacity(addrs_iter.len());

    for addr in addrs_iter {
        log::trace!("Found {}", addr);
        match addr {
            SocketAddr::V4(n) if !n.ip().is_loopback() => {
                if let Some(x) = f(&addr)? {
                    obj_list.push(x);
                }
            }
            // Filter all loopback and global IPv6 addresses
            SocketAddr::V6(n) if is_not_globally(n.ip()) => {
                if let Some(x) = f(&addr)? {
                    obj_list.push(x);
                }
            }
            _ => (),
        }
    }

    Ok(obj_list)
}
fn is_not_globally(ip: &Ipv6Addr) -> bool {
    // Non-exhaustive list of notable addresses that are not globally reachable:
    // - The [unspecified address] ([`is_unspecified`](Ipv6Addr::is_unspecified))
    // - The [loopback address] ([`is_loopback`](Ipv6Addr::is_loopback))
    // - IPv4-mapped addresses
    // - Addresses reserved for benchmarking
    // - Addresses reserved for documentation ([`is_documentation`](Ipv6Addr::is_documentation))
    // - Unique local addresses ([`is_unique_local`](Ipv6Addr::is_unique_local))
    // - Unicast addresses with link-local scope ([`is_unicast_link_local`](Ipv6Addr::is_unicast_link_local))
    let segments = ip.segments();
    let is_unspecified = ip.is_unspecified();
    let is_loopback = ip.is_loopback();
    let is_benchmarking = (segments[0] == 0x2001) && (segments[1] == 0x2) && (segments[2] == 0);
    let is_documentation = (segments[0] == 0x2001) && (segments[1] == 0xdb8);
    let is_unique_local = (segments[0] & 0xfe00) == 0xfc00;
    let is_uniquest = (segments[0] & 0xffc0) == 0xfe80;

    is_unspecified || is_loopback || is_benchmarking || is_documentation || is_unique_local || is_uniquest
}

/// Generate a list of some object R constructed from all local `Ipv4Addr` objects.
///
/// If any of the `SocketAddr`'s fail to resolve, this function will not return an error.
fn get_local_addrs() -> io::Result<Vec<SocketAddr>> {
    let iface_iter = get_if_addrs::get_if_addrs()?.into_iter();
    Ok(iface_iter
        .map(|iface| SocketAddr::new(iface.addr.ip(), 0))
        .collect())
}
