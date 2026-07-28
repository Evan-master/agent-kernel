//! Fixed one-descriptor split virtqueue and entropy buffer owner.
//!
//! This architecture module encodes DMA-visible little-endian structures,
//! publishes one device-writable request with a release fence, and validates
//! the used ring under an acquire fence before exposing entropy bytes.

use core::sync::atomic::{fence, Ordering};

use agent_kernel_core::DMA_PAGE_BYTES;

pub const VIRTIO_RNG_DESCRIPTOR_OFFSET: u16 = 0x000;
pub const VIRTIO_RNG_AVAILABLE_OFFSET: u16 = 0x100;
pub const VIRTIO_RNG_USED_OFFSET: u16 = 0x200;
pub const VIRTIO_RNG_ENTROPY_BYTES: usize = DMA_PAGE_BYTES as usize;

const QUEUE_PAGE_BYTES: usize = DMA_PAGE_BYTES as usize;
const DESCRIPTOR_WRITE: u16 = 2;
const DESCRIPTOR_INDEX: u16 = 0;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct VirtioRngQueueLayout {
    metadata_iova: u64,
    entropy_iova: u64,
}

impl VirtioRngQueueLayout {
    pub const fn new(metadata_iova: u64, entropy_iova: u64) -> Result<Self, VirtioRngQueueError> {
        if metadata_iova & (DMA_PAGE_BYTES - 1) != 0 {
            return Err(VirtioRngQueueError::PageMisaligned(metadata_iova));
        }
        if entropy_iova & (DMA_PAGE_BYTES - 1) != 0 {
            return Err(VirtioRngQueueError::PageMisaligned(entropy_iova));
        }
        if metadata_iova == entropy_iova {
            return Err(VirtioRngQueueError::AliasedPages);
        }
        if metadata_iova.checked_add(DMA_PAGE_BYTES).is_none()
            || entropy_iova.checked_add(DMA_PAGE_BYTES).is_none()
        {
            return Err(VirtioRngQueueError::AddressOverflow);
        }
        Ok(Self {
            metadata_iova,
            entropy_iova,
        })
    }

    pub const fn descriptor_iova(self) -> u64 {
        self.metadata_iova + VIRTIO_RNG_DESCRIPTOR_OFFSET as u64
    }

    pub const fn driver_iova(self) -> u64 {
        self.metadata_iova + VIRTIO_RNG_AVAILABLE_OFFSET as u64
    }

    pub const fn device_iova(self) -> u64 {
        self.metadata_iova + VIRTIO_RNG_USED_OFFSET as u64
    }

    pub const fn entropy_iova(self) -> u64 {
        self.entropy_iova
    }
}

pub struct VirtioRngQueueMemory<'a> {
    metadata: &'a mut [u8; QUEUE_PAGE_BYTES],
    entropy: &'a mut [u8; VIRTIO_RNG_ENTROPY_BYTES],
    layout: VirtioRngQueueLayout,
    last_used_index: u16,
    next_sequence: u64,
    active_sequence: Option<u64>,
}

impl<'a> VirtioRngQueueMemory<'a> {
    pub fn bind(
        metadata: &'a mut [u8; QUEUE_PAGE_BYTES],
        entropy: &'a mut [u8; VIRTIO_RNG_ENTROPY_BYTES],
        layout: VirtioRngQueueLayout,
    ) -> Self {
        metadata.fill(0);
        entropy.fill(0);
        Self {
            metadata,
            entropy,
            layout,
            last_used_index: 0,
            next_sequence: 1,
            active_sequence: None,
        }
    }

    pub fn metadata(&self) -> &[u8] {
        self.metadata
    }

    pub fn entropy_page(&self) -> &[u8] {
        self.entropy
    }

    pub const fn layout(&self) -> VirtioRngQueueLayout {
        self.layout
    }

