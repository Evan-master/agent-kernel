use agent_kernel_x86_64::{
    acpi_topology::{load_acpi_topology, AcpiTopologyError, DirectAcpiHandler},
    cpu::{ApicId, CpuIndex},
};

const RSDP_ADDRESS: usize = 0x100;
const ROOT_ADDRESS: usize = 0x200;
const MADT_ADDRESS: usize = 0x300;
const FIRMWARE_BYTES: usize = 0x1000;

#[derive(Clone, Copy)]
enum RootKind {
    Rsdt,
    Xsdt,
}

fn firmware(kind: RootKind) -> Vec<u8> {
    let mut memory = vec![0; FIRMWARE_BYTES];
    let madt = minimal_madt();
    memory[MADT_ADDRESS..MADT_ADDRESS + madt.len()].copy_from_slice(&madt);

    let root = root_table(kind);
    memory[ROOT_ADDRESS..ROOT_ADDRESS + root.len()].copy_from_slice(&root);

    let rsdp = rsdp(kind);
    memory[RSDP_ADDRESS..RSDP_ADDRESS + rsdp.len()].copy_from_slice(&rsdp);
    memory
}

fn sdt_header(signature: [u8; 4], length: usize) -> Vec<u8> {
    let mut table = vec![0; length];
    table[0..4].copy_from_slice(&signature);
    table[4..8].copy_from_slice(&(length as u32).to_le_bytes());
    table[8] = 1;
    table[10..16].copy_from_slice(b"AGENTK");
    table[16..24].copy_from_slice(b"SMPV12  ");
    table[24..28].copy_from_slice(&1u32.to_le_bytes());
    table[28..32].copy_from_slice(b"AKRN");
    table[32..36].copy_from_slice(&1u32.to_le_bytes());
    table
}

fn root_table(kind: RootKind) -> Vec<u8> {
    let entry_bytes = match kind {
        RootKind::Rsdt => 4,
        RootKind::Xsdt => 8,
    };
    let signature = match kind {
        RootKind::Rsdt => *b"RSDT",
        RootKind::Xsdt => *b"XSDT",
    };
    let mut table = sdt_header(signature, 36 + entry_bytes);
    match kind {
        RootKind::Rsdt => table[36..40].copy_from_slice(&(MADT_ADDRESS as u32).to_le_bytes()),
        RootKind::Xsdt => table[36..44].copy_from_slice(&(MADT_ADDRESS as u64).to_le_bytes()),
    }
    repair_checksum(&mut table, 9);
    table
}

fn rsdp(kind: RootKind) -> Vec<u8> {
    let mut table = vec![0; 36];
    table[0..8].copy_from_slice(b"RSD PTR ");
    table[9..15].copy_from_slice(b"AGENTK");
    match kind {
        RootKind::Rsdt => {
            table[15] = 0;
            table[16..20].copy_from_slice(&(ROOT_ADDRESS as u32).to_le_bytes());
        }
        RootKind::Xsdt => {
            table[15] = 2;
            table[20..24].copy_from_slice(&36u32.to_le_bytes());
            table[24..32].copy_from_slice(&(ROOT_ADDRESS as u64).to_le_bytes());
            repair_checksum(&mut table, 32);
        }
    }
    repair_checksum(&mut table[..20], 8);
    if matches!(kind, RootKind::Xsdt) {
        repair_checksum(&mut table, 32);
    }
    table
}

fn minimal_madt() -> Vec<u8> {
    let mut table = sdt_header(*b"APIC", 44 + 8 + 8 + 12);
    table[36..40].copy_from_slice(&0xfee0_0000u32.to_le_bytes());
    table[40..44].copy_from_slice(&1u32.to_le_bytes());
    table[44..52].copy_from_slice(&[0, 8, 1, 2, 1, 0, 0, 0]);
    table[52..60].copy_from_slice(&[0, 8, 2, 3, 1, 0, 0, 0]);
    table[60..72].copy_from_slice(&[1, 12, 4, 0, 0, 0, 0xc0, 0xfe, 0, 0, 0, 0]);
    repair_checksum(&mut table, 9);
    table
}

