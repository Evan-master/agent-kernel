//! Strict byte parser for ACPI DMA Remapping Reporting tables.
//!
//! This x86 architecture child validates ACPI and DRHD bounds, fills the
//! fixed-capacity DMAR model, and performs no allocation or MMIO. Keep firmware
//! byte handling here so policy and VT-d register ownership remain separate.

use super::{
    DmarDeviceScope, DmarDeviceScopeKind, DmarHardwareUnit, DmarPciPath, DmarTable, DmarTableError,
    DEVICE_SCOPE_HEADER_BYTES, DMAR_HEADER_BYTES, DRHD_HEADER_BYTES, MAX_DMAR_PATH_ENTRIES,
};

pub fn parse_dmar<const UNITS: usize, const SCOPES: usize>(
    bytes: &[u8],
) -> Result<DmarTable<UNITS, SCOPES>, DmarTableError> {
    if bytes.len() < DMAR_HEADER_BYTES {
        return Err(DmarTableError::TableTooShort);
    }
    if &bytes[..4] != b"DMAR" {
        return Err(DmarTableError::InvalidSignature);
    }
    let declared = read_u32(bytes, 4) as usize;
    if !(DMAR_HEADER_BYTES..=bytes.len()).contains(&declared) {
        return Err(DmarTableError::InvalidLength {
            declared,
            available: bytes.len(),
        });
    }
    let bytes = &bytes[..declared];
    if checksum(bytes) != 0 {
        return Err(DmarTableError::InvalidChecksum);
    }
    let host_address_width = bytes[36]
        .checked_add(1)
        .filter(|width| *width <= 64)
        .ok_or(DmarTableError::InvalidHostAddressWidth(bytes[36]))?;
    if !bytes[38..48].iter().all(|byte| *byte == 0) {
        return Err(DmarTableError::ReservedFieldNonZero);
    }

    let mut table = DmarTable {
        host_address_width,
        interrupt_remapping: bytes[37] & 1 != 0,
        units: [DmarHardwareUnit::empty(); UNITS],
        unit_len: 0,
    };
    let mut offset = DMAR_HEADER_BYTES;
    while offset < bytes.len() {
        let remaining = bytes.len() - offset;
        if remaining < 4 {
            return Err(DmarTableError::StructureOutOfBounds {
                length: 4,
                remaining,
            });
        }
        let structure_length = read_u16(bytes, offset + 2) as usize;
        if structure_length < 4 {
            return Err(DmarTableError::InvalidStructureLength {
                length: structure_length,
            });
        }
        if structure_length > remaining {
            return Err(DmarTableError::StructureOutOfBounds {
                length: structure_length,
                remaining,
            });
        }
        if read_u16(bytes, offset) == 0 {
            let unit = parse_hardware_unit::<SCOPES>(&bytes[offset..offset + structure_length])?;
            let Some(slot) = table.units.get_mut(table.unit_len) else {
                return Err(DmarTableError::HardwareUnitCapacityExceeded { capacity: UNITS });
            };
            *slot = unit;
            table.unit_len += 1;
        }
        offset += structure_length;
    }
    Ok(table)
}

fn parse_hardware_unit<const SCOPES: usize>(
    bytes: &[u8],
) -> Result<DmarHardwareUnit<SCOPES>, DmarTableError> {
    if bytes.len() < DRHD_HEADER_BYTES {
        return Err(DmarTableError::InvalidStructureLength {
            length: bytes.len(),
        });
    }
    let flags = bytes[4];
    if flags & !1 != 0 {
        return Err(DmarTableError::InvalidHardwareUnitFlags(flags));
    }
    if bytes[5] != 0 {
        return Err(DmarTableError::ReservedFieldNonZero);
    }
    let register_base = read_u64(bytes, 8);
    if register_base == 0 || register_base & 0xfff != 0 {
        return Err(DmarTableError::InvalidRegisterBase(register_base));
    }
    let mut unit = DmarHardwareUnit {
        include_all: flags & 1 != 0,
        segment: read_u16(bytes, 6),
        register_base,
        scopes: [DmarDeviceScope::EMPTY; SCOPES],
        scope_len: 0,
    };
    let mut offset = DRHD_HEADER_BYTES;
    while offset < bytes.len() {
        let remaining = bytes.len() - offset;
        if remaining < DEVICE_SCOPE_HEADER_BYTES {
            return Err(DmarTableError::InvalidDeviceScopeLength { length: remaining });
        }
        let length = bytes[offset + 1] as usize;
        if length < DEVICE_SCOPE_HEADER_BYTES + 2
            || !(length - DEVICE_SCOPE_HEADER_BYTES).is_multiple_of(2)
            || length > remaining
        {
            return Err(DmarTableError::InvalidDeviceScopeLength { length });
        }
        let scope = parse_device_scope(&bytes[offset..offset + length])?;
        let Some(slot) = unit.scopes.get_mut(unit.scope_len) else {
            return Err(DmarTableError::DeviceScopeCapacityExceeded { capacity: SCOPES });
        };
        *slot = scope;
        unit.scope_len += 1;
        offset += length;
    }
    Ok(unit)
}

fn parse_device_scope(bytes: &[u8]) -> Result<DmarDeviceScope, DmarTableError> {
    let kind = match bytes[0] {
        1 => DmarDeviceScopeKind::PciEndpoint,
        2 => DmarDeviceScopeKind::PciBridge,
        3 => DmarDeviceScopeKind::IoApic,
        4 => DmarDeviceScopeKind::Hpet,
        5 => DmarDeviceScopeKind::AcpiNamespace,
        raw => return Err(DmarTableError::InvalidDeviceScopeKind(raw)),
    };
    if bytes[2] != 0 || bytes[3] != 0 {
        return Err(DmarTableError::ReservedFieldNonZero);
    }
    let path_len = (bytes.len() - DEVICE_SCOPE_HEADER_BYTES) / 2;
    if path_len > MAX_DMAR_PATH_ENTRIES {
        return Err(DmarTableError::DevicePathCapacityExceeded {
            capacity: MAX_DMAR_PATH_ENTRIES,
        });
    }
    let mut path = [DmarPciPath::EMPTY; MAX_DMAR_PATH_ENTRIES];
    for (index, entry) in bytes[DEVICE_SCOPE_HEADER_BYTES..]
        .as_chunks::<2>()
        .0
        .iter()
        .enumerate()
    {
        if entry[0] > 31 || entry[1] > 7 {
            return Err(DmarTableError::InvalidDevicePath);
        }
        path[index] = DmarPciPath {
            device: entry[0],
            function: entry[1],
        };
    }
    Ok(DmarDeviceScope {
        kind,
        enumeration_id: bytes[4],
        start_bus: bytes[5],
        path,
        path_len,
    })
}

fn checksum(bytes: &[u8]) -> u8 {
    bytes.iter().fold(0u8, |sum, byte| sum.wrapping_add(*byte))
}

fn read_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

fn read_u64(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
        bytes[offset + 4],
        bytes[offset + 5],
        bytes[offset + 6],
        bytes[offset + 7],
    ])
}
