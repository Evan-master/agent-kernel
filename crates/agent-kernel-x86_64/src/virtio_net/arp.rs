//! Minimal Ethernet/ARP wire contract for the native network proof.
//!
//! This allocation-free encoder targets QEMU user networking and validates a
//! gateway reply against the guest MAC and fixed IPv4 identities.

use agent_kernel_core::NetworkMacAddress;

pub const ARP_FRAME_BYTES: usize = 60;
pub const QEMU_USER_GUEST_IPV4: [u8; 4] = [10, 0, 2, 15];
pub const QEMU_USER_GATEWAY_IPV4: [u8; 4] = [10, 0, 2, 2];

const ETHERNET_ARP: [u8; 2] = 0x0806_u16.to_be_bytes();
const HARDWARE_ETHERNET: [u8; 2] = 1_u16.to_be_bytes();
const PROTOCOL_IPV4: [u8; 2] = 0x0800_u16.to_be_bytes();
const ARP_REQUEST: [u8; 2] = 1_u16.to_be_bytes();
const ARP_REPLY: [u8; 2] = 2_u16.to_be_bytes();

pub fn build_arp_request(frame: &mut [u8; ARP_FRAME_BYTES], guest: NetworkMacAddress) {
    frame.fill(0);
    frame[0..6].fill(0xff);
    frame[6..12].copy_from_slice(&guest.bytes());
    frame[12..14].copy_from_slice(&ETHERNET_ARP);
    frame[14..16].copy_from_slice(&HARDWARE_ETHERNET);
    frame[16..18].copy_from_slice(&PROTOCOL_IPV4);
    frame[18] = 6;
    frame[19] = 4;
    frame[20..22].copy_from_slice(&ARP_REQUEST);
    frame[22..28].copy_from_slice(&guest.bytes());
    frame[28..32].copy_from_slice(&QEMU_USER_GUEST_IPV4);
    frame[38..42].copy_from_slice(&QEMU_USER_GATEWAY_IPV4);
}

pub fn is_expected_arp_reply(frame: &[u8], guest: NetworkMacAddress) -> bool {
    frame.len() >= 42
        && frame[0..6] == guest.bytes()
        && frame[6..12] == frame[22..28]
        && frame[12..14] == ETHERNET_ARP
        && frame[14..16] == HARDWARE_ETHERNET
        && frame[16..18] == PROTOCOL_IPV4
        && frame[18] == 6
        && frame[19] == 4
        && frame[20..22] == ARP_REPLY
        && frame[28..32] == QEMU_USER_GATEWAY_IPV4
        && frame[32..38] == guest.bytes()
        && frame[38..42] == QEMU_USER_GUEST_IPV4
}
