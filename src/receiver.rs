//! Primitives for non-blocking SSDP message receiving.

use std::io;
use std::net::{SocketAddr, UdpSocket};
use std::result::Result;
use std::sync::mpsc::{self, Iter, Receiver, RecvError, Sender, TryRecvError};
use std::thread;
use std::time::Duration;

use crate::net::packet::PacketReceiver;
use crate::SSDPResult;

/// Trait for constructing an object from some serialized SSDP message.
pub trait FromRawSSDP: Sized {
    fn raw_ssdp(bytes: &[u8]) -> SSDPResult<Self>;
}

/// Iterator for an `SSDPReceiver`.
pub struct SSDPIter<T> {
    recv: SSDPReceiver<T>,
}

impl<T> SSDPIter<T> {
    fn new(recv: SSDPReceiver<T>) -> SSDPIter<T> {
        SSDPIter { recv }
    }
}

impl<T> Iterator for SSDPIter<T> {
    type Item = (T, SocketAddr);

    fn next(&mut self) -> Option<Self::Item> {
        self.recv.recv().ok()
    }
}

/// A non-blocking SSDP message receiver.
pub struct SSDPReceiver<T> {
    recvr: Receiver<(T, SocketAddr)>,
}

impl<T> SSDPReceiver<T>
where
    T: FromRawSSDP + Send + 'static,
{
    /// Construct a receiver that receives bytes from a number of UdpSockets and
    /// tries to construct an object T from them. If a duration is provided, the
    /// channel will be shutdown after the specified duration.
    ///
    /// Due to implementation details, none of the UdpSockets should be bound to
    /// the default route, 0.0.0.0, address.
    pub fn new(socks: Vec<UdpSocket>, time: Option<Duration>) -> io::Result<SSDPReceiver<T>> {
        let (send, recv) = mpsc::channel();

        // Ensure `receive_packets` times out in the event the timeout packet is not received
        for sock in socks.iter() {
            sock.set_read_timeout(time)?;
        }

        // Spawn Receiver Threads
        spawn_receivers(socks, send);

        Ok(SSDPReceiver { recvr: recv })
    }
}

/// Spawn a number of receiver threads that will receive packets, forward the
/// bytes on to T, and send successfully constructed objects through the sender.
fn spawn_receivers<T>(socks: Vec<UdpSocket>, sender: Sender<(T, SocketAddr)>)
where
    T: FromRawSSDP + Send + 'static,
{
    for sock in socks {
        let pckt_recv = PacketReceiver::new(sock);
        let sender = sender.clone();

        thread::spawn(move || {
            receive_packets(pckt_recv, sender);
        });
    }
}

impl<T> SSDPReceiver<T> {
    /// Non-blocking method that attempts to read a value from the receiver.
    pub fn try_recv(&self) -> Result<(T, SocketAddr), TryRecvError> {
        self.recvr.try_recv()
    }

    /// Blocking method that reads a value from the receiver until one is available.
    pub fn recv(&self) -> Result<(T, SocketAddr), RecvError> {
        self.recvr.recv()
    }
}

impl<'a, T> IntoIterator for &'a SSDPReceiver<T> {
    type Item = (T, SocketAddr);
    type IntoIter = Iter<'a, (T, SocketAddr)>;

    fn into_iter(self) -> Self::IntoIter {
        self.recvr.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut SSDPReceiver<T> {
    type Item = (T, SocketAddr);
    type IntoIter = Iter<'a, (T, SocketAddr)>;

    fn into_iter(self) -> Self::IntoIter {
        self.recvr.iter()
    }
}

impl<T> IntoIterator for SSDPReceiver<T> {
    type Item = (T, SocketAddr);
    type IntoIter = SSDPIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        SSDPIter::new(self)
    }
}

/// Receives bytes and attempts to construct a T which will be sent through the supplied channel.
///
/// This should almost always be run in it's own thread.
fn receive_packets<T>(recv: PacketReceiver, send: Sender<(T, SocketAddr)>)
where
    T: FromRawSSDP + Send,
{
    // TODO: Add logging to this function. Maybe forward sender IP Address along
    // so that we can do some checks when we parse the http.
    loop {
        log::trace!("Waiting on packet at {}...", recv);
        let (msg_bytes, addr) = match recv.recv_pckt() {
            Ok((bytes, addr)) => (bytes, addr),
            // Unix returns WouldBlock on timeout while Windows returns TimedOut
            Err(ref err)
                if err.kind() == io::ErrorKind::WouldBlock || err.kind() == io::ErrorKind::TimedOut =>
            {
                // We have waited for at least the desired timeout (or possibly longer)
                log::trace!("Receiver at {} timed out", recv);
                return;
            }
            Err(_) => {
                continue;
            }
        };

        log::trace!("Received packet with {} bytes", msg_bytes.len());

        // Unwrap Will Cause A Panic If Receiver Hung Up Which Is Desired
        match T::raw_ssdp(&msg_bytes[..]) {
            Ok(n) => match send.send((n, addr)) {
                Ok(_) => {}
                Err(e) => {
                    log::error!("Error {e:?} on receive packages ");
                    return;
                }
            },
            Err(_) => {
                continue;
            }
        };
    }
}