fn repair_checksum(bytes: &mut [u8], checksum_offset: usize) {
    bytes[checksum_offset] = 0;
    let sum = bytes.iter().fold(0u8, |sum, byte| sum.wrapping_add(*byte));
    bytes[checksum_offset] = 0u8.wrapping_sub(sum);
}

fn handler(memory: &[u8]) -> DirectAcpiHandler {
    // SAFETY: the backing vector remains allocated and immovable for every
    // discovery call in these tests.
    unsafe { DirectAcpiHandler::new(memory.as_ptr() as usize, memory.len()) }
}

#[test]
fn strict_discovery_accepts_rsdt_and_xsdt_roots() {
    for kind in [RootKind::Rsdt, RootKind::Xsdt] {
        let memory = firmware(kind);
        // SAFETY: RSDP_ADDRESS names a complete RSDP in the handler window.
        let topology =
            unsafe { load_acpi_topology::<_, 8>(handler(&memory), RSDP_ADDRESS, ApicId::new(2)) }
                .unwrap();
        assert_eq!(topology.cpus().len(), 2);
        assert_eq!(topology.cpus().bsp().index(), CpuIndex::BSP);
        assert_eq!(topology.cpus().bsp().processor().apic_id(), ApicId::new(2));
        assert_eq!(topology.io_apics()[0].id(), 4);
    }
}

#[test]
fn strict_discovery_rejects_rsdp_signature_and_checksums() {
    let mut signature = firmware(RootKind::Xsdt);
    signature[RSDP_ADDRESS] = b'X';
    assert_eq!(
        unsafe { load_acpi_topology::<_, 8>(handler(&signature), RSDP_ADDRESS, ApicId::new(2)) },
        Err(AcpiTopologyError::InvalidRsdpSignature)
    );

    let mut v1_checksum = firmware(RootKind::Rsdt);
    v1_checksum[RSDP_ADDRESS + 8] ^= 1;
    assert_eq!(
        unsafe { load_acpi_topology::<_, 8>(handler(&v1_checksum), RSDP_ADDRESS, ApicId::new(2)) },
        Err(AcpiTopologyError::InvalidRsdpChecksum)
    );

    let mut v2_checksum = firmware(RootKind::Xsdt);
    v2_checksum[RSDP_ADDRESS + 32] ^= 1;
    assert_eq!(
        unsafe { load_acpi_topology::<_, 8>(handler(&v2_checksum), RSDP_ADDRESS, ApicId::new(2)) },
        Err(AcpiTopologyError::InvalidRsdpChecksum)
    );
}

#[test]
fn strict_discovery_rejects_root_signature_checksum_and_shape() {
    let mut signature = firmware(RootKind::Xsdt);
    signature[ROOT_ADDRESS] = b'R';
    assert_eq!(
        unsafe { load_acpi_topology::<_, 8>(handler(&signature), RSDP_ADDRESS, ApicId::new(2)) },
        Err(AcpiTopologyError::InvalidRootTableSignature)
    );

    let mut checksum = firmware(RootKind::Xsdt);
    checksum[ROOT_ADDRESS + 20] ^= 1;
    assert_eq!(
        unsafe { load_acpi_topology::<_, 8>(handler(&checksum), RSDP_ADDRESS, ApicId::new(2)) },
        Err(AcpiTopologyError::InvalidRootTableChecksum)
    );

    let mut shape = firmware(RootKind::Xsdt);
    shape[ROOT_ADDRESS + 4..ROOT_ADDRESS + 8].copy_from_slice(&37u32.to_le_bytes());
    assert_eq!(
        unsafe { load_acpi_topology::<_, 8>(handler(&shape), RSDP_ADDRESS, ApicId::new(2)) },
        Err(AcpiTopologyError::InvalidRootTableLength(37))
    );
}

#[test]
fn strict_discovery_rejects_invalid_extended_rsdp_length() {
    let mut memory = firmware(RootKind::Xsdt);
    memory[RSDP_ADDRESS + 20..RSDP_ADDRESS + 24].copy_from_slice(&35u32.to_le_bytes());
    assert_eq!(
        unsafe { load_acpi_topology::<_, 8>(handler(&memory), RSDP_ADDRESS, ApicId::new(2)) },
        Err(AcpiTopologyError::InvalidRsdpLength(35))
    );
}
