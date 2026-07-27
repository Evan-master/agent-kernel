//! Reversible PCI Type-0 BAR sizing.
//!
//! The Ring-0 probe disables decode and bus mastering, sizes one BAR at a time,
//! restores every touched register, and accepts observations only after exact
//! restoration. Status bits are never written back as ones.

use super::{
    PciBar, PciBarIndex, PciBarKind, PciBarSet, PciConfigAccess, PciConfigMutationAccess,
    PciConfigRegister, PciFunctionAddress, PCI_BAR_CAPACITY,
};

const COMMAND_DECODE_AND_MASTER: u16 = 0x0007;
const TYPE_ZERO_HEADER: u8 = 0;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PciBarProbeError {
    UnsupportedHeaderType {
        header_type: u8,
    },
    CommandDecodeDisableMismatch {
        expected: u16,
        actual: u16,
    },
    ReservedMemoryType {
        index: PciBarIndex,
    },
    Unpaired64BitBar {
        index: PciBarIndex,
    },
    InvalidSize {
        index: PciBarIndex,
        mask: u64,
    },
    MisalignedBase {
        index: PciBarIndex,
        base: u64,
        size: u64,
    },
    RestoreMismatch {
        register: PciConfigRegister,
        expected: u32,
        actual: u32,
    },
}

pub fn probe_pci_function_bars<A: PciConfigMutationAccess>(
    access: &mut A,
    address: PciFunctionAddress,
    header_type: u8,
) -> Result<PciBarSet, PciBarProbeError> {
    if header_type != TYPE_ZERO_HEADER {
        return Err(PciBarProbeError::UnsupportedHeaderType { header_type });
    }
    let command_register = PciConfigRegister::COMMAND_STATUS;
    let original_command = access.read_u32(address, command_register) as u16;
    let disabled_command = original_command & !COMMAND_DECODE_AND_MASTER;
    access.write_u32(address, command_register, u32::from(disabled_command));
    let actual_command = access.read_u32(address, command_register) as u16;
    if actual_command != disabled_command {
        let error = PciBarProbeError::CommandDecodeDisableMismatch {
            expected: disabled_command,
            actual: actual_command,
        };
        return match restore_command(access, address, original_command) {
            Ok(()) => Err(error),
            Err(restore) => Err(restore),
        };
    }

    let result = probe_disabled_bars(access, address);
    match restore_command(access, address, original_command) {
        Ok(()) => result,
        Err(restore) => Err(restore),
    }
}

fn probe_disabled_bars<A: PciConfigMutationAccess>(
    access: &mut A,
    address: PciFunctionAddress,
) -> Result<PciBarSet, PciBarProbeError> {
    let mut bars = PciBarSet::new();
    let mut raw_index = 0_u8;
    while raw_index < PCI_BAR_CAPACITY as u8 {
        let index = PciBarIndex::from_probe(raw_index);
        let register = PciConfigRegister::bar(raw_index);
        let original_low = access.read_u32(address, register);
        let kind_bits = ((original_low >> 1) & 0x3) as u8;
        if original_low & 1 == 0 && kind_bits == 0x3 {
            return Err(PciBarProbeError::ReservedMemoryType { index });
        }
        if original_low & 1 == 0 && kind_bits == 0x2 {
            if raw_index + 1 >= PCI_BAR_CAPACITY as u8 {
                return Err(PciBarProbeError::Unpaired64BitBar { index });
            }
            if let Some(bar) = probe_memory64(access, address, index, original_low)? {
                bars.push(bar);
            }
            raw_index += 2;
            continue;
        }
        if let Some(bar) = probe_low_bar(access, address, index, original_low)? {
            bars.push(bar);
        }
        raw_index += 1;
    }
    Ok(bars)
}

