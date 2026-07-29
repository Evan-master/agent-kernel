//! Native modern Virtio network-device support.
//!
//! This x86 architecture layer owns two fixed split virtqueues, strict minimal
//! feature negotiation, PCI capability discovery, and bounded frame handling.

mod arp;
mod device;
mod device_config;
mod ipv4_udp;
mod pci;
mod queue_layout;
mod rx_queue;
mod transport;
mod transport_queue;
mod transport_types;
mod tx_queue;

pub use arp::{
    build_arp_request, is_expected_arp_reply, ARP_FRAME_BYTES, QEMU_USER_GATEWAY_IPV4,
    QEMU_USER_GUEST_IPV4,
};
pub use device::{VirtioNetDevice, VirtioNetDeviceError};
pub use device_config::{
    VirtioNetDeviceConfigError, VirtioNetDeviceConfigIo, VolatileVirtioNetDeviceConfig,
};
pub use ipv4_udp::{
    build_udp_ipv4_frame, decode_udp_ipv4_frame, UdpIpv4Error, UdpIpv4Packet, UdpIpv4Route,
};
pub use pci::{
    virtio_net_selector, VirtioNetPciCapabilities, VirtioNetPciCapabilityError, VirtioNetPciRegion,
    VirtioNetPciRegionError, VirtioNetPciRegions, VirtioNetRequiredCapability,
};
pub use queue_layout::{
    VirtioNetQueueError, VirtioNetQueueLayout, VIRTIO_NET_AVAILABLE_OFFSET,
    VIRTIO_NET_DESCRIPTOR_OFFSET, VIRTIO_NET_HEADER_BYTES, VIRTIO_NET_RX_BUFFER_BYTES,
    VIRTIO_NET_USED_OFFSET,
};
pub use rx_queue::{VirtioNetRxCompletion, VirtioNetRxQueue, VirtioNetRxRequest};
pub use transport::VirtioNetTransport;
pub use transport_types::{
    VirtioNetInterrupt, VirtioNetTransportError, VIRTIO_F_ACCESS_PLATFORM, VIRTIO_F_VERSION_1,
    VIRTIO_NET_DEVICE_ID, VIRTIO_NET_F_MAC, VIRTIO_NET_F_MRG_RXBUF, VIRTIO_NET_VENDOR_ID,
    VIRTIO_PCI_STATUS_ACKNOWLEDGE, VIRTIO_PCI_STATUS_DRIVER, VIRTIO_PCI_STATUS_DRIVER_OK,
    VIRTIO_PCI_STATUS_FAILED, VIRTIO_PCI_STATUS_FEATURES_OK,
};
pub use tx_queue::{VirtioNetTxCompletion, VirtioNetTxQueue, VirtioNetTxRequest};
