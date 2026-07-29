//! One-buffer Virtio network receive queue.
//!
//! The queue publishes one device-writable modern-header buffer and exposes a
//! frame only after strict used-ring and header validation.

use core::sync::atomic::{fence, Ordering};

use super::queue_layout::{
    frame_length_valid, publish_available, read_u16, used_descriptor, used_index, used_length,
    write_descriptor, VirtioNetQueueError, VirtioNetQueueLayout, DESCRIPTOR_INDEX,
    DESCRIPTOR_WRITE, QUEUE_PAGE_BYTES, VIRTIO_NET_HEADER_BYTES, VIRTIO_NET_RX_BUFFER_BYTES,
};

pub struct VirtioNetRxQueue<'a> {
    metadata: &'a mut [u8; QUEUE_PAGE_BYTES],
    packet: &'a mut [u8; QUEUE_PAGE_BYTES],
    layout: VirtioNetQueueLayout,
    last_used_index: u16,
    next_sequence: u64,
    active_sequence: Option<u64>,
    completed_sequence: Option<u64>,
}

impl<'a> VirtioNetRxQueue<'a> {
    pub fn bind(
        metadata: &'a mut [u8; QUEUE_PAGE_BYTES],
        packet: &'a mut [u8; QUEUE_PAGE_BYTES],
        layout: VirtioNetQueueLayout,
    ) -> Self {
        metadata.fill(0);
        packet.fill(0);
        Self {
            metadata,
            packet,
            layout,
            last_used_index: 0,
            next_sequence: 1,
            active_sequence: None,
            completed_sequence: None,
        }
    }

    pub fn metadata(&self) -> &[u8] {
        self.metadata
    }

    pub const fn layout(&self) -> VirtioNetQueueLayout {
        self.layout
    }

    pub fn post_buffer(&mut self) -> Result<VirtioNetRxRequest, VirtioNetQueueError> {
        if self.active_sequence.is_some() {
            return Err(VirtioNetQueueError::RequestOutstanding);
        }
        let next_sequence = self
            .next_sequence
            .checked_add(1)
            .ok_or(VirtioNetQueueError::SequenceExhausted)?;
        self.completed_sequence = None;
        self.packet[..VIRTIO_NET_RX_BUFFER_BYTES].fill(0);
        write_descriptor(
            self.metadata,
            self.layout.packet_iova(),
            VIRTIO_NET_RX_BUFFER_BYTES as u32,
            DESCRIPTOR_WRITE,
        );
        fence(Ordering::Release);
        let expected_used_index = self.last_used_index.wrapping_add(1);
        publish_available(self.metadata, expected_used_index);
        fence(Ordering::Release);

        let sequence = self.next_sequence;
        self.next_sequence = next_sequence;
        self.active_sequence = Some(sequence);
        Ok(VirtioNetRxRequest {
            sequence,
            expected_used_index,
        })
    }

    pub fn complete_buffer(
        &mut self,
        request: VirtioNetRxRequest,
    ) -> Result<VirtioNetRxCompletion, VirtioNetQueueError> {
        self.ensure_request(request.sequence)?;
        fence(Ordering::Acquire);
        let actual_index = used_index(self.metadata);
        if actual_index == self.last_used_index {
            return Err(VirtioNetQueueError::CompletionPending);
        }
        if actual_index != request.expected_used_index {
            return Err(VirtioNetQueueError::UnexpectedUsedIndex {
                expected: request.expected_used_index,
                actual: actual_index,
            });
        }
        let id = used_descriptor(self.metadata);
        if id != u32::from(DESCRIPTOR_INDEX) {
            return Err(VirtioNetQueueError::UnexpectedDescriptor { id });
        }
        let used = used_length(self.metadata);
        let used = usize::try_from(used)
            .map_err(|_| VirtioNetQueueError::InvalidCompletionLength(used))?;
        if !(VIRTIO_NET_HEADER_BYTES + 14..=VIRTIO_NET_RX_BUFFER_BYTES).contains(&used) {
            return Err(VirtioNetQueueError::InvalidCompletionLength(used as u32));
        }
        validate_header(self.packet)?;
        let frame_len = used - VIRTIO_NET_HEADER_BYTES;
        if !frame_length_valid(frame_len) {
            return Err(VirtioNetQueueError::InvalidFrameLength(frame_len));
        }
        let ether_type = read_u16(self.packet, VIRTIO_NET_HEADER_BYTES + 12).swap_bytes();
        if ether_type < 0x0600 {
            return Err(VirtioNetQueueError::InvalidEtherType(ether_type));
        }
        fence(Ordering::Acquire);
        self.last_used_index = actual_index;
        self.active_sequence = None;
        self.completed_sequence = Some(request.sequence);
        Ok(VirtioNetRxCompletion {
            sequence: request.sequence,
            frame_len: frame_len as u16,
            ether_type,
        })
    }

    pub fn frame<'b>(
        &'b self,
        completion: &VirtioNetRxCompletion,
    ) -> Result<&'b [u8], VirtioNetQueueError> {
        if self.completed_sequence != Some(completion.sequence) {
            return Err(VirtioNetQueueError::StaleCompletion);
        }
        Ok(&self.packet
            [VIRTIO_NET_HEADER_BYTES..VIRTIO_NET_HEADER_BYTES + usize::from(completion.frame_len)])
    }

    pub(super) fn reset_after_device_reset(&mut self) {
        self.metadata.fill(0);
        self.packet.fill(0);
        self.last_used_index = 0;
        self.active_sequence = None;
        self.completed_sequence = None;
    }

    fn ensure_request(&self, sequence: u64) -> Result<(), VirtioNetQueueError> {
        match self.active_sequence {
            None => Err(VirtioNetQueueError::NoRequestOutstanding),
            Some(active) if active != sequence => Err(VirtioNetQueueError::RequestMismatch),
            Some(_) => Ok(()),
        }
    }
}

fn validate_header(packet: &[u8]) -> Result<(), VirtioNetQueueError> {
    if packet[0..10].iter().any(|byte| *byte != 0) {
        return Err(VirtioNetQueueError::InvalidNetworkHeader);
    }
    let buffers = read_u16(packet, 10);
    if buffers != 1 {
        return Err(VirtioNetQueueError::InvalidBufferCount(buffers));
    }
    Ok(())
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct VirtioNetRxRequest {
    sequence: u64,
    expected_used_index: u16,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct VirtioNetRxCompletion {
    sequence: u64,
    frame_len: u16,
    ether_type: u16,
}

impl VirtioNetRxCompletion {
    pub const fn frame_len(self) -> u16 {
        self.frame_len
    }

    pub const fn ether_type(self) -> u16 {
        self.ether_type
    }
}
