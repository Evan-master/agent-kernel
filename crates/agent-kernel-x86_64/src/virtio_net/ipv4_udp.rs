//! Allocation-free Ethernet II, IPv4, and UDP wire codec.
//!
//! The decoder accepts optional Ethernet padding while requiring canonical
//! IPv4 and UDP headers, exact routes, and valid nonzero checksums.

use agent_kernel_core::{
    NetworkIpv4Address, NetworkMacAddress, NetworkUdpPort, ETHERNET_MAX_FRAME_BYTES,
    UDP_IPV4_MAX_PAYLOAD_BYTES,
};

const ETHERNET_HEADER_BYTES: usize = 14;
const IPV4_HEADER_BYTES: usize = 20;
const UDP_HEADER_BYTES: usize = 8;
const ETHERNET_MIN_FRAME_BYTES: usize = 60;
const ETHER_TYPE_IPV4: u16 = 0x0800;
const IPV4_PROTOCOL_UDP: u8 = 17;
const IPV4_TTL: u8 = 64;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct UdpIpv4Route {
    source_mac: NetworkMacAddress,
    destination_mac: NetworkMacAddress,
    source_ip: NetworkIpv4Address,
    destination_ip: NetworkIpv4Address,
    source_port: NetworkUdpPort,
    destination_port: NetworkUdpPort,
}

impl UdpIpv4Route {
    pub const fn new(
        source_mac: NetworkMacAddress,
        destination_mac: NetworkMacAddress,
        source_ip: NetworkIpv4Address,
        destination_ip: NetworkIpv4Address,
        source_port: NetworkUdpPort,
        destination_port: NetworkUdpPort,
    ) -> Self {
        Self {
            source_mac,
            destination_mac,
            source_ip,
            destination_ip,
            source_port,
            destination_port,
        }
    }

    pub const fn source_ip(self) -> NetworkIpv4Address {
        self.source_ip
    }

    pub const fn destination_ip(self) -> NetworkIpv4Address {
        self.destination_ip
    }

    pub const fn source_port(self) -> NetworkUdpPort {
        self.source_port
    }

