//! Ordered modern virtio-rng device owner.
//!
//! This architecture driver composes the transport and DMA-visible queue,
//! enforcing one request at a time and the prepare, Notify, ISR, used-ring
//! completion order. Shutdown clears queue state only after device reset.

use super::{
    VirtioCommonConfigIo, VirtioIsrIo, VirtioNotifyIo, VirtioRngCompletion, VirtioRngQueueError,
    VirtioRngQueueLayout, VirtioRngQueueMemory, VirtioRngRequest, VirtioRngTransport,
    VirtioRngTransportError, VIRTIO_RNG_ENTROPY_BYTES,
};

pub struct VirtioRngDevice<'a, C, N, I> {
    transport: VirtioRngTransport<C, N, I>,
    queue: VirtioRngQueueMemory<'a>,
    pending: Option<PendingEntropyRequest>,
}

impl<'a, C: VirtioCommonConfigIo, N: VirtioNotifyIo, I: VirtioIsrIo> VirtioRngDevice<'a, C, N, I> {
    #[allow(clippy::too_many_arguments)]
    pub fn bind(
        common: C,
        notify: N,
        isr: I,
        poll_budget: u32,
        metadata: &'a mut [u8; 4096],
        entropy: &'a mut [u8; VIRTIO_RNG_ENTROPY_BYTES],
        layout: VirtioRngQueueLayout,
    ) -> Result<Self, VirtioRngDeviceError> {
        let transport = VirtioRngTransport::bind(common, notify, isr, poll_budget)
            .map_err(VirtioRngDeviceError::Transport)?;
        Ok(Self {
            transport,
            queue: VirtioRngQueueMemory::bind(metadata, entropy, layout),
            pending: None,
        })
    }

    pub fn initialize(&mut self, msix_vector: u16) -> Result<(), VirtioRngDeviceError> {
        self.transport
            .initialize(self.queue.layout(), msix_vector)
            .map_err(VirtioRngDeviceError::Transport)
    }

    pub fn request_entropy(&mut self, length: u32) -> Result<(), VirtioRngDeviceError> {
        self.prepare_entropy_request(length)?;
        self.notify_entropy_request()
    }

    pub fn prepare_entropy_request(&mut self, length: u32) -> Result<(), VirtioRngDeviceError> {
        if self.pending.is_some() {
            return Err(VirtioRngDeviceError::RequestPending);
        }
        let request = self
            .queue
            .prepare_request(length)
            .map_err(VirtioRngDeviceError::Queue)?;
        self.pending = Some(PendingEntropyRequest {
            request,
            notified: false,
        });
        Ok(())
    }

    pub fn notify_entropy_request(&mut self) -> Result<(), VirtioRngDeviceError> {
        let pending = self.pending.ok_or(VirtioRngDeviceError::NoRequestPending)?;
        if pending.notified {
            return Err(VirtioRngDeviceError::RequestAlreadyNotified);
        }
        self.transport
            .notify_queue()
            .map_err(VirtioRngDeviceError::Transport)?;
        self.pending
            .as_mut()
            .expect("pending request remains owned")
            .notified = true;
        Ok(())
    }

    pub fn complete_interrupt(&mut self) -> Result<VirtioRngCompletion, VirtioRngDeviceError> {
        let pending = self.pending.ok_or(VirtioRngDeviceError::NoRequestPending)?;
        if !pending.notified {
            return Err(VirtioRngDeviceError::RequestNotNotified);
        }
        let cause = self
            .transport
            .acknowledge_interrupt()
            .map_err(VirtioRngDeviceError::Transport)?;
        if !cause.queue_used() {
            return Err(VirtioRngDeviceError::QueueInterruptMissing);
        }
        let completion = self
            .queue
            .complete_request(pending.request)
            .map_err(VirtioRngDeviceError::Queue)?;
        self.pending = None;
        Ok(completion)
    }

    pub fn entropy<'b>(&'b self, completion: &VirtioRngCompletion) -> &'b [u8] {
        self.queue.entropy(completion)
    }

    pub fn shutdown(&mut self) -> Result<(), VirtioRngDeviceError> {
        self.transport
            .shutdown()
            .map_err(VirtioRngDeviceError::Transport)?;
        self.queue.reset_after_device_reset();
        self.pending = None;
        Ok(())
    }
}

#[derive(Copy, Clone)]
struct PendingEntropyRequest {
    request: VirtioRngRequest,
    notified: bool,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum VirtioRngDeviceError {
    Transport(VirtioRngTransportError),
    Queue(VirtioRngQueueError),
    RequestPending,
    NoRequestPending,
    RequestAlreadyNotified,
    RequestNotNotified,
    QueueInterruptMissing,
}
