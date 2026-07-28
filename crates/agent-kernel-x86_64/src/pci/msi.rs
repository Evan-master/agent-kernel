//! PCI MSI capability programming and physical xAPIC message encoding.
//!
//! This architecture module supports one verified 64-bit message per function.
//! Sources remain disabled until every message field passes readback.

use super::{
    config_field, PciCapability, PciConfigAccess, PciConfigMutationAccess, PciFunctionAddress,
    XapicMsiMessage, PCI_CAPABILITY_ID_MSI,
};

const MSI_ENABLE: u16 = 1;
const MULTIPLE_MESSAGE_CAPABLE_MASK: u16 = 0x000e;
const MULTIPLE_MESSAGE_ENABLE_MASK: u16 = 0x0070;
const ADDRESS_64_BIT_CAPABLE: u16 = 1 << 7;
const PER_VECTOR_MASKING_CAPABLE: u16 = 1 << 8;
const PER_VECTOR_MASK_BITS_OFFSET: u16 = 16;
const LAST_PVM_CAPABILITY_OFFSET: u8 = 0xe8;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct MsiCapability {
    offset: u8,
    multiple_message_capable: u8,
    per_vector_masking: bool,
}

impl MsiCapability {
    pub fn decode<A: PciConfigAccess>(
        access: &mut A,
        address: PciFunctionAddress,
        record: PciCapability,
    ) -> Result<Self, MsiError> {
        if record.id() != PCI_CAPABILITY_ID_MSI {
            return Err(MsiError::WrongCapabilityId {
                actual: record.id(),
            });
        }
        if record.offset() > 0xf0 {
            return Err(MsiError::CapabilityOutOfRange {
                offset: record.offset(),
            });
        }
        let header = config_field::read_u32(access, address, u16::from(record.offset()));
        if header as u8 != PCI_CAPABILITY_ID_MSI {
            return Err(MsiError::WrongCapabilityId {
                actual: header as u8,
            });
        }
        let control = (header >> 16) as u16;
        if control & ADDRESS_64_BIT_CAPABLE == 0 {
            return Err(MsiError::UnsupportedAddressWidth);
        }
        let per_vector_masking = control & PER_VECTOR_MASKING_CAPABLE != 0;
        if per_vector_masking && record.offset() > LAST_PVM_CAPABILITY_OFFSET {
            return Err(MsiError::CapabilityOutOfRange {
                offset: record.offset(),
            });
        }
        Ok(Self {
            offset: record.offset(),
            multiple_message_capable: ((control & MULTIPLE_MESSAGE_CAPABLE_MASK) >> 1) as u8,
            per_vector_masking,
        })
    }

    pub const fn offset(self) -> u8 {
        self.offset
    }

    pub const fn multiple_message_capable(self) -> u8 {
        self.multiple_message_capable
    }

    pub const fn per_vector_masking(self) -> bool {
        self.per_vector_masking
    }

    pub fn configure<A: PciConfigMutationAccess>(
        self,
        access: &mut A,
        address: PciFunctionAddress,
        message: XapicMsiMessage,
    ) -> Result<(), MsiError> {
        let control_offset = u16::from(self.offset) + 2;
        let original = config_field::read_u16(access, address, control_offset);
        if original & ADDRESS_64_BIT_CAPABLE == 0 {
            return Err(MsiError::UnsupportedAddressWidth);
        }
        let disabled = original & !(MSI_ENABLE | MULTIPLE_MESSAGE_ENABLE_MASK);
        write_and_verify_u16(
            access,
            address,
            control_offset,
            disabled,
            MsiRegister::Control,
        )?;

        let address_low = message.address() as u32;
        let address_high = (message.address() >> 32) as u32;
        write_and_verify_u32(
            access,
            address,
            u16::from(self.offset) + 4,
            address_low,
            MsiRegister::MessageAddressLow,
        )?;
        write_and_verify_u32(
            access,
            address,
            u16::from(self.offset) + 8,
            address_high,
            MsiRegister::MessageAddressHigh,
        )?;
        write_and_verify_u16(
            access,
            address,
            u16::from(self.offset) + 12,
            message.data() as u16,
            MsiRegister::MessageData,
        )?;
        if self.per_vector_masking {
            let mask_offset = u16::from(self.offset) + PER_VECTOR_MASK_BITS_OFFSET;
            let original_mask = config_field::read_u32(access, address, mask_offset);
            write_and_verify_u32(
                access,
                address,
                mask_offset,
                original_mask & !1,
                MsiRegister::MaskBits,
            )?;
        }

        write_and_verify_u16(
            access,
            address,
            control_offset,
            disabled | MSI_ENABLE,
            MsiRegister::Control,
        )
    }

    pub fn disable<A: PciConfigMutationAccess>(
        self,
        access: &mut A,
        address: PciFunctionAddress,
    ) -> Result<(), MsiError> {
        let offset = u16::from(self.offset) + 2;
        let control = config_field::read_u16(access, address, offset);
        write_and_verify_u16(
            access,
            address,
            offset,
            control & !(MSI_ENABLE | MULTIPLE_MESSAGE_ENABLE_MASK),
            MsiRegister::Control,
        )
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum MsiRegister {
    Control,
    MessageAddressLow,
    MessageAddressHigh,
    MessageData,
    MaskBits,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum MsiError {
    WrongCapabilityId {
        actual: u8,
    },
    CapabilityOutOfRange {
        offset: u8,
    },
    UnsupportedAddressWidth,
    VerificationFailed {
        register: MsiRegister,
        expected: u32,
        actual: u32,
    },
}

fn write_and_verify_u16<A: PciConfigMutationAccess>(
    access: &mut A,
    address: PciFunctionAddress,
    offset: u16,
    value: u16,
    register: MsiRegister,
) -> Result<(), MsiError> {
    config_field::write_u16(access, address, offset, value);
    let actual = config_field::read_u16(access, address, offset);
    if actual != value {
        return Err(MsiError::VerificationFailed {
            register,
            expected: u32::from(value),
            actual: u32::from(actual),
        });
    }
    Ok(())
}

fn write_and_verify_u32<A: PciConfigMutationAccess>(
    access: &mut A,
    address: PciFunctionAddress,
    offset: u16,
    value: u32,
    register: MsiRegister,
) -> Result<(), MsiError> {
    config_field::write_u32(access, address, offset, value);
    let actual = config_field::read_u32(access, address, offset);
    if actual != value {
        return Err(MsiError::VerificationFailed {
            register,
            expected: value,
            actual,
        });
    }
    Ok(())
}
