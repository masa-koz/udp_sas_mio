//! This crate provides an [extension trait] for `mio::net::UdpSocket` that supports source address
//! selection for outgoing UDP datagrams. This is useful for implementing a UDP server that binds
//! multiple network interfaces.
//! 
//! The implementation relies on socket options [`IP_PKTINFO`] \(for IPv4) and [`IPV6_RECVPKTINFO`]
//! \(for IPv6).
//! 
//! [extension trait]:      trait.UdpSas.html
//! [`IP_PKTINFO`]:         http://man7.org/linux/man-pages/man7/ip.7.html      
//! [`IPV6_RECVPKTINFO`]:   http://man7.org/linux/man-pages/man7/ipv6.7.html
//! 
//! 
//! ```
//! extern crate mio;
//! extern crate udp_sas_mio;
//! 
//! use std::net::SocketAddr;
//! use std::time::Duration;
//! 
//! use mio::{Events,Poll,PollOpt,Ready,Token};
//! use mio::net::UdpSocket;
//! 
//! use udp_sas_mio::UdpSas;
//! 
//! fn main() {
//!     demo().unwrap();
//! }
//! fn demo() -> std::io::Result<()>
//! {
//!     let mut buf = [0u8;128];
//! 
//!     // Create the server socket and bind it to 0.0.0.0:30012
//!     //
//!     // Note: we will use 127.0.0.23 as source/destination address
//!     //       for our datagrams (to demonstrate the crate features)
//!     //
//!     let srv = UdpSocket::bind_sas(&"0.0.0.0:30012".parse::<SocketAddr>().unwrap())?;
//!     let srv_addr : SocketAddr = "127.0.0.23:30012".parse().unwrap();
//! 
//!     // Create the client socket and bind it to an anonymous port
//!     //
//!     // Note: we will use 127.0.0.45 as source/destination address
//!     //       for our datagrams (to demonstrate the crate features)
//!     //
//!     let cli = UdpSocket::bind_sas(&"0.0.0.0:0".parse::<SocketAddr>().unwrap())?;
//!     let cli_addr = SocketAddr::new(
//!         "127.0.0.45".parse().unwrap(),
//!         cli.local_addr().unwrap().port());
//!     assert_ne!(cli_addr.port(), 0);
//!     
//!     // Messages to be sent
//!     let msg1 = "What do you get if you multiply six by nine?";
//!     let msg2 = "Forty-two";
//!
//!     // create the Poll object and register the sockets
//!     let poll = Poll::new()?;
//!     const SRV : Token = Token(0);
//!     const CLI : Token = Token(1);
//!     poll.register(&cli, CLI, Ready::writable(), PollOpt::level() | PollOpt::oneshot())?;
//!     poll.register(&srv, SRV, Ready::readable(), PollOpt::level())?;
//!     
//!     let timeout = Some(Duration::from_millis(100));
//!     let mut events = Events::with_capacity(1);
//!     loop
//!     {
//!         let nb = poll.poll(&mut events, timeout)?;
//!         assert!(nb != 0, "timeout");
//!         
//!         for event in events.iter()
//!         {
//!             match event.token() {
//!                 CLI if event.readiness().is_writable() => {
//!                     // send a request (msg1) from the client to the server
//!                     let nb = cli.send_sas(msg1.as_bytes(), &srv_addr, &cli_addr.ip())?;
//!                     assert_eq!(nb, msg1.as_bytes().len());
//!                     
//!                     poll.reregister(&cli, CLI, Ready::readable(), PollOpt::level())?;
//!                 },
//!                 
//!                 SRV if event.readiness().is_readable() => {
//!                     // receive the request on the server
//!                     let (nb, peer, local) = srv.recv_sas(&mut buf)?;
//!                     assert_eq!(peer,  cli_addr);
//!                     assert_eq!(local, srv_addr.ip());
//!                     assert_eq!(nb,          msg1.as_bytes().len());
//!                     assert_eq!(&buf[0..nb], msg1.as_bytes());
//!                     
//!                     poll.reregister(&srv, SRV, Ready::writable(),
//!                                     PollOpt::level() | PollOpt::oneshot())?;
//!                 },
//!                 
//!                 SRV if event.readiness().is_writable() => {
//!                     // send a reply (msg2) from the server to the client
//!                     let nb = srv.send_sas(msg2.as_bytes(), &cli_addr, &srv_addr.ip())?;
//!                     assert_eq!(nb, msg2.as_bytes().len());
//!                 },
//!                 
//!                 CLI if event.readiness().is_readable() => {
//!                     // receive the reply on the client
//!                     let (nb, peer, local) = cli.recv_sas(&mut buf)?;
//!                     assert_eq!(peer,  srv_addr);
//!                     assert_eq!(local, cli_addr.ip());
//!                     assert_eq!(nb,          msg2.as_bytes().len());
//!                     assert_eq!(&buf[0..nb], msg2.as_bytes());
//!                     
//!                     return Ok(());
//!                 },
//!             
//!                 token => panic!("unexpected token: {:?}", token)
//!             }
//!         }
//!     }
//! }
//! ```

extern crate mio;
extern crate udp_sas;

use std::io;
use std::net::{SocketAddr, IpAddr};
use std::os::unix::io::AsRawFd;

use mio::net::UdpSocket;

use udp_sas::{recv_sas,send_sas,set_pktinfo};


/// An extension trait to support source address selection in `mio::net::UdpSocket`
/// 
/// See [module level][mod] documentation for more details.
/// 
/// [mod]: index.html
/// 
pub trait UdpSas : Sized
{
    /// Creates a UDP socket from the given address.
    ///
    /// The new socket is configured with the `IP_PKTINFO` or `IPV6_RECVPKTINFO` option enabled.
    /// 
    fn bind_sas(addr: &SocketAddr) -> io::Result<Self>;


    /// Sends a datagram to the given `target` address and use the `local` address as its
    /// source.
    /// 
    /// On success, returns the number of bytes written.
    /// 
    fn send_sas(&self, buf: &[u8], target: &SocketAddr, local: &IpAddr) -> io::Result<usize>;

    /// Receive a datagram
    /// 
    /// On success, returns a tuple `(nb, source, local)` containing the number of bytes read, the
    /// source socket address (peer address), and the destination ip address (local address).
    /// 
    fn recv_sas(&self, buf: &mut[u8]) -> io::Result<(usize, SocketAddr, IpAddr)>;
}


impl UdpSas for UdpSocket
{
    fn bind_sas(addr: &SocketAddr) -> io::Result<UdpSocket> {
        let sock = UdpSocket::bind(addr)?;
        set_pktinfo(sock.as_raw_fd())?;
        Ok(sock)
    }

    fn send_sas(&self, buf: &[u8], target: &SocketAddr, local: &IpAddr) -> io::Result<usize>
    {
        send_sas(self.as_raw_fd(), buf, Some(target), Some(local))
    }

    fn recv_sas(&self, buf: &mut[u8]) -> io::Result<(usize, SocketAddr, IpAddr)>
    {
        let (nb, src, local) = recv_sas(self.as_raw_fd(), buf)?;
        match (src, local) {
            (Some(src), Some(local)) => Ok((nb, src, local)),
            (None, _) => Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "local address not available (IP_PKTINFO/IPV6_RECVPKTINFO may not be enabled on the socket)")),
            (_, None) => Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "source address not available (maybe the socket is connected)"
                    )),
        }
    }
}


