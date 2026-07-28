//! Physical xAPIC message encoding for PCI message-signaled interrupts.
//!
//! The x86 architecture layer restricts device vectors below kernel-reserved
//! Local APIC vectors and accepts only 8-bit physical destination IDs.

use crate::{apic::ApicVector, cpu::ApicId};

const MINIMUM_DEVICE_VECTOR: u8 = 0x20;
const MAXIMUM_DEVICE_VECTOR: u8 = 0xdf;
const XAPIC_MESSAGE_BASE: u64 = 0xfee0_0000;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct XapicMsiMessage {
    address: u64,
    data: u32,
}

impl XapicMsiMessage {
    pub const fn new(
        destination: ApicId,
        vector: ApicVector,
    ) -> Result<Self, XapicMsiMessageError> {
        let apic_id = destination.get();
        if apic_id > u8::MAX as u32 {
            return Err(XapicMsiMessageError::DestinationOutOfRange { apic_id });
        }
        let vector = vector.get();
        if vector < MINIMUM_DEVICE_VECTOR || vector > MAXIMUM_DEVICE_VECTOR {
            return Err(XapicMsiMessageError::VectorOutOfRange { vector });
        }
        Ok(Self {
            address: XAPIC_MESSAGE_BASE | ((apic_id as u64) << 12),
            data: vector as u32,
        })
    }

    pub const fn address(self) -> u64 {
        self.address
    }

    pub const fn data(self) -> u32 {
        self.data
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum XapicMsiMessageError {
    DestinationOutOfRange { apic_id: u32 },
    VectorOutOfRange { vector: u8 },
}
