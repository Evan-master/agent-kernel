use agent_kernel_core::NetworkMacAddress;
use agent_kernel_x86_64::virtio_net::{
    build_arp_request, is_expected_arp_reply, ARP_FRAME_BYTES, QEMU_USER_GATEWAY_IPV4,
    QEMU_USER_GUEST_IPV4,
};

const GUEST_MAC: NetworkMacAddress = match NetworkMacAddress::new([0x52, 0x54, 0, 0x12, 0x34, 0x56])
{
    Some(mac) => mac,
    None => panic!("fixed guest MAC"),
};

#[test]
fn request_encodes_broadcast_ethernet_and_qemu_user_gateway_target() {
    let mut frame = [0xa5; ARP_FRAME_BYTES];
    build_arp_request(&mut frame, GUEST_MAC);

    assert_eq!(&frame[0..6], &[0xff; 6]);
    assert_eq!(&frame[6..12], &GUEST_MAC.bytes());
    assert_eq!(&frame[12..14], &0x0806_u16.to_be_bytes());
    assert_eq!(&frame[20..22], &1_u16.to_be_bytes());
    assert_eq!(&frame[22..28], &GUEST_MAC.bytes());
    assert_eq!(&frame[28..32], &QEMU_USER_GUEST_IPV4);
    assert_eq!(&frame[32..38], &[0; 6]);
    assert_eq!(&frame[38..42], &QEMU_USER_GATEWAY_IPV4);
    assert_eq!(&frame[42..], &[0; ARP_FRAME_BYTES - 42]);
}

#[test]
fn reply_validation_binds_gateway_ip_and_guest_identity() {
    let mut reply = [0; 60];
    reply[0..6].copy_from_slice(&GUEST_MAC.bytes());
    reply[6..12].copy_from_slice(&[0x52, 0x55, 0x0a, 0, 2, 2]);
    reply[12..14].copy_from_slice(&0x0806_u16.to_be_bytes());
    reply[14..16].copy_from_slice(&1_u16.to_be_bytes());
    reply[16..18].copy_from_slice(&0x0800_u16.to_be_bytes());
    reply[18] = 6;
    reply[19] = 4;
    reply[20..22].copy_from_slice(&2_u16.to_be_bytes());
    reply[22..28].copy_from_slice(&[0x52, 0x55, 0x0a, 0, 2, 2]);
    reply[28..32].copy_from_slice(&QEMU_USER_GATEWAY_IPV4);
    reply[32..38].copy_from_slice(&GUEST_MAC.bytes());
    reply[38..42].copy_from_slice(&QEMU_USER_GUEST_IPV4);

    assert!(is_expected_arp_reply(&reply, GUEST_MAC));
    reply[31] = 3;
    assert!(!is_expected_arp_reply(&reply, GUEST_MAC));
}
