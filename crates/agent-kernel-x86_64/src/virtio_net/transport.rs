//! Strict modern Virtio PCI network transport.
//!
//! The transport negotiates the minimal network feature set, configures
//! receive queue zero and transmit queue one, and retains bounded Notify
//! offsets for each queue.

use core::sync::atomic::{fence, Ordering};

use crate::virtio_rng::{VirtioCommonConfigIo, VirtioIsrIo, VirtioNotifyIo};

use super::{
    transport_queue::configure_queue,
    transport_types::{
        ensure_no_device_fault, VirtioNetInterrupt, VirtioNetTransportError, DEVICE_FEATURE,
        DEVICE_FEATURE_SELECT, DEVICE_STATUS, DRIVER_FEATURE, DRIVER_FEATURE_SELECT, NUM_QUEUES,
        REQUIRED_FEATURES, VIRTIO_PCI_STATUS_ACKNOWLEDGE, VIRTIO_PCI_STATUS_DRIVER,
        VIRTIO_PCI_STATUS_DRIVER_OK, VIRTIO_PCI_STATUS_FAILED, VIRTIO_PCI_STATUS_FEATURES_OK,
    },
    VirtioNetQueueLayout,
};

const RX_QUEUE: u16 = 0;
const TX_QUEUE: u16 = 1;
const NO_MSIX_VECTOR: u16 = u16::MAX;

pub struct VirtioNetTransport<C, N, I> {
    common: C,
    notify: N,
    isr: I,
    poll_budget: u32,
    notify_offsets: [Option<u32>; 2],
    ready: bool,
}

