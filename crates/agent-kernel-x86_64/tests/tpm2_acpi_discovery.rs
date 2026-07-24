use agent_kernel_x86_64::acpi_topology::{
    load_acpi_tpm2_table, AcpiTpm2DiscoveryError, DirectAcpiHandler,
};
use agent_kernel_x86_64::tpm2::{Tpm2AcpiTableError, Tpm2StartMethod};

const XSDT: usize = 64;
const TPM2: usize = 128;
const CONTROL_AREA: u64 = 0xfed4_0040;

#[test]
fn root_discovery_finds_and_validates_the_tpm2_table() {
    let memory = firmware(true, false);
    let handler = unsafe { DirectAcpiHandler::new(memory.as_ptr() as usize, memory.len()) };

    let table = unsafe { load_acpi_tpm2_table(handler, 0) }
        .unwrap()
        .expect("TPM2 table");

    assert_eq!(table.control_area(), CONTROL_AREA);
    assert_eq!(table.start_method(), Tpm2StartMethod::Crb);
}

#[test]
fn root_discovery_distinguishes_missing_and_malformed_tpm2_tables() {
    let memory = firmware(false, false);
    let handler = unsafe { DirectAcpiHandler::new(memory.as_ptr() as usize, memory.len()) };
    assert_eq!(unsafe { load_acpi_tpm2_table(handler, 0) }, Ok(None));

    let memory = firmware(true, true);
    let handler = unsafe { DirectAcpiHandler::new(memory.as_ptr() as usize, memory.len()) };
    assert_eq!(
        unsafe { load_acpi_tpm2_table(handler, 0) },
        Err(AcpiTpm2DiscoveryError::Table(
            Tpm2AcpiTableError::InvalidChecksum
        ))
    );
}

fn firmware(include_tpm: bool, corrupt_tpm: bool) -> Vec<u8> {
    let mut bytes = vec![0; 256];
    bytes[..8].copy_from_slice(b"RSD PTR ");
    bytes[9..15].copy_from_slice(b"AGENTK");
    bytes[15] = 2;
    bytes[20..24].copy_from_slice(&36_u32.to_le_bytes());
    bytes[24..32].copy_from_slice(&(XSDT as u64).to_le_bytes());

    bytes[XSDT..XSDT + 4].copy_from_slice(b"XSDT");
    let xsdt_length = if include_tpm { 44 } else { 36 };
    bytes[XSDT + 4..XSDT + 8].copy_from_slice(&(xsdt_length as u32).to_le_bytes());
    bytes[XSDT + 8] = 1;
    if include_tpm {
        bytes[XSDT + 36..XSDT + 44].copy_from_slice(&(TPM2 as u64).to_le_bytes());
    }
    set_checksum(&mut bytes[XSDT..XSDT + xsdt_length]);

    if include_tpm {
        bytes[TPM2..TPM2 + 4].copy_from_slice(b"TPM2");
        bytes[TPM2 + 4..TPM2 + 8].copy_from_slice(&52_u32.to_le_bytes());
        bytes[TPM2 + 8] = 6;
        bytes[TPM2 + 40..TPM2 + 48].copy_from_slice(&CONTROL_AREA.to_le_bytes());
        bytes[TPM2 + 48..TPM2 + 52].copy_from_slice(&7_u32.to_le_bytes());
        set_checksum(&mut bytes[TPM2..TPM2 + 52]);
        if corrupt_tpm {
            bytes[TPM2 + 12] ^= 1;
        }
    }
    bytes[8] = 0;
    bytes[8] = 0_u8.wrapping_sub(
        bytes[..20]
            .iter()
            .fold(0_u8, |sum, byte| sum.wrapping_add(*byte)),
    );
    bytes[32] = 0;
    bytes[32] = 0_u8.wrapping_sub(
        bytes[..36]
            .iter()
            .fold(0_u8, |sum, byte| sum.wrapping_add(*byte)),
    );
    bytes
}

fn set_checksum(bytes: &mut [u8]) {
    bytes[9] = 0;
    bytes[9] = 0_u8.wrapping_sub(bytes.iter().fold(0_u8, |sum, byte| sum.wrapping_add(*byte)));
}