    pub const fn destination_port(self) -> NetworkUdpPort {
        self.destination_port
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum UdpIpv4Error {
    BufferTooSmall,
    PayloadTooLarge,
    EthernetRouteMismatch,
    EthernetTypeInvalid,
    Ipv4HeaderInvalid,
    Ipv4ChecksumInvalid,
    Ipv4Fragmented,
    Ipv4RouteMismatch,
    UdpHeaderInvalid,
    UdpRouteMismatch,
    UdpChecksumMissing,
    UdpChecksumInvalid,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct UdpIpv4Packet<'a> {
    frame_length: usize,
    ipv4_packet_length: usize,
    payload: &'a [u8],
}

impl<'a> UdpIpv4Packet<'a> {
    pub const fn frame_length(self) -> usize {
        self.frame_length
    }

    pub const fn ipv4_packet_length(self) -> usize {
        self.ipv4_packet_length
    }

    pub const fn payload(self) -> &'a [u8] {
        self.payload
    }
}

pub fn build_udp_ipv4_frame(
    frame: &mut [u8],
    route: UdpIpv4Route,
    identification: u16,
    payload: &[u8],
) -> Result<usize, UdpIpv4Error> {
    if payload.len() > usize::from(UDP_IPV4_MAX_PAYLOAD_BYTES) {
        return Err(UdpIpv4Error::PayloadTooLarge);
    }
    let udp_length = UDP_HEADER_BYTES + payload.len();
    let ipv4_length = IPV4_HEADER_BYTES + udp_length;
    let unpadded_length = ETHERNET_HEADER_BYTES + ipv4_length;
    let frame_length = unpadded_length.max(ETHERNET_MIN_FRAME_BYTES);
    if frame.len() < frame_length {
        return Err(UdpIpv4Error::BufferTooSmall);
    }

    frame[..frame_length].fill(0);
    frame[0..6].copy_from_slice(&route.destination_mac.bytes());
    frame[6..12].copy_from_slice(&route.source_mac.bytes());
    frame[12..14].copy_from_slice(&ETHER_TYPE_IPV4.to_be_bytes());

    let ipv4 = ETHERNET_HEADER_BYTES;
    frame[ipv4] = 0x45;
    frame[ipv4 + 2..ipv4 + 4].copy_from_slice(&(ipv4_length as u16).to_be_bytes());
    frame[ipv4 + 4..ipv4 + 6].copy_from_slice(&identification.to_be_bytes());
    frame[ipv4 + 8] = IPV4_TTL;
    frame[ipv4 + 9] = IPV4_PROTOCOL_UDP;
    frame[ipv4 + 12..ipv4 + 16].copy_from_slice(&route.source_ip.bytes());
    frame[ipv4 + 16..ipv4 + 20].copy_from_slice(&route.destination_ip.bytes());
    let header_checksum = internet_checksum(&frame[ipv4..ipv4 + IPV4_HEADER_BYTES]);
    frame[ipv4 + 10..ipv4 + 12].copy_from_slice(&header_checksum.to_be_bytes());

    let udp = ipv4 + IPV4_HEADER_BYTES;
    frame[udp..udp + 2].copy_from_slice(&route.source_port.get().to_be_bytes());
    frame[udp + 2..udp + 4].copy_from_slice(&route.destination_port.get().to_be_bytes());
    frame[udp + 4..udp + 6].copy_from_slice(&(udp_length as u16).to_be_bytes());
    frame[udp + UDP_HEADER_BYTES..udp + udp_length].copy_from_slice(payload);
    let mut udp_checksum = transport_checksum(
        route.source_ip.bytes(),
        route.destination_ip.bytes(),
        &frame[udp..udp + udp_length],
    );
    if udp_checksum == 0 {
        udp_checksum = u16::MAX;
    }
    frame[udp + 6..udp + 8].copy_from_slice(&udp_checksum.to_be_bytes());
    Ok(frame_length)
}

pub fn decode_udp_ipv4_frame<'a>(
    frame: &'a [u8],
    route: UdpIpv4Route,
) -> Result<UdpIpv4Packet<'a>, UdpIpv4Error> {
    if frame.len() < ETHERNET_HEADER_BYTES + IPV4_HEADER_BYTES + UDP_HEADER_BYTES
        || frame.len() > usize::from(ETHERNET_MAX_FRAME_BYTES)
    {
        return Err(UdpIpv4Error::Ipv4HeaderInvalid);
    }
    if frame[0..6] != route.destination_mac.bytes() || frame[6..12] != route.source_mac.bytes() {
        return Err(UdpIpv4Error::EthernetRouteMismatch);
    }
    if read_u16(frame, 12) != ETHER_TYPE_IPV4 {
        return Err(UdpIpv4Error::EthernetTypeInvalid);
    }

    let ipv4 = ETHERNET_HEADER_BYTES;
    if frame[ipv4] != 0x45 || frame[ipv4 + 1] != 0 {
        return Err(UdpIpv4Error::Ipv4HeaderInvalid);
    }
    let ipv4_length = usize::from(read_u16(frame, ipv4 + 2));
    if ipv4_length < IPV4_HEADER_BYTES + UDP_HEADER_BYTES
        || ipv4_length > usize::from(agent_kernel_core::NETWORK_MAX_MTU)
        || ETHERNET_HEADER_BYTES + ipv4_length > frame.len()
        || frame[ipv4 + 8] == 0
        || frame[ipv4 + 9] != IPV4_PROTOCOL_UDP
    {
        return Err(UdpIpv4Error::Ipv4HeaderInvalid);
    }
    if read_u16(frame, ipv4 + 6) & 0x3fff != 0 {
        return Err(UdpIpv4Error::Ipv4Fragmented);
    }
    if internet_checksum(&frame[ipv4..ipv4 + IPV4_HEADER_BYTES]) != 0 {
        return Err(UdpIpv4Error::Ipv4ChecksumInvalid);
    }
    if frame[ipv4 + 12..ipv4 + 16] != route.source_ip.bytes()
        || frame[ipv4 + 16..ipv4 + 20] != route.destination_ip.bytes()
    {
        return Err(UdpIpv4Error::Ipv4RouteMismatch);
    }

    let udp = ipv4 + IPV4_HEADER_BYTES;
    let udp_length = usize::from(read_u16(frame, udp + 4));
    if udp_length < UDP_HEADER_BYTES || udp_length != ipv4_length - IPV4_HEADER_BYTES {
        return Err(UdpIpv4Error::UdpHeaderInvalid);
    }
    if read_u16(frame, udp) != route.source_port.get()
        || read_u16(frame, udp + 2) != route.destination_port.get()
    {
        return Err(UdpIpv4Error::UdpRouteMismatch);
    }
    if read_u16(frame, udp + 6) == 0 {
        return Err(UdpIpv4Error::UdpChecksumMissing);
    }
    if transport_checksum(
        route.source_ip.bytes(),
        route.destination_ip.bytes(),
        &frame[udp..udp + udp_length],
    ) != 0
    {
        return Err(UdpIpv4Error::UdpChecksumInvalid);
    }

    Ok(UdpIpv4Packet {
        frame_length: frame.len(),
        ipv4_packet_length: ipv4_length,
        payload: &frame[udp + UDP_HEADER_BYTES..udp + udp_length],
    })
}

fn read_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_be_bytes([bytes[offset], bytes[offset + 1]])
}

fn internet_checksum(bytes: &[u8]) -> u16 {
    finalize_checksum(add_bytes(0, bytes))
}

fn transport_checksum(source: [u8; 4], destination: [u8; 4], udp: &[u8]) -> u16 {
    let mut sum = add_bytes(0, &source);
    sum = add_bytes(sum, &destination);
    sum += u32::from(IPV4_PROTOCOL_UDP);
    sum += udp.len() as u32;
    finalize_checksum(add_bytes(sum, udp))
}

fn add_bytes(mut sum: u32, bytes: &[u8]) -> u32 {
    let mut chunks = bytes.chunks_exact(2);
    for pair in &mut chunks {
        sum += u32::from(u16::from_be_bytes([pair[0], pair[1]]));
    }
    if let Some(last) = chunks.remainder().first() {
        sum += u32::from(*last) << 8;
    }
    sum
}

fn finalize_checksum(mut sum: u32) -> u16 {
    while sum >> 16 != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !(sum as u16)
}
