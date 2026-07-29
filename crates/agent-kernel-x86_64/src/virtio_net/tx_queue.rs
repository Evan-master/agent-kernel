//! One-buffer Virtio network transmit queue.
//!
//! This owner prepends a zeroed 12-byte header, publishes one device-readable
//! descriptor, and requires a write-free used-ring completion.

use core::sync::atomic::{fence, Ordering};

use super::queue_layout::{
    frame_length_valid, publish_available, used_descriptor, used_index, used_length,
    write_descriptor, VirtioNetQueueError, VirtioNetQueueLayout, DESCRIPTOR_INDEX,
    QUEUE_PAGE_BYTES, VIRTIO_NET_HEADER_BYTES,
};

pub struct VirtioNetTxQueue<'a> {
    metadata: &'a mut [u8; QUEUE_PAGE_BYTES],
    packet: &'a mut [u8; QUEUE_PAGE_BYTES],
    layout: VirtioNetQueueLayout,
    last_used_index: u16,
    next_sequence: u64,
    active_sequence: Option<u64>,
}

impl<'a> VirtioNetTxQueue<'a> {
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
        }
    }

    pub fn metadata(&self) -> &[u8] {
        self.metadata
    }

    pub fn packet(&self) -> &[u8] {
        self.packet
    }

    pub const fn layout(&self) -> VirtioNetQueueLayout {
        self.layout
    }

    pub fn prepare_frame(
        &mut self,
        frame: &[u8],
    ) -> Result<VirtioNetTxRequest, VirtioNetQueueError> {
        if self.active_sequence.is_some() {
            return Err(VirtioNetQueueError::RequestOutstanding);
        }
        if !frame_length_valid(frame.len()) {
            return Err(VirtioNetQueueError::InvalidFrameLength(frame.len()));
        }
        let ether_type = u16::from_be_bytes([frame[12], frame[13]]);
        if ether_type < 0x0600 {
            return Err(VirtioNetQueueError::InvalidEtherType(ether_type));
        }
        let next_sequence = self
            .next_sequence
            .checked_add(1)
            .ok_or(VirtioNetQueueError::SequenceExhausted)?;
        let packet_len = VIRTIO_NET_HEADER_BYTES + frame.len();
        self.packet[..VIRTIO_NET_HEADER_BYTES].fill(0);
        self.packet[VIRTIO_NET_HEADER_BYTES..packet_len].copy_from_slice(frame);
        write_descriptor(
            self.metadata,
            self.layout.packet_iova(),
            packet_len as u32,
            0,
        );
        fence(Ordering::Release);
        let expected_used_index = self.last_used_index.wrapping_add(1);
        publish_available(self.metadata, expected_used_index);
        fence(Ordering::Release);

        let sequence = self.next_sequence;
        self.next_sequence = next_sequence;
        self.active_sequence = Some(sequence);
        Ok(VirtioNetTxRequest {
            sequence,
            expected_used_index,
            frame_len: frame.len() as u16,
        })
    }

    pub fn complete_frame(
        &mut self,
        request: VirtioNetTxRequest,
    ) -> Result<VirtioNetTxCompletion, VirtioNetQueueError> {
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
        let written = used_length(self.metadata);
        if written != 0 {
            return Err(VirtioNetQueueError::UnexpectedDeviceWrite(written));
        }
        fence(Ordering::Acquire);
        self.last_used_index = actual_index;
        self.active_sequence = None;
        Ok(VirtioNetTxCompletion {
            sequence: request.sequence,
            frame_len: request.frame_len,
        })
    }

    pub(super) fn reset_after_device_reset(&mut self) {
        self.metadata.fill(0);
        self.packet.fill(0);
        self.last_used_index = 0;
        self.active_sequence = None;
    }

    fn ensure_request(&self, sequence: u64) -> Result<(), VirtioNetQueueError> {
        match self.active_sequence {
            None => Err(VirtioNetQueueError::NoRequestOutstanding),
            Some(active) if active != sequence => Err(VirtioNetQueueError::RequestMismatch),
            Some(_) => Ok(()),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct VirtioNetTxRequest {
    sequence: u64,
    expected_used_index: u16,
    frame_len: u16,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct VirtioNetTxCompletion {
    sequence: u64,
    frame_len: u16,
}

impl VirtioNetTxCompletion {
    pub const fn frame_len(self) -> u16 {
        self.frame_len
    }
}
