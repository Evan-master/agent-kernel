use agent_kernel_x86_64::tpm2::{parse_tpm2_acpi_table, Tpm2AcpiTableError, Tpm2StartMethod};

const CONTROL_AREA: u64 = 0x0000_0000_fed4_0040;

#[test]
fn revision_four_and_six_crb_tables_bind_locality_zero_control_area() {
    for revision in [4, 6] {
        let table = tpm2_table(revision, CONTROL_AREA, 7);

        let descriptor = parse_tpm2_acpi_table(&table).unwrap();

        assert_eq!(descriptor.revision(), revision);
        assert_eq!(descriptor.start_method(), Tpm2StartMethod::Crb);
        assert_eq!(descriptor.control_area(), CONTROL_AREA);
        assert_eq!(descriptor.locality_base(), CONTROL_AREA - 0x40);
    }
}

#[test]
fn parser_rejects_unimplemented_start_methods_and_future_layouts() {
    let acpi_start = tpm2_table(6, CONTROL_AREA, 8);
    assert_eq!(
        parse_tpm2_acpi_table(&acpi_start),
        Err(Tpm2AcpiTableError::UnsupportedStartMethod { method: 8 })
    );

    let future = tpm2_table(7, CONTROL_AREA, 7);
    assert_eq!(
        parse_tpm2_acpi_table(&future),
        Err(Tpm2AcpiTableError::UnsupportedRevision { revision: 7 })
    );
}

#[test]
fn parser_rejects_corrupt_or_ambiguous_control_areas() {
    let mut bad_checksum = tpm2_table(6, CONTROL_AREA, 7);
    bad_checksum[10] ^= 1;
    assert_eq!(
        parse_tpm2_acpi_table(&bad_checksum),
        Err(Tpm2AcpiTableError::InvalidChecksum)
    );

    let zero = tpm2_table(6, 0, 7);
    assert_eq!(
        parse_tpm2_acpi_table(&zero),
        Err(Tpm2AcpiTableError::InvalidControlArea { address: 0 })
    );

    let unaligned = tpm2_table(6, CONTROL_AREA + 1, 7);
    assert_eq!(
        parse_tpm2_acpi_table(&unaligned),
        Err(Tpm2AcpiTableError::InvalidControlArea {
            address: CONTROL_AREA + 1
        })
    );
}

#[test]
fn parser_honors_declared_length_and_zero_reserved_field() {
    let mut short = tpm2_table(6, CONTROL_AREA, 7);
    short[4..8].copy_from_slice(&51_u32.to_le_bytes());
    set_checksum(&mut short);
    assert_eq!(
        parse_tpm2_acpi_table(&short),
        Err(Tpm2AcpiTableError::TableTooShort)
    );

    let mut reserved = tpm2_table(6, CONTROL_AREA, 7);
    reserved[38] = 1;
    set_checksum(&mut reserved);
    assert_eq!(
        parse_tpm2_acpi_table(&reserved),
        Err(Tpm2AcpiTableError::ReservedNotZero)
    );
}

fn tpm2_table(revision: u8, control_area: u64, start_method: u32) -> [u8; 52] {
    let mut bytes = [0_u8; 52];
    bytes[..4].copy_from_slice(b"TPM2");
    bytes[4..8].copy_from_slice(&52_u32.to_le_bytes());
    bytes[8] = revision;
    bytes[10..16].copy_from_slice(b"AGENTK");
    bytes[16..24].copy_from_slice(b"KERNEL19");
    bytes[36..38].copy_from_slice(&0_u16.to_le_bytes());
    bytes[40..48].copy_from_slice(&control_area.to_le_bytes());
    bytes[48..52].copy_from_slice(&start_method.to_le_bytes());
    set_checksum(&mut bytes);
    bytes
}

fn set_checksum(bytes: &mut [u8]) {
    bytes[9] = 0;
    bytes[9] = 0_u8.wrapping_sub(bytes.iter().fold(0_u8, |sum, byte| sum.wrapping_add(*byte)));
}
