//! Modern Virtio PCI entropy transport state machine.
//!
//! This architecture module resets the device, negotiates Version 1 plus
//! Access Platform, configures split queue zero and its MSI-X vector, validates
//! Notify bounds, and acknowledges ISR causes through width-aware adapters.

use core::sync::atomic::{fence, Ordering};

use super::VirtioRngQueueLayout;

pub const VIRTIO_RNG_VENDOR_ID: u16 = 0x1af4;
pub const VIRTIO_RNG_DEVICE_ID: u16 = 0x1044;
pub const VIRTIO_F_VERSION_1: u64 = 1 << 32;
pub const VIRTIO_F_ACCESS_PLATFORM: u64 = 1 << 33;
pub const VIRTIO_PCI_STATUS_ACKNOWLEDGE: u8 = 1;
pub const VIRTIO_PCI_STATUS_DRIVER: u8 = 2;
pub const VIRTIO_PCI_STATUS_DRIVER_OK: u8 = 4;
pub const VIRTIO_PCI_STATUS_FEATURES_OK: u8 = 8;
pub const VIRTIO_PCI_STATUS_DEVICE_NEEDS_RESET: u8 = 64;
pub const VIRTIO_PCI_STATUS_FAILED: u8 = 128;

const REQUIRED_FEATURES: u64 = VIRTIO_F_VERSION_1 | VIRTIO_F_ACCESS_PLATFORM;
const DEVICE_FEATURE_SELECT: u16 = 0x00;
const DEVICE_FEATURE: u16 = 0x04;
const DRIVER_FEATURE_SELECT: u16 = 0x08;
const DRIVER_FEATURE: u16 = 0x0c;
const NUM_QUEUES: u16 = 0x12;
const DEVICE_STATUS: u16 = 0x14;
const QUEUE_SELECT: u16 = 0x16;
const QUEUE_SIZE: u16 = 0x18;
const QUEUE_MSIX_VECTOR: u16 = 0x1a;
const QUEUE_ENABLE: u16 = 0x1c;
const QUEUE_NOTIFY_OFFSET: u16 = 0x1e;
const QUEUE_DESCRIPTOR: u16 = 0x20;
const QUEUE_DRIVER: u16 = 0x28;
const QUEUE_DEVICE: u16 = 0x30;
const QUEUE_INDEX: u16 = 0;
const QUEUE_ENTRY_COUNT: u16 = 1;
const NO_MSIX_VECTOR: u16 = u16::MAX;

pub trait VirtioCommonConfigIo {
    fn read_u8(&mut self, offset: u16) -> u8;
    fn read_u16(&mut self, offset: u16) -> u16;
    fn read_u32(&mut self, offset: u16) -> u32;
    fn write_u8(&mut self, offset: u16, value: u8);
    fn write_u16(&mut self, offset: u16, value: u16);
    fn write_u32(&mut self, offset: u16, value: u32);
    fn write_u64(&mut self, offset: u16, value: u64);
}

pub trait VirtioNotifyIo {
    fn region_bytes(&self) -> u32;
    fn offset_multiplier(&self) -> u32;
    fn write_u16(&mut self, byte_offset: u32, value: u16);
}

pub trait VirtioIsrIo {
    fn read_and_acknowledge(&mut self) -> u8;
}

pub struct VirtioRngTransport<C, N, I> {
    common: C,
    notify: N,
    isr: I,
    poll_budget: u32,
    notify_offset: Option<u32>,
    ready: bool,
}

impl<C: VirtioCommonConfigIo, N: VirtioNotifyIo, I: VirtioIsrIo> VirtioRngTransport<C, N, I> {
    pub fn bind(
        common: C,
        notify: N,
        isr: I,
        poll_budget: u32,
    ) -> Result<Self, VirtioRngTransportError> {
        if poll_budget == 0 {
            return Err(VirtioRngTransportError::InvalidPollBudget);
        }
        Ok(Self {
            common,
            notify,
            isr,
            poll_budget,
            notify_offset: None,
            ready: false,
        })
    }

