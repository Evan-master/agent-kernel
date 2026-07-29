//! Shared one-descriptor split-queue layout and encoding helpers.
//!
//! This x86 architecture module defines the allocation-free DMA geometry used
//! by both queues. Metadata and packet pages must stay disjoint and 4 KiB
//! aligned; device-written fields are decoded only through bounded helpers.

use agent_kernel_core::{DMA_PAGE_BYTES, ETHERNET_HEADER_BYTES, ETHERNET_MAX_FRAME_BYTES};

pub const VIRTIO_NET_DESCRIPTOR_OFFSET: u16 = 0x000;
pub const VIRTIO_NET_AVAILABLE_OFFSET: u16 = 0x100;
pub const VIRTIO_NET_USED_OFFSET: u16 = 0x200;
pub const VIRTIO_NET_HEADER_BYTES: usize = 12;
pub const VIRTIO_NET_RX_BUFFER_BYTES: usize =
    VIRTIO_NET_HEADER_BYTES + ETHERNET_MAX_FRAME_BYTES as usize;

pub(crate) const QUEUE_PAGE_BYTES: usize = DMA_PAGE_BYTES as usize;
pub(crate) const DESCRIPTOR_INDEX: u16 = 0;
pub(crate) const DESCRIPTOR_WRITE: u16 = 2;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct VirtioNetQueueLayout {
    metadata_iova: u64,
    packet_iova: u64,
}

impl VirtioNetQueueLayout {
    pub const fn new(metadata_iova: u64, packet_iova: u64) -> Result<Self, VirtioNetQueueError> {
        if metadata_iova & (DMA_PAGE_BYTES - 1) != 0 {
            return Err(VirtioNetQueueError::PageMisaligned(metadata_iova));
        }
        if packet_iova & (DMA_PAGE_BYTES - 1) != 0 {
            return Err(VirtioNetQueueError::PageMisaligned(packet_iova));
        }
        if metadata_iova == packet_iova {
            return Err(VirtioNetQueueError::AliasedPages);
        }
        if metadata_iova.checked_add(DMA_PAGE_BYTES).is_none()
            || packet_iova.checked_add(DMA_PAGE_BYTES).is_none()
        {
            return Err(VirtioNetQueueError::AddressOverflow);
        }
        Ok(Self {
            metadata_iova,
            packet_iova,
        })
    }

    pub const fn descriptor_iova(self) -> u64 {
        self.metadata_iova + VIRTIO_NET_DESCRIPTOR_OFFSET as u64
    }

    pub const fn driver_iova(self) -> u64 {
        self.metadata_iova + VIRTIO_NET_AVAILABLE_OFFSET as u64
    }

    pub const fn device_iova(self) -> u64 {
        self.metadata_iova + VIRTIO_NET_USED_OFFSET as u64
    }

    pub const fn packet_iova(self) -> u64 {
        self.packet_iova
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum VirtioNetQueueError {
    PageMisaligned(u64),
    AliasedPages,
    AddressOverflow,
    InvalidFrameLength(usize),
    InvalidEtherType(u16),
    RequestOutstanding,
    NoRequestOutstanding,
    RequestMismatch,
    SequenceExhausted,
    CompletionPending,
    UnexpectedUsedIndex { expected: u16, actual: u16 },
    UnexpectedDescriptor { id: u32 },
    InvalidCompletionLength(u32),
    InvalidNetworkHeader,
    InvalidBufferCount(u16),
    StaleCompletion,
    UnexpectedDeviceWrite(u32),
}

pub(crate) const fn frame_length_valid(length: usize) -> bool {
    length >= ETHERNET_HEADER_BYTES as usize && length <= ETHERNET_MAX_FRAME_BYTES as usize
}

pub(crate) fn write_descriptor(metadata: &mut [u8], packet_iova: u64, length: u32, flags: u16) {
    write_u64(metadata, VIRTIO_NET_DESCRIPTOR_OFFSET, packet_iova);
    write_u32(metadata, VIRTIO_NET_DESCRIPTOR_OFFSET + 8, length);
    write_u16(metadata, VIRTIO_NET_DESCRIPTOR_OFFSET + 12, flags);
    write_u16(metadata, VIRTIO_NET_DESCRIPTOR_OFFSET + 14, 0);
    write_u16(metadata, VIRTIO_NET_AVAILABLE_OFFSET + 4, DESCRIPTOR_INDEX);
}

pub(crate) fn publish_available(metadata: &mut [u8], index: u16) {
    write_u16(metadata, VIRTIO_NET_AVAILABLE_OFFSET + 2, index);
}

pub(crate) fn used_index(metadata: &[u8]) -> u16 {
    read_u16(metadata, usize::from(VIRTIO_NET_USED_OFFSET + 2))
}

pub(crate) fn used_descriptor(metadata: &[u8]) -> u32 {
    read_u32(metadata, VIRTIO_NET_USED_OFFSET + 4)
}

pub(crate) fn used_length(metadata: &[u8]) -> u32 {
    read_u32(metadata, VIRTIO_NET_USED_OFFSET + 8)
}

pub(crate) fn read_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

fn write_u16(bytes: &mut [u8], offset: u16, value: u16) {
    let offset = usize::from(offset);
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn write_u32(bytes: &mut [u8], offset: u16, value: u32) {
    let offset = usize::from(offset);
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_u64(bytes: &mut [u8], offset: u16, value: u64) {
    let offset = usize::from(offset);
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn read_u32(bytes: &[u8], offset: u16) -> u32 {
    let offset = usize::from(offset);
    u32::from_le_bytes(
        bytes[offset..offset + 4]
            .try_into()
            .expect("fixed split-ring field"),
    )
}
