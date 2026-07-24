//! Strict parser for the ACPI `TPM2` hardware-interface table.
//!
//! The parser consumes already mapped firmware bytes. Root-table enumeration
//! and MMIO mapping remain boot-owner responsibilities.

const TPM2_TABLE_BYTES: usize = 52;
const CONTROL_AREA_TO_LOCALITY_BASE: u64 = 0x40;
const CONTROL_REGISTER_ALIGNMENT: u64 = 4;
const CRB_LOCALITY_BYTES: u64 = 4096;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Tpm2StartMethod {
    Crb,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Tpm2AcpiTable {
    revision: u8,
    control_area: u64,
    start_method: Tpm2StartMethod,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Tpm2AcpiTableError {
    TableTooShort,
    InvalidSignature,
    LengthOutOfBounds { declared: usize, available: usize },
    UnsupportedRevision { revision: u8 },
    InvalidChecksum,
    ReservedNotZero,
    InvalidControlArea { address: u64 },
    UnsupportedStartMethod { method: u32 },
}

pub fn parse_tpm2_acpi_table(bytes: &[u8]) -> Result<Tpm2AcpiTable, Tpm2AcpiTableError> {
    if bytes.len() < TPM2_TABLE_BYTES {
        return Err(Tpm2AcpiTableError::TableTooShort);
    }
    if &bytes[..4] != b"TPM2" {
        return Err(Tpm2AcpiTableError::InvalidSignature);
    }
    let declared = read_u32(bytes, 4) as usize;
    if declared < TPM2_TABLE_BYTES {
        return Err(Tpm2AcpiTableError::TableTooShort);
    }
    if declared > bytes.len() {
        return Err(Tpm2AcpiTableError::LengthOutOfBounds {
            declared,
            available: bytes.len(),
        });
    }
    let revision = bytes[8];
    if !(4..=6).contains(&revision) {
        return Err(Tpm2AcpiTableError::UnsupportedRevision { revision });
    }
    if checksum(&bytes[..declared]) != 0 {
        return Err(Tpm2AcpiTableError::InvalidChecksum);
    }
    if bytes[38] != 0 || bytes[39] != 0 {
        return Err(Tpm2AcpiTableError::ReservedNotZero);
    }

    let control_area = read_u64(bytes, 40);
    if control_area < CONTROL_AREA_TO_LOCALITY_BASE
        || !control_area.is_multiple_of(CONTROL_REGISTER_ALIGNMENT)
        || !(control_area - CONTROL_AREA_TO_LOCALITY_BASE).is_multiple_of(CRB_LOCALITY_BYTES)
    {
        return Err(Tpm2AcpiTableError::InvalidControlArea {
            address: control_area,
        });
    }
    let method = read_u32(bytes, 48);
    let start_method = match method {
        7 => Tpm2StartMethod::Crb,
        _ => return Err(Tpm2AcpiTableError::UnsupportedStartMethod { method }),
    };

    Ok(Tpm2AcpiTable {
        revision,
        control_area,
        start_method,
    })
}

impl Tpm2AcpiTable {
    pub const fn revision(self) -> u8 {
        self.revision
    }

    pub const fn control_area(self) -> u64 {
        self.control_area
    }

    pub const fn locality_base(self) -> u64 {
        self.control_area - CONTROL_AREA_TO_LOCALITY_BASE
    }

    pub const fn start_method(self) -> Tpm2StartMethod {
        self.start_method
    }
}

fn checksum(bytes: &[u8]) -> u8 {
    bytes.iter().fold(0_u8, |sum, byte| sum.wrapping_add(*byte))
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    let mut value = [0; 4];
    value.copy_from_slice(&bytes[offset..offset + 4]);
    u32::from_le_bytes(value)
}

fn read_u64(bytes: &[u8], offset: usize) -> u64 {
    let mut value = [0; 8];
    value.copy_from_slice(&bytes[offset..offset + 8]);
    u64::from_le_bytes(value)
}
