use std::net::{IpAddr, SocketAddr};

use log::debug;

use crate::error::SSDPResult;
use crate::message::{self, Config};
use crate::net;
use crate::receiver::{FromRawSSDP, SSDPReceiver};

pub trait Listen {
    type Message: FromRawSSDP + Send + 'static;

    /// Listen for messages on all local network interfaces.
    ///
    /// This will call `listen_with_config()` with _default_ values.
    fn listen() -> SSDPResult<SSDPReceiver<Self::Message>> {
        Self::listen_with_config(&Default::default())
    }

    /// Listen for messages on all local network interfaces.
    ///
    /// # Notes
    /// This will _bind_ to each interface, **NOT** to `INADDR_ANY`.
    ///
    /// If you are on an environment where the network interface will be changing,
    /// you will have to stop listening and start listening again,
    /// or we recommend using `listen_anyaddr_with_config()` instead.
    fn listen_with_config(config: &Config) -> SSDPResult<SSDPReceiver<Self::Message>> {
        let mut ipv4_sock = None;
        let mut ipv6_sock = None;

        // Generate a list of reused sockets on the standard multicast address.
        let addrs: Vec<SocketAddr> = message::map_local(|&addr| Ok(Some(addr)))?;

        for addr in addrs {
            match addr {
                SocketAddr::V4(_) => {
                    let mcast_ip = config.ipv4_addr.parse().unwrap();

                    if ipv4_sock.is_none() {
                        ipv4_sock = Some(net::bind_reuse(("0.0.0.0", config.port))?);
                    }

                    let sock = &ipv4_sock.as_ref().unwrap();

                    debug!("Joining ipv4 multicast {} at iface: {}", mcast_ip, addr);
                    net::join_multicast(sock, &addr, &mcast_ip)?;
                }
                SocketAddr::V6(_) => {
                    let mcast_ip = config.ipv6_addr.parse().unwrap();

                    if ipv6_sock.is_none() {
                        let socket = net::bind_reuse(("::", config.port))?;
                        ipv6_sock = Some(socket);
                    }

                    let sock = &ipv6_sock.as_ref().unwrap();

                    debug!("Joining ipv6 multicast {} at iface: {}", mcast_ip, addr);
                    net::join_multicast(sock, &addr, &IpAddr::V6(mcast_ip))?;
                }
            }
        }

        let sockets = vec![ipv4_sock, ipv6_sock].into_iter().flatten().collect();

        let receiver = SSDPReceiver::new(sockets, None)?;
        Ok(receiver)
    }

    /// Listen on any interface
    ///
    /// # Important
    ///
    /// This version of the `listen`()` will _bind_ to `INADDR_ANY` instead of binding to each interface
    #[cfg(target_os = "linux")]
    fn listen_anyaddr_with_config(config: &Config) -> SSDPResult<SSDPReceiver<Self::Message>> {
        // Ipv4
        let mcast_ip = config.ipv4_addr.parse().unwrap();
        let ipv4_sock = net::bind_reuse(("0.0.0.0", config.port))?;
        ipv4_sock.join_multicast_v4(&mcast_ip, &"0.0.0.0".parse().unwrap())?;

        // Ipv6
        let mcast_ip = config.ipv6_addr.parse().unwrap();
        let ipv6_sock = net::bind_reuse(("::", config.port))?;
        ipv6_sock.join_multicast_v6(&mcast_ip, 0)?;

        let sockets = vec![ipv4_sock, ipv6_sock];
        let receiver = SSDPReceiver::new(sockets, None)?;
        Ok(receiver)
    }
}