fn probe_low_bar<A: PciConfigMutationAccess>(
    access: &mut A,
    address: PciFunctionAddress,
    index: PciBarIndex,
    original: u32,
) -> Result<Option<PciBar>, PciBarProbeError> {
    let register = PciConfigRegister::bar(index.number());
    access.write_u32(address, register, u32::MAX);
    let mask = access.read_u32(address, register);
    access.write_u32(address, register, original);
    verify_restored(access, address, register, original)?;

    if original & 1 != 0 {
        return decode_bar(
            index,
            PciBarKind::Io,
            u64::from(original & 0xffff_fffc),
            u64::from(mask & 0xffff_fffc),
            u64::from(u32::MAX),
            u64::from(mask),
        );
    }
    let prefetchable = original & 0x8 != 0;
    match (original >> 1) & 0x3 {
        0 => decode_bar(
            index,
            PciBarKind::Memory32 { prefetchable },
            u64::from(original & 0xffff_fff0),
            u64::from(mask & 0xffff_fff0),
            u64::from(u32::MAX),
            u64::from(mask),
        ),
        1 => decode_bar(
            index,
            PciBarKind::MemoryBelowOneMegabyte { prefetchable },
            u64::from(original & 0x000f_fff0),
            u64::from(mask & 0x000f_fff0),
            0x000f_ffff,
            u64::from(mask),
        ),
        _ => Err(PciBarProbeError::ReservedMemoryType { index }),
    }
}

fn probe_memory64<A: PciConfigMutationAccess>(
    access: &mut A,
    address: PciFunctionAddress,
    index: PciBarIndex,
    original_low: u32,
) -> Result<Option<PciBar>, PciBarProbeError> {
    let low_register = PciConfigRegister::bar(index.number());
    let high_register = PciConfigRegister::bar(index.number() + 1);
    let original_high = access.read_u32(address, high_register);
    access.write_u32(address, low_register, u32::MAX);
    access.write_u32(address, high_register, u32::MAX);
    let mask_low = access.read_u32(address, low_register);
    let mask_high = access.read_u32(address, high_register);
    access.write_u32(address, high_register, original_high);
    access.write_u32(address, low_register, original_low);
    verify_restored(access, address, high_register, original_high)?;
    verify_restored(access, address, low_register, original_low)?;

    let base = (u64::from(original_high) << 32) | u64::from(original_low & 0xffff_fff0);
    let address_mask = (u64::from(mask_high) << 32) | u64::from(mask_low & 0xffff_fff0);
    let raw_mask = (u64::from(mask_high) << 32) | u64::from(mask_low);
    decode_bar(
        index,
        PciBarKind::Memory64 {
            prefetchable: original_low & 0x8 != 0,
        },
        base,
        address_mask,
        u64::MAX,
        raw_mask,
    )
}

fn decode_bar(
    index: PciBarIndex,
    kind: PciBarKind,
    base: u64,
    address_mask: u64,
    address_bits: u64,
    raw_mask: u64,
) -> Result<Option<PciBar>, PciBarProbeError> {
    if address_mask == 0 {
        return Ok(None);
    }
    let size = ((!address_mask) & address_bits).wrapping_add(1);
    if size == 0 || !size.is_power_of_two() {
        return Err(PciBarProbeError::InvalidSize {
            index,
            mask: raw_mask,
        });
    }
    if base != 0 && base & (size - 1) != 0 {
        return Err(PciBarProbeError::MisalignedBase { index, base, size });
    }
    Ok(Some(PciBar::new(index, kind, base, size)))
}

fn restore_command<A: PciConfigMutationAccess>(
    access: &mut A,
    address: PciFunctionAddress,
    original: u16,
) -> Result<(), PciBarProbeError> {
    let register = PciConfigRegister::COMMAND_STATUS;
    access.write_u32(address, register, u32::from(original));
    let actual = access.read_u32(address, register) as u16;
    if actual != original {
        return Err(PciBarProbeError::RestoreMismatch {
            register,
            expected: u32::from(original),
            actual: u32::from(actual),
        });
    }
    Ok(())
}

fn verify_restored<A: PciConfigAccess>(
    access: &mut A,
    address: PciFunctionAddress,
    register: PciConfigRegister,
    expected: u32,
) -> Result<(), PciBarProbeError> {
    let actual = access.read_u32(address, register);
    if actual != expected {
        return Err(PciBarProbeError::RestoreMismatch {
            register,
            expected,
            actual,
        });
    }
    Ok(())
}
