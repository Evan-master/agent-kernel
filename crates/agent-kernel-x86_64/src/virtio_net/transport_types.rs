//! Virtio network transport constants, observations, and typed failures.
//!
//! This allocation-free x86 protocol surface centralizes stable PCI identity,
//! feature bits, status bits, and validation results shared by the transport.

pub const VIRTIO_NET_VENDOR_ID: u16 = 0x1af4;
pub const VIRTIO_NET_DEVICE_ID: u16 = 0x1041;
pub const VIRTIO_NET_F_MAC: u64 = 1 << 5;
pub const VIRTIO_NET_F_MRG_RXBUF: u64 = 1 << 15;
pub const VIRTIO_F_VERSION_1: u64 = 1 << 32;
pub const VIRTIO_F_ACCESS_PLATFORM: u64 = 1 << 33;
pub const VIRTIO_PCI_STATUS_ACKNOWLEDGE: u8 = 1;
pub const VIRTIO_PCI_STATUS_DRIVER: u8 = 2;
pub const VIRTIO_PCI_STATUS_DRIVER_OK: u8 = 4;
pub const VIRTIO_PCI_STATUS_FEATURES_OK: u8 = 8;
pub const VIRTIO_PCI_STATUS_DEVICE_NEEDS_RESET: u8 = 64;
pub const VIRTIO_PCI_STATUS_FAILED: u8 = 128;

pub(super) const REQUIRED_FEATURES: u64 =
    VIRTIO_NET_F_MAC | VIRTIO_NET_F_MRG_RXBUF | VIRTIO_F_VERSION_1 | VIRTIO_F_ACCESS_PLATFORM;
pub(super) const DEVICE_FEATURE_SELECT: u16 = 0x00;
pub(super) const DEVICE_FEATURE: u16 = 0x04;
pub(super) const DRIVER_FEATURE_SELECT: u16 = 0x08;
pub(super) const DRIVER_FEATURE: u16 = 0x0c;
pub(super) const NUM_QUEUES: u16 = 0x12;
pub(super) const DEVICE_STATUS: u16 = 0x14;

pub(super) fn ensure_no_device_fault(status: u8) -> Result<(), VirtioNetTransportError> {
    if status & VIRTIO_PCI_STATUS_DEVICE_NEEDS_RESET != 0 {
        Err(VirtioNetTransportError::DeviceNeedsReset)
    } else if status & VIRTIO_PCI_STATUS_FAILED != 0 {
        Err(VirtioNetTransportError::DeviceFailed)
    } else {
        Ok(())
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct VirtioNetInterrupt {
    pub(super) queue_used: bool,
    pub(super) configuration_changed: bool,
}

impl VirtioNetInterrupt {
    pub const fn queue_used(self) -> bool {
        self.queue_used
    }

    pub const fn configuration_changed(self) -> bool {
        self.configuration_changed
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum VirtioNetTransportError {
    InvalidPollBudget,
    AlreadyInitialized,
    NotInitialized,
    InvalidMsixVector,
    DuplicateMsixVector,
    MissingRequiredFeatures {
        required: u64,
        observed: u64,
    },
    FeaturesRejected,
    QueueUnavailable(u16),
    QueueAlreadyEnabled(u16),
    QueueSizeRejected {
        queue: u16,
        expected: u16,
        actual: u16,
    },
    QueueVectorRejected {
        queue: u16,
        expected: u16,
        actual: u16,
    },
    NotifyOffsetOverflow(u16),
    NotifyOutsideRegion {
        queue: u16,
        offset: u32,
        region_bytes: u32,
    },
    QueueEnableRejected(u16),
    DriverStatusRejected,
    DeviceNeedsReset,
    DeviceFailed,
    InvalidIsrStatus(u8),
    SpuriousInterrupt,
    ResetTimeout,
}
