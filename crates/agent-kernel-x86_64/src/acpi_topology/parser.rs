//! Bounded MADT byte parser and topology conversion.
//!
//! The parser validates the table checksum and every traversed entry before
//! reading fields. Known entry sizes are exact; unknown entries are skipped
//! only after a nonzero bounded length has been established.

use crate::cpu::{
    ApicId, CpuTopologyBuilder, FirmwareCpuFlags, FirmwareProcessor, ProcessorSource, MAX_CPU_COUNT,
};

use super::{
    AcpiMachineTopology, AcpiTopologyError, InterruptPolarity, InterruptSourceOverride,
    InterruptTrigger, IoApicDescriptor, MAX_INTERRUPT_OVERRIDES, MAX_IO_APICS,
};

const MADT_HEADER_BYTES: usize = 44;
const SDT_SIGNATURE: &[u8; 4] = b"APIC";
const MMIO_PAGE_MASK: u64 = 4095;

pub fn parse_madt<const CPU_CAPACITY: usize>(
    bytes: &[u8],
    bsp_apic_id: ApicId,
) -> Result<AcpiMachineTopology<CPU_CAPACITY>, AcpiTopologyError> {
    let table = validated_table(bytes)?;
    let mut cpus = CpuTopologyBuilder::<CPU_CAPACITY>::new();
    let mut io_apics = [IoApicDescriptor::EMPTY; MAX_IO_APICS];
    let mut io_apic_count = 0;
    let mut overrides = [InterruptSourceOverride::EMPTY; MAX_INTERRUPT_OVERRIDES];
    let mut override_count = 0;
    let mut local_apic_address = read_u32(table, 36) as u64;
    let supports_legacy_pic = read_u32(table, 40) & 1 != 0;
    let mut local_apic_overridden = false;
    let mut offset = MADT_HEADER_BYTES;

    while offset < table.len() {
        let (entry_type, length) = entry_bounds(table, offset)?;
        let entry = &table[offset..offset + length];
        match entry_type {
            0 => {
                require_length(offset, entry_type, length, 8)?;
                cpus.insert(FirmwareProcessor::new(
                    entry[2] as u32,
                    ApicId::new(entry[3] as u32),
                    ProcessorSource::LocalApic,
                    FirmwareCpuFlags::from_madt_bits(read_u32(entry, 4)),
                ))?;
            }
            1 => {
                require_length(offset, entry_type, length, 12)?;
                let descriptor =
                    IoApicDescriptor::new(entry[2], read_u32(entry, 4) as u64, read_u32(entry, 8));
                insert_io_apic(&mut io_apics, &mut io_apic_count, descriptor)?;
            }
            2 => {
                require_length(offset, entry_type, length, 10)?;
                let descriptor = parse_interrupt_override(entry)?;
                insert_override(&mut overrides, &mut override_count, descriptor)?;
            }
            5 => {
                require_length(offset, entry_type, length, 12)?;
                if local_apic_overridden {
                    return Err(AcpiTopologyError::DuplicateLocalApicAddressOverride);
                }
                local_apic_address = read_u64(entry, 4);
                local_apic_overridden = true;
            }
            6 => return Err(AcpiTopologyError::UnsupportedIoSapic),
            7 => return Err(AcpiTopologyError::UnsupportedLocalSapic),
            9 => {
                require_length(offset, entry_type, length, 16)?;
                cpus.insert(FirmwareProcessor::new(
                    read_u32(entry, 12),
                    ApicId::new(read_u32(entry, 4)),
                    ProcessorSource::LocalX2Apic,
                    FirmwareCpuFlags::from_madt_bits(read_u32(entry, 8)),
                ))?;
            }
            _ => {}
        }
        offset += length;
    }

    if local_apic_address == 0 || local_apic_address & MMIO_PAGE_MASK != 0 {
        return Err(AcpiTopologyError::InvalidLocalApicAddress(
            local_apic_address,
        ));
    }
    if io_apic_count == 0 {
        return Err(AcpiTopologyError::MissingIoApic);
    }
    let cpus = cpus.freeze(bsp_apic_id)?;
    Ok(AcpiMachineTopology::new(
        cpus,
        local_apic_address,
        supports_legacy_pic,
        io_apics,
        io_apic_count,
        overrides,
        override_count,
    ))
}