    pub fn prepare_request(
        &mut self,
        requested_len: u32,
    ) -> Result<VirtioRngRequest, VirtioRngQueueError> {
        if self.active_sequence.is_some() {
            return Err(VirtioRngQueueError::RequestOutstanding);
        }
        if requested_len == 0 || requested_len as usize > VIRTIO_RNG_ENTROPY_BYTES {
            return Err(VirtioRngQueueError::InvalidRequestLength(requested_len));
        }
        let next_sequence = self
            .next_sequence
            .checked_add(1)
            .ok_or(VirtioRngQueueError::SequenceExhausted)?;
        self.entropy[..requested_len as usize].fill(0);
        write_u64(
            self.metadata,
            VIRTIO_RNG_DESCRIPTOR_OFFSET,
            self.layout.entropy_iova(),
        );
        write_u32(
            self.metadata,
            VIRTIO_RNG_DESCRIPTOR_OFFSET + 8,
            requested_len,
        );
        write_u16(
            self.metadata,
            VIRTIO_RNG_DESCRIPTOR_OFFSET + 12,
            DESCRIPTOR_WRITE,
        );
        write_u16(self.metadata, VIRTIO_RNG_DESCRIPTOR_OFFSET + 14, 0);
        write_u16(
            self.metadata,
            VIRTIO_RNG_AVAILABLE_OFFSET + 4,
            DESCRIPTOR_INDEX,
        );
        fence(Ordering::Release);
        let expected_used_index = self.last_used_index.wrapping_add(1);
        write_u16(
            self.metadata,
            VIRTIO_RNG_AVAILABLE_OFFSET + 2,
            expected_used_index,
        );
        fence(Ordering::Release);

        let sequence = self.next_sequence;
        self.next_sequence = next_sequence;
        self.active_sequence = Some(sequence);
        Ok(VirtioRngRequest {
            sequence,
            expected_used_index,
            requested_len,
        })
    }

    pub fn complete_request(
        &mut self,
        request: VirtioRngRequest,
    ) -> Result<VirtioRngCompletion, VirtioRngQueueError> {
        let Some(active_sequence) = self.active_sequence else {
            return Err(VirtioRngQueueError::NoRequestOutstanding);
        };
        if active_sequence != request.sequence {
            return Err(VirtioRngQueueError::RequestMismatch);
        }
        fence(Ordering::Acquire);
        let actual_used_index = read_u16(self.metadata, VIRTIO_RNG_USED_OFFSET + 2);
        if actual_used_index == self.last_used_index {
            return Err(VirtioRngQueueError::CompletionPending);
        }
        if actual_used_index != request.expected_used_index {
            return Err(VirtioRngQueueError::UnexpectedUsedIndex {
                expected: request.expected_used_index,
                actual: actual_used_index,
            });
        }
        let id = read_u32(self.metadata, VIRTIO_RNG_USED_OFFSET + 4);
        if id != u32::from(DESCRIPTOR_INDEX) {
            return Err(VirtioRngQueueError::UnexpectedDescriptor { id });
        }
        let len = read_u32(self.metadata, VIRTIO_RNG_USED_OFFSET + 8);
        if len == 0 || len > request.requested_len {
            return Err(VirtioRngQueueError::InvalidCompletionLength {
                requested: request.requested_len,
                actual: len,
            });
        }
        fence(Ordering::Acquire);
        self.last_used_index = actual_used_index;
        self.active_sequence = None;
        Ok(VirtioRngCompletion {
            sequence: request.sequence,
            len,
        })
    }

    pub fn entropy<'b>(&'b self, completion: &VirtioRngCompletion) -> &'b [u8] {
        let _ = completion.sequence;
        &self.entropy[..completion.len as usize]
    }

    pub(super) fn reset_after_device_reset(&mut self) {
        self.metadata.fill(0);
        self.entropy.fill(0);
        self.last_used_index = 0;
        self.active_sequence = None;
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct VirtioRngRequest {
    sequence: u64,
    expected_used_index: u16,
    requested_len: u32,
}

impl VirtioRngRequest {
    pub const fn requested_len(self) -> u32 {
        self.requested_len
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct VirtioRngCompletion {
    sequence: u64,
    len: u32,
}

impl VirtioRngCompletion {
    pub const fn len(self) -> u32 {
        self.len
    }

    pub const fn is_empty(self) -> bool {
        self.len == 0
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum VirtioRngQueueError {
    PageMisaligned(u64),
    AliasedPages,
    AddressOverflow,
    InvalidRequestLength(u32),
    RequestOutstanding,
    NoRequestOutstanding,
    RequestMismatch,
    SequenceExhausted,
    CompletionPending,
    UnexpectedUsedIndex { expected: u16, actual: u16 },
    UnexpectedDescriptor { id: u32 },
    InvalidCompletionLength { requested: u32, actual: u32 },
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

fn read_u16(bytes: &[u8], offset: u16) -> u16 {
    let offset = usize::from(offset);
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

fn read_u32(bytes: &[u8], offset: u16) -> u32 {
    let offset = usize::from(offset);
    u32::from_le_bytes(
        bytes[offset..offset + 4]
            .try_into()
            .expect("fixed queue field"),
    )
}
