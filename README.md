# udp_sas_mio

[![Build Status](https://travis-ci.org/a-ba/udp_sas_mio.svg?branch=master)](https://travis-ci.org/a-ba/udp_sas_mio)
[![Crates.io](https://img.shields.io/crates/v/udp_sas_mio.svg)](https://crates.io/crates/udp_sas_mio)

**Source address selection for UDP mio sockets in Rust**

This crate provides an extension trait for `mio::net::UdpSocket` that supports
source address selection for outgoing UDP datagrams. This is useful for
implementing a UDP server that binds multiple network interfaces.
 
The implementation relies on crate [udp_sas](https://crates.io/crates/udp_sas)

