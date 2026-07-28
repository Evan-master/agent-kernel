use agent_kernel_x86_64::acpi_topology::{
    load_acpi_dmar_table, parse_dmar, AcpiDmarDiscoveryError, DirectAcpiHandler, DmarPciRequester,
    DmarTableError,
};

const XSDT: usize = 0x80;
const DMAR: usize = 0x100;
const SECOND_DMAR: usize = 0x180;
const IOMMU_BASE: u64 = 0xfed9_0000;

#[test]
fn strict_dmar_parser_selects_the_drhd_covering_edu() {
    let bytes = dmar(&[(0, 5, 0)]);
    let table = parse_dmar::<2, 8>(&bytes).unwrap();
    let requester = DmarPciRequester::new(0, 0, 5, 0).unwrap();
    let unit = table.hardware_unit_for(requester).unwrap();

    assert_eq!(table.host_address_width(), 39);
    assert!(!table.interrupt_remapping());
    assert_eq!(table.hardware_units().len(), 1);
    assert_eq!(unit.segment(), 0);
    assert_eq!(unit.register_base(), IOMMU_BASE);
    assert!(!unit.include_all());
    assert_eq!(unit.scopes().len(), 1);
    assert_eq!(unit.scopes()[0].start_bus(), 0);
    assert_eq!(unit.scopes()[0].path()[0].device(), 5);
    assert_eq!(unit.scopes()[0].path()[0].function(), 0);
}

#[test]
fn strict_dmar_parser_rejects_checksum_reserved_and_scope_shape_errors() {
    let mut checksum = dmar(&[(0, 5, 0)]);
    checksum[20] ^= 1;
    assert_eq!(
        parse_dmar::<2, 8>(&checksum),
        Err(DmarTableError::InvalidChecksum)
    );

    let mut reserved = dmar(&[(0, 5, 0)]);
    reserved[38] = 1;
    set_checksum(&mut reserved);
    assert_eq!(
        parse_dmar::<2, 8>(&reserved),
        Err(DmarTableError::ReservedFieldNonZero)
    );

    let mut scope = dmar(&[(0, 5, 0)]);
    scope[65] = 7;
    set_checksum(&mut scope);
    assert_eq!(
        parse_dmar::<2, 8>(&scope),
        Err(DmarTableError::InvalidDeviceScopeLength { length: 7 })
    );
}

#[test]
fn strict_dmar_parser_enforces_fixed_scope_capacity() {
    let bytes = dmar(&[(0, 4, 0), (0, 5, 0)]);
    assert_eq!(
        parse_dmar::<1, 1>(&bytes),
        Err(DmarTableError::DeviceScopeCapacityExceeded { capacity: 1 })
    );
}

#[test]
fn root_discovery_distinguishes_missing_duplicate_and_valid_dmar_tables() {
    let missing = firmware(0);
    let handler = unsafe { DirectAcpiHandler::new(missing.as_ptr() as usize, missing.len()) };
    assert_eq!(
        unsafe { load_acpi_dmar_table::<_, 2, 8>(handler, 0) },
        Ok(None)
    );

    let valid = firmware(1);
    let handler = unsafe { DirectAcpiHandler::new(valid.as_ptr() as usize, valid.len()) };
    let discovered = unsafe { load_acpi_dmar_table::<_, 2, 8>(handler, 0) }
        .unwrap()
        .unwrap();
    assert_eq!(discovered.hardware_units()[0].register_base(), IOMMU_BASE);

    let duplicate = firmware(2);
    let handler = unsafe { DirectAcpiHandler::new(duplicate.as_ptr() as usize, duplicate.len()) };
    assert_eq!(
        unsafe { load_acpi_dmar_table::<_, 2, 8>(handler, 0) },
        Err(AcpiDmarDiscoveryError::DuplicateTable)
    );
}

fn firmware(dmar_count: usize) -> Vec<u8> {
    let mut bytes = vec![0; 0x240];
    bytes[..8].copy_from_slice(b"RSD PTR ");
    bytes[9..15].copy_from_slice(b"AGENTK");
    bytes[15] = 2;
    bytes[20..24].copy_from_slice(&36_u32.to_le_bytes());
    bytes[24..32].copy_from_slice(&(XSDT as u64).to_le_bytes());

    let xsdt_length = 36 + dmar_count * 8;
    bytes[XSDT..XSDT + 4].copy_from_slice(b"XSDT");
    bytes[XSDT + 4..XSDT + 8].copy_from_slice(&(xsdt_length as u32).to_le_bytes());
    bytes[XSDT + 8] = 1;
    if dmar_count >= 1 {
        bytes[XSDT + 36..XSDT + 44].copy_from_slice(&(DMAR as u64).to_le_bytes());
        let table = dmar(&[(0, 5, 0)]);
        bytes[DMAR..DMAR + table.len()].copy_from_slice(&table);
    }
    if dmar_count == 2 {
        bytes[XSDT + 44..XSDT + 52].copy_from_slice(&(SECOND_DMAR as u64).to_le_bytes());
        let table = dmar(&[(0, 6, 0)]);
        bytes[SECOND_DMAR..SECOND_DMAR + table.len()].copy_from_slice(&table);
    }
    set_checksum(&mut bytes[XSDT..XSDT + xsdt_length]);
    set_rsdp_checksums(&mut bytes);
    bytes
}

fn dmar(scopes: &[(u8, u8, u8)]) -> Vec<u8> {
    let length = 48 + 16 + scopes.len() * 8;
    let mut bytes = vec![0; length];
    bytes[..4].copy_from_slice(b"DMAR");
    bytes[4..8].copy_from_slice(&(length as u32).to_le_bytes());
    bytes[8] = 1;
    bytes[36] = 38;

    bytes[48..50].copy_from_slice(&0_u16.to_le_bytes());
    bytes[50..52].copy_from_slice(&((16 + scopes.len() * 8) as u16).to_le_bytes());
    bytes[54..56].copy_from_slice(&0_u16.to_le_bytes());
    bytes[56..64].copy_from_slice(&IOMMU_BASE.to_le_bytes());
    for (index, (bus, device, function)) in scopes.iter().copied().enumerate() {
        let offset = 64 + index * 8;
        bytes[offset] = 1;
        bytes[offset + 1] = 8;
        bytes[offset + 5] = bus;
        bytes[offset + 6] = device;
        bytes[offset + 7] = function;
    }
    set_checksum(&mut bytes);
    bytes
}

fn set_rsdp_checksums(bytes: &mut [u8]) {
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
}

fn set_checksum(bytes: &mut [u8]) {
    bytes[9] = 0;
    bytes[9] = 0_u8.wrapping_sub(bytes.iter().fold(0_u8, |sum, byte| sum.wrapping_add(*byte)));
}
