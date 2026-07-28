//! Architecture-neutral message-interrupt route records.
//!
//! This Core module models Capability-bound Device routes without retaining
//! PCI configuration offsets, APIC addresses, MMIO pointers, or controller
//! register values. Hardware owners pair these records with architecture state.

use crate::ResourceId;

pub const INTERRUPT_DEVICE_VECTOR_MIN: u8 = 0x20;
pub const INTERRUPT_DEVICE_VECTOR_MAX: u8 = 0xdf;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum InterruptMode {
    Msi,
    MsiX { table_entry: u16 },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct InterruptTarget {
    destination: u32,
    vector: u8,
}

impl InterruptTarget {
    pub const fn new(destination: u32, vector: u8) -> Option<Self> {
        if vector < INTERRUPT_DEVICE_VECTOR_MIN || vector > INTERRUPT_DEVICE_VECTOR_MAX {
            None
        } else {
            Some(Self {
                destination,
                vector,
            })
        }
    }

    pub const fn destination(self) -> u32 {
        self.destination
    }

    pub const fn vector(self) -> u8 {
        self.vector
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum InterruptRouteStatus {
    Reserved,
    Active,
    Revoking,
    Released,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct InterruptRouteRecord {
    resource: ResourceId,
    device: ResourceId,
    mode: InterruptMode,
    target: InterruptTarget,
    status: InterruptRouteStatus,
}

impl InterruptRouteRecord {
    pub const fn resource(self) -> ResourceId {
        self.resource
    }

    pub const fn device(self) -> ResourceId {
        self.device
    }

    pub const fn mode(self) -> InterruptMode {
        self.mode
    }

    pub const fn target(self) -> InterruptTarget {
        self.target
    }

    pub const fn status(self) -> InterruptRouteStatus {
        self.status
    }

    pub const fn occupies_route(self) -> bool {
        !matches!(self.status, InterruptRouteStatus::Released)
    }

    pub(crate) const fn new(
        resource: ResourceId,
        device: ResourceId,
        mode: InterruptMode,
        target: InterruptTarget,
    ) -> Self {
        Self {
            resource,
            device,
            mode,
            target,
            status: InterruptRouteStatus::Reserved,
        }
    }

    pub(crate) const fn empty() -> Self {
        Self {
            resource: ResourceId::new(0),
            device: ResourceId::new(0),
            mode: InterruptMode::Msi,
            target: InterruptTarget {
                destination: 0,
                vector: INTERRUPT_DEVICE_VECTOR_MIN,
            },
            status: InterruptRouteStatus::Released,
        }
    }

    pub(crate) fn set_status(&mut self, status: InterruptRouteStatus) {
        self.status = status;
    }
}