    pub fn initialize(
        &mut self,
        layout: VirtioRngQueueLayout,
        msix_vector: u16,
    ) -> Result<(), VirtioRngTransportError> {
        if self.ready {
            return Err(VirtioRngTransportError::AlreadyInitialized);
        }
        let result = self.initialize_inner(layout, msix_vector);
        if result.is_err() {
            self.mark_failed();
        }
        result
    }

    pub fn notify_queue(&mut self) -> Result<(), VirtioRngTransportError> {
        let offset = self
            .notify_offset
            .ok_or(VirtioRngTransportError::NotInitialized)?;
        self.ensure_device_running()?;
        fence(Ordering::Release);
        self.notify.write_u16(offset, QUEUE_INDEX);
        Ok(())
    }

    pub fn acknowledge_interrupt(&mut self) -> Result<VirtioRngInterrupt, VirtioRngTransportError> {
        if !self.ready {
            return Err(VirtioRngTransportError::NotInitialized);
        }
        let status = self.isr.read_and_acknowledge();
        if status & !3 != 0 {
            return Err(VirtioRngTransportError::InvalidIsrStatus(status));
        }
        if status == 0 {
            return Err(VirtioRngTransportError::SpuriousInterrupt);
        }
        fence(Ordering::Acquire);
        Ok(VirtioRngInterrupt {
            queue_used: status & 1 != 0,
            configuration_changed: status & 2 != 0,
        })
    }

    pub fn shutdown(&mut self) -> Result<(), VirtioRngTransportError> {
        self.ready = false;
        self.notify_offset = None;
        self.reset_device()
    }

    pub fn into_parts(self) -> (C, N, I) {
        (self.common, self.notify, self.isr)
    }

