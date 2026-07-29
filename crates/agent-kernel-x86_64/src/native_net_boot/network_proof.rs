//! Frame evidence and detached-DMA checks for the native network proof.
//!
//! This x86 boot helper hashes validated Ethernet bytes for Core and owns the
//! post-release hardware probe. The probe accepts only an exact requester
//! fault inside the four-page IOVA window with no MSI-X completion.

use sha2::{Digest, Sha256};

use agent_kernel_core::NetworkFrameDescriptor;
use agent_kernel_x86_64::{
    cpu::ApicId,
    iommu::{IntelVtd, VtdRegisterIo},
    virtio_net::{
        VirtioNetDevice, VirtioNetDeviceConfigIo, VirtioNetDeviceError, VirtioNetQueueError,
    },
    virtio_rng::{VirtioCommonConfigIo, VirtioIsrIo, VirtioNotifyIo},
};

use crate::{fatal_boot, serial_write_line};

use super::{
    interrupts, pci,
    proof::{fatal_after_enable, wait_for_fault},
    NET_RX_METADATA_IOVA, NET_RX_PACKET_IOVA, NET_TX_METADATA_IOVA, NET_TX_PACKET_IOVA,
};

pub(super) fn run_detached_dma_probe<C, N, I, D, V>(
    hardware: &mut pci::PreparedNativeNetHardware,
    net: &mut VirtioNetDevice<'_, C, N, I, D>,
    iommu: &mut IntelVtd<V>,
    destination: ApicId,
    source_id: u16,
    frame: &[u8],
) where
    C: VirtioCommonConfigIo,
    N: VirtioNotifyIo,
    I: VirtioIsrIo,
    D: VirtioNetDeviceConfigIo,
    V: VtdRegisterIo,
{
    hardware
        .enable_memory_decode()
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_NET_DENIAL_PROBE_ERROR"));
    hardware
        .configure_msix(destination)
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_NET_DENIAL_PROBE_ERROR"));
    net.initialize(0, 1)
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_NET_DENIAL_PROBE_ERROR"));
    net.prepare_transmit(frame)
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_NET_DENIAL_PROBE_ERROR"));
    hardware.activate_bus_master().unwrap_or_else(|_| {
        fatal_after_enable(hardware, "AGENT_KERNEL_NATIVE_NET_DENIAL_PROBE_ERROR")
    });
    interrupts::reset();
    net.notify_transmit().unwrap_or_else(|_| {
        fatal_after_enable(hardware, "AGENT_KERNEL_NATIVE_NET_DENIAL_PROBE_ERROR")
    });
    let fault = wait_for_fault(iommu)
        .unwrap_or_else(|_| {
            fatal_after_enable(hardware, "AGENT_KERNEL_NATIVE_NET_DMA_FAULT_READ_ERROR")
        })
        .unwrap_or_else(|| {
            fatal_after_enable(hardware, "AGENT_KERNEL_NATIVE_NET_DMA_FAULT_MISSING_ERROR")
        });
    let page = fault.address() & !0xfff;
    if fault.source_id() != source_id
        || ![
            NET_RX_METADATA_IOVA,
            NET_RX_PACKET_IOVA,
            NET_TX_METADATA_IOVA,
            NET_TX_PACKET_IOVA,
        ]
        .contains(&page)
        || !interrupts::observe_no_interrupts()
    {
        fatal_after_enable(hardware, "AGENT_KERNEL_NATIVE_NET_DMA_FAULT_MISMATCH_ERROR");
    }
    net.shutdown().unwrap_or_else(|_| {
        fatal_after_enable(hardware, "AGENT_KERNEL_NATIVE_NET_DENIAL_PROBE_ERROR")
    });
    hardware.disable_msix().unwrap_or_else(|_| {
        fatal_after_enable(hardware, "AGENT_KERNEL_NATIVE_NET_DENIAL_PROBE_ERROR")
    });
    hardware
        .quiesce()
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_NET_DENIAL_PROBE_ERROR"));
    iommu
        .clear_fault()
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_NET_DMA_FAULT_CLEAR_ERROR"));
    serial_write_line("AGENT_KERNEL_NATIVE_NET_DMA_DENIAL_OK");
}

pub(super) fn frame_descriptor(frame: &[u8]) -> NetworkFrameDescriptor {
    let digest: [u8; 32] = Sha256::digest(frame).into();
    let ether_type = u16::from_be_bytes([frame[12], frame[13]]);
    NetworkFrameDescriptor::new(
        u16::try_from(frame.len()).expect("bounded Ethernet frame"),
        ether_type,
        digest,
    )
    .expect("validated Ethernet frame")
}

pub(super) const fn rx_error_marker(error: VirtioNetDeviceError) -> &'static str {
    match error {
        VirtioNetDeviceError::Queue(VirtioNetQueueError::InvalidNetworkHeader) => {
            "AGENT_KERNEL_NATIVE_NET_RX_HEADER_ERROR"
        }
        VirtioNetDeviceError::Queue(VirtioNetQueueError::InvalidBufferCount(_)) => {
            "AGENT_KERNEL_NATIVE_NET_RX_BUFFER_COUNT_ERROR"
        }
        VirtioNetDeviceError::Queue(VirtioNetQueueError::InvalidCompletionLength(_)) => {
            "AGENT_KERNEL_NATIVE_NET_RX_LENGTH_ERROR"
        }
        VirtioNetDeviceError::Queue(VirtioNetQueueError::UnexpectedUsedIndex { .. }) => {
            "AGENT_KERNEL_NATIVE_NET_RX_USED_INDEX_ERROR"
        }
        VirtioNetDeviceError::Queue(VirtioNetQueueError::UnexpectedDescriptor { .. }) => {
            "AGENT_KERNEL_NATIVE_NET_RX_DESCRIPTOR_ERROR"
        }
        VirtioNetDeviceError::Queue(VirtioNetQueueError::InvalidEtherType(_)) => {
            "AGENT_KERNEL_NATIVE_NET_RX_ETHERTYPE_ERROR"
        }
        _ => "AGENT_KERNEL_NATIVE_NET_RX_COMPLETION_ERROR",
    }
}
