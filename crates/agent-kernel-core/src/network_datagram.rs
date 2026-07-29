//! Architecture-neutral IPv4/UDP datagram evidence.
//!
//! Core retains semantic addresses, ports, lengths, and payload digests. Wire
//! headers, checksums, and packet bytes remain architecture-owned.

use crate::KernelError;

pub const IPV4_HEADER_BYTES: u16 = 20;
pub const UDP_HEADER_BYTES: u16 = 8;
pub const UDP_IPV4_MAX_PAYLOAD_BYTES: u16 =
    crate::NETWORK_MAX_MTU - IPV4_HEADER_BYTES - UDP_HEADER_BYTES;
pub const ETHERNET_MIN_FRAME_BYTES: u16 = 60;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct NetworkIpv4Address([u8; 4]);

impl NetworkIpv4Address {
    pub const fn new(bytes: [u8; 4]) -> Option<Self> {
        let unspecified = bytes[0] == 0 && bytes[1] == 0 && bytes[2] == 0 && bytes[3] == 0;
        let loopback = bytes[0] == 127;
        let multicast_or_reserved = bytes[0] >= 224;
        if unspecified || loopback || multicast_or_reserved {
            None
        } else {
            Some(Self(bytes))
        }
    }

    pub const fn bytes(self) -> [u8; 4] {
        self.0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct NetworkUdpPort(u16);

impl NetworkUdpPort {
    pub const fn new(value: u16) -> Option<Self> {
        if value == 0 {
            None
        } else {
            Some(Self(value))
        }
    }

    pub const fn get(self) -> u16 {
        self.0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct NetworkDatagramDescriptor {
    source: NetworkIpv4Address,
    destination: NetworkIpv4Address,
    source_port: NetworkUdpPort,
    destination_port: NetworkUdpPort,
    payload_length: u16,
    payload_digest: [u8; 32],
}

impl NetworkDatagramDescriptor {
    pub const fn new(
        source: NetworkIpv4Address,
        destination: NetworkIpv4Address,
        source_port: NetworkUdpPort,
        destination_port: NetworkUdpPort,
        payload_length: u16,
        payload_digest: [u8; 32],
    ) -> Result<Self, KernelError> {
        if payload_length > UDP_IPV4_MAX_PAYLOAD_BYTES {
            Err(KernelError::NetworkDatagramInvalid)
        } else {
            Ok(Self {
                source,
                destination,
                source_port,
                destination_port,
                payload_length,
                payload_digest,
            })
        }
    }

    pub const fn source(self) -> NetworkIpv4Address {
        self.source
    }

    pub const fn destination(self) -> NetworkIpv4Address {
        self.destination
    }

    pub const fn source_port(self) -> NetworkUdpPort {
        self.source_port
    }

    pub const fn destination_port(self) -> NetworkUdpPort {
        self.destination_port
    }

    pub const fn payload_length(self) -> u16 {
        self.payload_length
    }

    pub const fn payload_digest(self) -> [u8; 32] {
        self.payload_digest
    }

    pub const fn ipv4_packet_length(self) -> u16 {
        IPV4_HEADER_BYTES + UDP_HEADER_BYTES + self.payload_length
    }

    pub const fn ethernet_frame_length(self) -> u16 {
        let unpadded = crate::ETHERNET_HEADER_BYTES + self.ipv4_packet_length();
        if unpadded < ETHERNET_MIN_FRAME_BYTES {
            ETHERNET_MIN_FRAME_BYTES
        } else {
            unpadded
        }
    }
}