    fn initialize_inner(
        &mut self,
        layout: VirtioRngQueueLayout,
        msix_vector: u16,
    ) -> Result<(), VirtioRngTransportError> {
        if msix_vector == NO_MSIX_VECTOR {
            return Err(VirtioRngTransportError::InvalidMsixVector);
        }
        self.reset_device()?;

        let mut status = VIRTIO_PCI_STATUS_ACKNOWLEDGE;
        self.common.write_u8(DEVICE_STATUS, status);
        status |= VIRTIO_PCI_STATUS_DRIVER;
        self.common.write_u8(DEVICE_STATUS, status);

        let observed = self.read_device_features();
        if observed & REQUIRED_FEATURES != REQUIRED_FEATURES {
            return Err(VirtioRngTransportError::MissingRequiredFeatures {
                required: REQUIRED_FEATURES,
                observed,
            });
        }
        self.write_driver_features(REQUIRED_FEATURES);
        status |= VIRTIO_PCI_STATUS_FEATURES_OK;
        self.common.write_u8(DEVICE_STATUS, status);
        let accepted = self.common.read_u8(DEVICE_STATUS);
        if accepted & VIRTIO_PCI_STATUS_FEATURES_OK == 0 {
            return Err(VirtioRngTransportError::FeaturesRejected);
        }
        ensure_no_device_fault(accepted)?;

        if self.common.read_u16(NUM_QUEUES) == 0 {
            return Err(VirtioRngTransportError::QueueUnavailable);
        }
        self.common.write_u16(QUEUE_SELECT, QUEUE_INDEX);
        if self.common.read_u16(QUEUE_ENABLE) != 0 {
            return Err(VirtioRngTransportError::QueueAlreadyEnabled);
        }
        if self.common.read_u16(QUEUE_SIZE) < QUEUE_ENTRY_COUNT {
            return Err(VirtioRngTransportError::QueueUnavailable);
        }
        self.common.write_u16(QUEUE_SIZE, QUEUE_ENTRY_COUNT);
        let queue_size = self.common.read_u16(QUEUE_SIZE);
        if queue_size != QUEUE_ENTRY_COUNT {
            return Err(VirtioRngTransportError::QueueSizeRejected {
                expected: QUEUE_ENTRY_COUNT,
                actual: queue_size,
            });
        }
        self.common.write_u16(QUEUE_MSIX_VECTOR, msix_vector);
        let actual_vector = self.common.read_u16(QUEUE_MSIX_VECTOR);
        if actual_vector != msix_vector {
            return Err(VirtioRngTransportError::QueueVectorRejected {
                expected: msix_vector,
                actual: actual_vector,
            });
        }
        self.common
            .write_u64(QUEUE_DESCRIPTOR, layout.descriptor_iova());
        self.common.write_u64(QUEUE_DRIVER, layout.driver_iova());
        self.common.write_u64(QUEUE_DEVICE, layout.device_iova());

        let notify_index = u32::from(self.common.read_u16(QUEUE_NOTIFY_OFFSET));
        let notify_offset = notify_index
            .checked_mul(self.notify.offset_multiplier())
            .ok_or(VirtioRngTransportError::NotifyOffsetOverflow)?;
        let region_bytes = self.notify.region_bytes();
        if notify_offset
            .checked_add(2)
            .is_none_or(|end| end > region_bytes)
        {
            return Err(VirtioRngTransportError::NotifyOutsideRegion {
                offset: notify_offset,
                region_bytes,
            });
        }
        self.common.write_u16(QUEUE_ENABLE, 1);
        if self.common.read_u16(QUEUE_ENABLE) != 1 {
            return Err(VirtioRngTransportError::QueueEnableRejected);
        }

        status |= VIRTIO_PCI_STATUS_DRIVER_OK;
        self.common.write_u8(DEVICE_STATUS, status);
        let running = self.common.read_u8(DEVICE_STATUS);
        ensure_no_device_fault(running)?;
        if running & VIRTIO_PCI_STATUS_DRIVER_OK == 0 {
            return Err(VirtioRngTransportError::DriverStatusRejected);
        }
        self.notify_offset = Some(notify_offset);
        self.ready = true;
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

    fn ensure_device_running(&mut self) -> Result<(), VirtioRngTransportError> {
        let status = self.common.read_u8(DEVICE_STATUS);
        ensure_no_device_fault(status)?;
        if status & VIRTIO_PCI_STATUS_DRIVER_OK == 0 {
            return Err(VirtioRngTransportError::NotInitialized);
        }
        Ok(())
    }

    fn reset_device(&mut self) -> Result<(), VirtioRngTransportError> {
        self.common.write_u8(DEVICE_STATUS, 0);
        for _ in 0..self.poll_budget {
            if self.common.read_u8(DEVICE_STATUS) == 0 {
                return Ok(());
            }
            core::hint::spin_loop();
        }
        Err(VirtioRngTransportError::ResetTimeout)
    }

    fn mark_failed(&mut self) {
        let status = self.common.read_u8(DEVICE_STATUS);
        self.common
            .write_u8(DEVICE_STATUS, status | VIRTIO_PCI_STATUS_FAILED);
        self.ready = false;
        self.notify_offset = None;
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct VirtioRngInterrupt {
    queue_used: bool,
    configuration_changed: bool,
}

impl VirtioRngInterrupt {
    pub const fn queue_used(self) -> bool {
        self.queue_used
    }

    pub const fn configuration_changed(self) -> bool {
        self.configuration_changed
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum VirtioRngTransportError {
    InvalidPollBudget,
    AlreadyInitialized,
    NotInitialized,
    InvalidMsixVector,
    ResetTimeout,
    MissingRequiredFeatures { required: u64, observed: u64 },
    FeaturesRejected,
    DeviceNeedsReset,
    DeviceFailed,
    QueueUnavailable,
    QueueAlreadyEnabled,
    QueueSizeRejected { expected: u16, actual: u16 },
    QueueVectorRejected { expected: u16, actual: u16 },
    NotifyOffsetOverflow,
    NotifyOutsideRegion { offset: u32, region_bytes: u32 },
    QueueEnableRejected,
    DriverStatusRejected,
    InvalidIsrStatus(u8),
    SpuriousInterrupt,
}

fn ensure_no_device_fault(status: u8) -> Result<(), VirtioRngTransportError> {
    if status & VIRTIO_PCI_STATUS_DEVICE_NEEDS_RESET != 0 {
        return Err(VirtioRngTransportError::DeviceNeedsReset);
    }
    if status & VIRTIO_PCI_STATUS_FAILED != 0 {
        return Err(VirtioRngTransportError::DeviceFailed);
    }
    Ok(())
}
