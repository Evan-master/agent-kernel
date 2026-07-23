//! Strict byte-oriented ACPI root discovery and MADT conversion.
//!
//! Firmware SDT entries begin after a packed 36-byte header. XSDT entries are
//! therefore not naturally aligned as `u64` values, so discovery decodes every
//! address from bytes before passing the mapped MADT to the bounded parser.

use acpi::Handler;

use crate::cpu::ApicId;

use super::{parse_madt, AcpiMachineTopology, AcpiTopologyError};

const RSDP_V1_BYTES: usize = 20;
const RSDP_V2_BYTES: usize = 36;
const SDT_HEADER_BYTES: usize = 36;
const MAX_RSDP_BYTES: usize = 4096;
const MAX_ROOT_TABLE_BYTES: usize = 64 * 1024;
const MAX_MADT_BYTES: usize = 64 * 1024;

/// Validate RSDP and root-table bytes before constructing upstream ACPI table
/// enumeration state.
///
/// # Safety
///
/// `rsdp_address` must identify at least 36 readable physical bytes in the
/// handler's mapping domain. Every root-table address named by those bytes must
/// also refer to readable firmware memory.
pub unsafe fn load_acpi_topology<H: Handler, const CPU_CAPACITY: usize>(
    handler: H,
    rsdp_address: usize,
    bsp_apic_id: ApicId,
) -> Result<AcpiMachineTopology<CPU_CAPACITY>, AcpiTopologyError> {
    let (revision, root_address) = unsafe { validate_rsdp(&handler, rsdp_address)? };
    let root_length = unsafe { validate_root_table(&handler, revision, root_address)? };
    unsafe { discover_madt(&handler, revision, root_address, root_length, bsp_apic_id) }
}

unsafe fn validate_rsdp<H: Handler>(
    handler: &H,
    address: usize,
) -> Result<(u8, usize), AcpiTopologyError> {
    let initial = unsafe { handler.map_physical_region::<u8>(address, RSDP_V2_BYTES) };
    let bytes = mapping_bytes(&initial);
    if &bytes[..8] != b"RSD PTR " {
        return Err(AcpiTopologyError::InvalidRsdpSignature);
    }
    if !bytes[9..15].iter().all(|byte| byte.is_ascii()) {
        return Err(AcpiTopologyError::InvalidRsdpOemId);
    }
    if checksum(&bytes[..RSDP_V1_BYTES]) != 0 {
        return Err(AcpiTopologyError::InvalidRsdpChecksum);
    }
    let revision = bytes[15];
    if revision == 0 {
        let root = read_u32(bytes, 16) as usize;
        return nonzero_root(revision, root);
    }

    let length = read_u32(bytes, 20) as usize;
    if !(RSDP_V2_BYTES..=MAX_RSDP_BYTES).contains(&length) {
        return Err(AcpiTopologyError::InvalidRsdpLength(length));
    }
    drop(initial);
    let complete = unsafe { handler.map_physical_region::<u8>(address, length) };
    let bytes = mapping_bytes(&complete);
    if checksum(bytes) != 0 {
        return Err(AcpiTopologyError::InvalidRsdpChecksum);
    }
    let root = read_u64(bytes, 24) as usize;
    nonzero_root(revision, root)
}

unsafe fn validate_root_table<H: Handler>(
    handler: &H,
    revision: u8,
    address: usize,
) -> Result<usize, AcpiTopologyError> {
    let header = unsafe { handler.map_physical_region::<u8>(address, SDT_HEADER_BYTES) };
    let bytes = mapping_bytes(&header);
    let (signature, entry_bytes) = if revision == 0 {
        (b"RSDT".as_slice(), 4)
    } else {
        (b"XSDT".as_slice(), 8)
    };
    if &bytes[..4] != signature {
        return Err(AcpiTopologyError::InvalidRootTableSignature);
    }
    let length = read_u32(bytes, 4) as usize;
    if !(SDT_HEADER_BYTES..=MAX_ROOT_TABLE_BYTES).contains(&length)
        || !(length - SDT_HEADER_BYTES).is_multiple_of(entry_bytes)
    {
        return Err(AcpiTopologyError::InvalidRootTableLength(length));
    }
    drop(header);
    let complete = unsafe { handler.map_physical_region::<u8>(address, length) };
    if checksum(mapping_bytes(&complete)) != 0 {
        return Err(AcpiTopologyError::InvalidRootTableChecksum);
    }
    Ok(length)
}

unsafe fn discover_madt<H: Handler, const CPU_CAPACITY: usize>(
    handler: &H,
    revision: u8,
    root_address: usize,
    root_length: usize,
    bsp_apic_id: ApicId,
) -> Result<AcpiMachineTopology<CPU_CAPACITY>, AcpiTopologyError> {
    let root = unsafe { handler.map_physical_region::<u8>(root_address, root_length) };
    let root_bytes = mapping_bytes(&root);
    let entry_bytes = if revision == 0 { 4 } else { 8 };
    for offset in (SDT_HEADER_BYTES..root_length).step_by(entry_bytes) {
        let table_address = if entry_bytes == 4 {
            read_u32(root_bytes, offset) as usize
        } else {
            read_u64(root_bytes, offset) as usize
        };
        if table_address == 0 {
            continue;
        }
        let header = unsafe { handler.map_physical_region::<u8>(table_address, SDT_HEADER_BYTES) };
        let header_bytes = mapping_bytes(&header);
        if &header_bytes[..4] != b"APIC" {
            continue;
        }
        let length = read_u32(header_bytes, 4) as usize;
        if length < SDT_HEADER_BYTES {
            return Err(AcpiTopologyError::TableTooShort);
        }
        if length > MAX_MADT_BYTES {
            return Err(AcpiTopologyError::LengthOutOfBounds {
                declared: length,
                available: MAX_MADT_BYTES,
            });
        }
        drop(header);
        let complete = unsafe { handler.map_physical_region::<u8>(table_address, length) };
        return parse_madt(mapping_bytes(&complete), bsp_apic_id);
    }
    Err(AcpiTopologyError::MissingMadt)
}

fn nonzero_root(revision: u8, root: usize) -> Result<(u8, usize), AcpiTopologyError> {
    if root == 0 {
        Err(AcpiTopologyError::RootAddressMissing)
    } else {
        Ok((revision, root))
    }
}

fn mapping_bytes<H: Handler>(mapping: &acpi::PhysicalMapping<H, u8>) -> &[u8] {
    // SAFETY: Handler guarantees the complete region is readable while the
    // mapping value remains live.
    unsafe { core::slice::from_raw_parts(mapping.virtual_start.as_ptr(), mapping.region_length) }
}

fn checksum(bytes: &[u8]) -> u8 {
    bytes.iter().fold(0u8, |sum, byte| sum.wrapping_add(*byte))
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