fn validated_table(bytes: &[u8]) -> Result<&[u8], AcpiTopologyError> {
    if bytes.len() < MADT_HEADER_BYTES {
        return Err(AcpiTopologyError::TableTooShort);
    }
    if &bytes[..4] != SDT_SIGNATURE {
        return Err(AcpiTopologyError::InvalidSignature);
    }
    let declared = read_u32(bytes, 4) as usize;
    if declared < MADT_HEADER_BYTES {
        return Err(AcpiTopologyError::TableTooShort);
    }
    if declared > bytes.len() {
        return Err(AcpiTopologyError::LengthOutOfBounds {
            declared,
            available: bytes.len(),
        });
    }
    let table = &bytes[..declared];
    if table.iter().fold(0u8, |sum, byte| sum.wrapping_add(*byte)) != 0 {
        return Err(AcpiTopologyError::InvalidChecksum);
    }
    Ok(table)
}

fn entry_bounds(table: &[u8], offset: usize) -> Result<(u8, usize), AcpiTopologyError> {
    if table.len() - offset < 2 {
        return Err(AcpiTopologyError::MalformedEntry {
            offset,
            entry_type: table[offset],
            length: table.len() - offset,
        });
    }
    let entry_type = table[offset];
    let length = table[offset + 1] as usize;
    if length < 2 {
        return Err(AcpiTopologyError::MalformedEntry {
            offset,
            entry_type,
            length,
        });
    }
    if length > table.len() - offset {
        return Err(AcpiTopologyError::EntryOutOfBounds {
            offset,
            length,
            table_length: table.len(),
        });
    }
    Ok((entry_type, length))
}

fn require_length(
    offset: usize,
    entry_type: u8,
    actual: usize,
    expected: usize,
) -> Result<(), AcpiTopologyError> {
    if actual == expected {
        Ok(())
    } else {
        Err(AcpiTopologyError::MalformedEntry {
            offset,
            entry_type,
            length: actual,
        })
    }
}

fn insert_io_apic(
    io_apics: &mut [IoApicDescriptor; MAX_IO_APICS],
    count: &mut usize,
    descriptor: IoApicDescriptor,
) -> Result<(), AcpiTopologyError> {
    if descriptor.address() == 0 || descriptor.address() & MMIO_PAGE_MASK != 0 {
        return Err(AcpiTopologyError::InvalidIoApicAddress(
            descriptor.address(),
        ));
    }
    for existing in io_apics[..*count].iter().copied() {
        if existing.id() == descriptor.id() {
            return Err(AcpiTopologyError::DuplicateIoApicId(descriptor.id()));
        }
        if existing.address() == descriptor.address() {
            return Err(AcpiTopologyError::DuplicateIoApicAddress(
                descriptor.address(),
            ));
        }
        if existing.gsi_base() == descriptor.gsi_base() {
            return Err(AcpiTopologyError::DuplicateIoApicGsiBase(
                descriptor.gsi_base(),
            ));
        }
    }
    if *count == MAX_IO_APICS {
        return Err(AcpiTopologyError::IoApicCapacity);
    }
    io_apics[*count] = descriptor;
    *count += 1;
    Ok(())
}

fn parse_interrupt_override(entry: &[u8]) -> Result<InterruptSourceOverride, AcpiTopologyError> {
    if entry[2] != 0 {
        return Err(AcpiTopologyError::InvalidInterruptBus(entry[2]));
    }
    let flags = read_u16(entry, 8);
    let polarity = match flags & 0b11 {
        0 => InterruptPolarity::SameAsBus,
        1 => InterruptPolarity::ActiveHigh,
        3 => InterruptPolarity::ActiveLow,
        _ => return Err(AcpiTopologyError::InvalidInterruptFlags(flags)),
    };
    let trigger = match (flags >> 2) & 0b11 {
        0 => InterruptTrigger::SameAsBus,
        1 => InterruptTrigger::Edge,
        3 => InterruptTrigger::Level,
        _ => return Err(AcpiTopologyError::InvalidInterruptFlags(flags)),
    };
    if flags & !0xf != 0 {
        return Err(AcpiTopologyError::InvalidInterruptFlags(flags));
    }
    Ok(InterruptSourceOverride::new(
        entry[3],
        read_u32(entry, 4),
        polarity,
        trigger,
    ))
}

fn insert_override(
    overrides: &mut [InterruptSourceOverride; MAX_INTERRUPT_OVERRIDES],
    count: &mut usize,
    descriptor: InterruptSourceOverride,
) -> Result<(), AcpiTopologyError> {
    if overrides[..*count]
        .iter()
        .any(|entry| entry.source_irq() == descriptor.source_irq())
    {
        return Err(AcpiTopologyError::DuplicateSourceIrq(
            descriptor.source_irq(),
        ));
    }
    if *count == MAX_INTERRUPT_OVERRIDES {
        return Err(AcpiTopologyError::InterruptOverrideCapacity);
    }
    overrides[*count] = descriptor;
    *count += 1;
    Ok(())
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

const _: () = assert!(MAX_CPU_COUNT <= u16::MAX as usize + 1);
