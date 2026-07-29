use agent_kernel_core::{NetworkIpv4Address, NetworkMacAddress, NetworkUdpPort};
use agent_kernel_x86_64::virtio_net::{
    build_udp_ipv4_frame, decode_udp_ipv4_frame, UdpIpv4Error, UdpIpv4Route,
};

const GUEST_MAC: NetworkMacAddress = match NetworkMacAddress::new([0x52, 0x54, 0, 0x12, 0x34, 0x56])
{
    Some(mac) => mac,
    None => panic!("fixed guest MAC"),
};
const GATEWAY_MAC: NetworkMacAddress = match NetworkMacAddress::new([0x52, 0x55, 0x0a, 0, 2, 2]) {
    Some(mac) => mac,
    None => panic!("fixed gateway MAC"),
};
const GUEST_IP: NetworkIpv4Address = match NetworkIpv4Address::new([10, 0, 2, 15]) {
    Some(address) => address,
    None => panic!("fixed guest IPv4"),
};
const GATEWAY_IP: NetworkIpv4Address = match NetworkIpv4Address::new([10, 0, 2, 2]) {
    Some(address) => address,
    None => panic!("fixed gateway IPv4"),
};
const GUEST_PORT: NetworkUdpPort = match NetworkUdpPort::new(40131) {
    Some(port) => port,
    None => panic!("fixed guest port"),
};
const ECHO_PORT: NetworkUdpPort = match NetworkUdpPort::new(40130) {
    Some(port) => port,
    None => panic!("fixed echo port"),
};
const PAYLOAD: &[u8] = b"AGENT-V30-UDP";

fn outbound() -> UdpIpv4Route {
    UdpIpv4Route::new(
        GUEST_MAC,
        GATEWAY_MAC,
        GUEST_IP,
        GATEWAY_IP,
        GUEST_PORT,
        ECHO_PORT,
    )
}

fn inbound() -> UdpIpv4Route {
    UdpIpv4Route::new(
        GATEWAY_MAC,
        GUEST_MAC,
        GATEWAY_IP,
        GUEST_IP,
        ECHO_PORT,
        GUEST_PORT,
    )
}

#[test]
fn encoder_emits_canonical_ipv4_udp_headers_and_checksums() {
    let mut frame = [0xa5; 128];
    let frame_length =
        build_udp_ipv4_frame(&mut frame, outbound(), 0x3001, PAYLOAD).expect("encode frame");

    assert_eq!(frame_length, 60);
    assert_eq!(&frame[0..6], &GATEWAY_MAC.bytes());
    assert_eq!(&frame[6..12], &GUEST_MAC.bytes());
    assert_eq!(&frame[12..14], &0x0800_u16.to_be_bytes());
    assert_eq!(frame[14], 0x45);
    assert_eq!(&frame[16..18], &41_u16.to_be_bytes());
    assert_eq!(&frame[18..20], &0x3001_u16.to_be_bytes());
    assert_eq!(&frame[20..22], &[0, 0]);
    assert_eq!(frame[22], 64);
    assert_eq!(frame[23], 17);
    assert_ne!(&frame[24..26], &[0, 0]);
    assert_eq!(&frame[26..30], &GUEST_IP.bytes());
    assert_eq!(&frame[30..34], &GATEWAY_IP.bytes());
    assert_eq!(&frame[34..36], &GUEST_PORT.get().to_be_bytes());
    assert_eq!(&frame[36..38], &ECHO_PORT.get().to_be_bytes());
    assert_eq!(&frame[38..40], &21_u16.to_be_bytes());
    assert_ne!(&frame[40..42], &[0, 0]);
    assert_eq!(&frame[42..55], PAYLOAD);
    assert_eq!(&frame[55..60], &[0; 5]);
}

#[test]
fn decoder_accepts_only_the_expected_unfragmented_flow() {
    let mut frame = [0; 128];
    let frame_length = build_udp_ipv4_frame(&mut frame, inbound(), 0x7123, PAYLOAD).unwrap();
    let packet = decode_udp_ipv4_frame(&frame[..frame_length], inbound()).unwrap();

    assert_eq!(packet.frame_length(), 60);
    assert_eq!(packet.ipv4_packet_length(), 41);
    assert_eq!(packet.payload(), PAYLOAD);

    let mut fragmented = frame;
    fragmented[20..22].copy_from_slice(&0x2000_u16.to_be_bytes());
    assert_eq!(
        decode_udp_ipv4_frame(&fragmented[..frame_length], inbound()),
        Err(UdpIpv4Error::Ipv4Fragmented)
    );

    let mut wrong_port = frame;
    wrong_port[36..38].copy_from_slice(&40132_u16.to_be_bytes());
    assert_eq!(
        decode_udp_ipv4_frame(&wrong_port[..frame_length], inbound()),
        Err(UdpIpv4Error::UdpRouteMismatch)
    );
}

#[test]
fn decoder_rejects_missing_or_corrupted_checksums() {
    let mut frame = [0; 128];
    let frame_length = build_udp_ipv4_frame(&mut frame, inbound(), 0x7123, PAYLOAD).unwrap();

    let mut missing_udp_checksum = frame;
    missing_udp_checksum[40..42].fill(0);
    assert_eq!(
        decode_udp_ipv4_frame(&missing_udp_checksum[..frame_length], inbound()),
        Err(UdpIpv4Error::UdpChecksumMissing)
    );

    let mut corrupted_payload = frame;
    corrupted_payload[42] ^= 0x01;
    assert_eq!(
        decode_udp_ipv4_frame(&corrupted_payload[..frame_length], inbound()),
        Err(UdpIpv4Error::UdpChecksumInvalid)
    );

    let mut corrupted_header = frame;
    corrupted_header[22] -= 1;
    assert_eq!(
        decode_udp_ipv4_frame(&corrupted_header[..frame_length], inbound()),
        Err(UdpIpv4Error::Ipv4ChecksumInvalid)
    );
}