impl<C: VirtioCommonConfigIo, N: VirtioNotifyIo, I: VirtioIsrIo> VirtioNetTransport<C, N, I> {
    pub fn bind(
        common: C,
        notify: N,
        isr: I,
        poll_budget: u32,
    ) -> Result<Self, VirtioNetTransportError> {
        if poll_budget == 0 {
            return Err(VirtioNetTransportError::InvalidPollBudget);
        }
        Ok(Self {
            common,
            notify,
            isr,
            poll_budget,
            notify_offsets: [None; 2],
            ready: false,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn initialize(
        &mut self,
        rx: VirtioNetQueueLayout,
        tx: VirtioNetQueueLayout,
        rx_msix_vector: u16,
        tx_msix_vector: u16,
    ) -> Result<(), VirtioNetTransportError> {
        if self.ready {
            return Err(VirtioNetTransportError::AlreadyInitialized);
        }
        let result = self.initialize_inner(rx, tx, rx_msix_vector, tx_msix_vector);
        if result.is_err() {
            self.mark_failed();
        }
        result
    }

    pub fn notify_receive(&mut self) -> Result<(), VirtioNetTransportError> {
        self.notify_queue(RX_QUEUE)
    }

    pub fn notify_transmit(&mut self) -> Result<(), VirtioNetTransportError> {
        self.notify_queue(TX_QUEUE)
    }

    pub fn acknowledge_interrupt(&mut self) -> Result<VirtioNetInterrupt, VirtioNetTransportError> {
        if !self.ready {
            return Err(VirtioNetTransportError::NotInitialized);
        }
        let status = self.isr.read_and_acknowledge();
        if status & !3 != 0 {
            return Err(VirtioNetTransportError::InvalidIsrStatus(status));
        }
        if status == 0 {
            return Err(VirtioNetTransportError::SpuriousInterrupt);
        }
        fence(Ordering::Acquire);
        Ok(VirtioNetInterrupt {
            queue_used: status & 1 != 0,
            configuration_changed: status & 2 != 0,
        })
    }

    pub fn shutdown(&mut self) -> Result<(), VirtioNetTransportError> {
        self.ready = false;
        self.notify_offsets = [None; 2];
        self.reset_device()
    }

    pub fn into_parts(self) -> (C, N, I) {
        (self.common, self.notify, self.isr)
    }

    fn initialize_inner(
        &mut self,
        rx: VirtioNetQueueLayout,
        tx: VirtioNetQueueLayout,
        rx_msix_vector: u16,
        tx_msix_vector: u16,
    ) -> Result<(), VirtioNetTransportError> {
        if rx_msix_vector == NO_MSIX_VECTOR || tx_msix_vector == NO_MSIX_VECTOR {
            return Err(VirtioNetTransportError::InvalidMsixVector);
        }
        if rx_msix_vector == tx_msix_vector {
            return Err(VirtioNetTransportError::DuplicateMsixVector);
        }
        self.reset_device()?;
        let mut status = VIRTIO_PCI_STATUS_ACKNOWLEDGE;
        self.common.write_u8(DEVICE_STATUS, status);
        status |= VIRTIO_PCI_STATUS_DRIVER;
        self.common.write_u8(DEVICE_STATUS, status);

        let observed = self.read_device_features();
        if observed & REQUIRED_FEATURES != REQUIRED_FEATURES {
            return Err(VirtioNetTransportError::MissingRequiredFeatures {
                required: REQUIRED_FEATURES,
                observed,
            });
        }
        self.write_driver_features(REQUIRED_FEATURES);
        status |= VIRTIO_PCI_STATUS_FEATURES_OK;
        self.common.write_u8(DEVICE_STATUS, status);
        let accepted = self.common.read_u8(DEVICE_STATUS);
        if accepted & VIRTIO_PCI_STATUS_FEATURES_OK == 0 {
            return Err(VirtioNetTransportError::FeaturesRejected);
        }
        ensure_no_device_fault(accepted)?;

        let queue_count = self.common.read_u16(NUM_QUEUES);
        let rx_notify = configure_queue(
            &mut self.common,
            &self.notify,
            RX_QUEUE,
            queue_count,
            rx,
            rx_msix_vector,
        )?;
        let tx_notify = configure_queue(
            &mut self.common,
            &self.notify,
            TX_QUEUE,
            queue_count,
            tx,
            tx_msix_vector,
        )?;
        status |= VIRTIO_PCI_STATUS_DRIVER_OK;
        self.common.write_u8(DEVICE_STATUS, status);
        let running = self.common.read_u8(DEVICE_STATUS);
        ensure_no_device_fault(running)?;
        if running & VIRTIO_PCI_STATUS_DRIVER_OK == 0 {
            return Err(VirtioNetTransportError::DriverStatusRejected);
        }
        self.notify_offsets = [Some(rx_notify), Some(tx_notify)];
        self.ready = true;
        Ok(())
    }

    fn notify_queue(&mut self, queue: u16) -> Result<(), VirtioNetTransportError> {
        let offset = self.notify_offsets[usize::from(queue)]
            .ok_or(VirtioNetTransportError::NotInitialized)?;
        self.ensure_device_running()?;
        fence(Ordering::Release);
        self.notify.write_u16(offset, queue);
        Ok(())
    }

    fn read_device_features(&mut self) -> u64 {
        self.common.write_u32(DEVICE_FEATURE_SELECT, 0);
        let low = self.common.read_u32(DEVICE_FEATURE);
        self.common.write_u32(DEVICE_FEATURE_SELECT, 1);
        let high = self.common.read_u32(DEVICE_FEATURE);
        u64::from(low) | (u64::from(high) << 32)
    }

    fn write_driver_features(&mut self, features: u64) {
        self.common.write_u32(DRIVER_FEATURE_SELECT, 0);
        self.common.write_u32(DRIVER_FEATURE, features as u32);
        self.common.write_u32(DRIVER_FEATURE_SELECT, 1);
        self.common
            .write_u32(DRIVER_FEATURE, (features >> 32) as u32);
    }

    fn ensure_device_running(&mut self) -> Result<(), VirtioNetTransportError> {
        let status = self.common.read_u8(DEVICE_STATUS);
        ensure_no_device_fault(status)?;
        if status & VIRTIO_PCI_STATUS_DRIVER_OK == 0 {
            return Err(VirtioNetTransportError::NotInitialized);
        }
        Ok(())
    }

    fn reset_device(&mut self) -> Result<(), VirtioNetTransportError> {
        self.common.write_u8(DEVICE_STATUS, 0);
        for _ in 0..self.poll_budget {
            if self.common.read_u8(DEVICE_STATUS) == 0 {
                return Ok(());
            }
            core::hint::spin_loop();
        }
        Err(VirtioNetTransportError::ResetTimeout)
    }

    fn mark_failed(&mut self) {
        let status = self.common.read_u8(DEVICE_STATUS);
        self.common
            .write_u8(DEVICE_STATUS, status | VIRTIO_PCI_STATUS_FAILED);
        self.ready = false;
        self.notify_offsets = [None; 2];
    }
}
