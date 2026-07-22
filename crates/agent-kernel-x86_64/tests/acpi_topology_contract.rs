use agent_kernel_x86_64::{
    acpi_topology::{
        parse_madt, AcpiTopologyError, InterruptPolarity, InterruptTrigger, MAX_INTERRUPT_OVERRIDES,
    },
    cpu::{ApicId, CpuIndex, ProcessorSource, TopologyError},
};

const MADT_HEADER_BYTES: usize = 44;

fn madt(entries: &[u8]) -> Vec<u8> {
    let mut table = vec![0; MADT_HEADER_BYTES];
    table[0..4].copy_from_slice(b"APIC");
    table[4..8].copy_from_slice(&((MADT_HEADER_BYTES + entries.len()) as u32).to_le_bytes());
    table[8] = 5;
    table[10..16].copy_from_slice(b"AGENTK");
    table[16..24].copy_from_slice(b"SMPV12  ");
    table[24..28].copy_from_slice(&1u32.to_le_bytes());
    table[28..32].copy_from_slice(b"AKRN");
    table[32..36].copy_from_slice(&1u32.to_le_bytes());
    table[36..40].copy_from_slice(&0xfee0_0000u32.to_le_bytes());
    table[40..44].copy_from_slice(&1u32.to_le_bytes());
    table.extend_from_slice(entries);
    repair_checksum(&mut table);
    table
}

fn repair_checksum(table: &mut [u8]) {
    table[9] = 0;
    let sum = table.iter().fold(0u8, |sum, byte| sum.wrapping_add(*byte));
    table[9] = 0u8.wrapping_sub(sum);
}

fn local_apic(uid: u8, apic_id: u8, flags: u32) -> [u8; 8] {
    let mut entry = [0; 8];
    entry[0] = 0;
    entry[1] = 8;
    entry[2] = uid;
    entry[3] = apic_id;
    entry[4..8].copy_from_slice(&flags.to_le_bytes());
    entry
}

fn local_x2apic(uid: u32, apic_id: u32, flags: u32) -> [u8; 16] {
    let mut entry = [0; 16];
    entry[0] = 9;
    entry[1] = 16;
    entry[4..8].copy_from_slice(&apic_id.to_le_bytes());
    entry[8..12].copy_from_slice(&flags.to_le_bytes());
    entry[12..16].copy_from_slice(&uid.to_le_bytes());
    entry
}

fn io_apic(id: u8, address: u32, gsi_base: u32) -> [u8; 12] {
    let mut entry = [0; 12];
    entry[0] = 1;
    entry[1] = 12;
    entry[2] = id;
    entry[4..8].copy_from_slice(&address.to_le_bytes());
    entry[8..12].copy_from_slice(&gsi_base.to_le_bytes());
    entry
}

fn interrupt_override(bus: u8, irq: u8, gsi: u32, flags: u16) -> [u8; 10] {
    let mut entry = [0; 10];
    entry[0] = 2;
    entry[1] = 10;
    entry[2] = bus;
    entry[3] = irq;
    entry[4..8].copy_from_slice(&gsi.to_le_bytes());
    entry[8..10].copy_from_slice(&flags.to_le_bytes());
    entry
}

fn local_apic_override(address: u64) -> [u8; 12] {
    let mut entry = [0; 12];
    entry[0] = 5;
    entry[1] = 12;
    entry[4..12].copy_from_slice(&address.to_le_bytes());
    entry
}

fn append<const N: usize>(entries: &mut Vec<u8>, entry: [u8; N]) {
    entries.extend_from_slice(&entry);
}

#[test]
fn madt_builds_cpu_and_interrupt_topology() {
    let mut entries = Vec::new();
    append(&mut entries, local_apic(7, 9, 1));
    append(&mut entries, local_x2apic(8, 0x1ff, 2));
    append(&mut entries, local_apic(99, 44, 0));
    append(&mut entries, local_apic(5, 2, 1));
    append(&mut entries, io_apic(3, 0xfec0_0000, 0));
    append(&mut entries, interrupt_override(0, 0, 2, 0b0101));
    append(&mut entries, local_apic_override(0xfee0_1000));
    let topology = parse_madt::<8>(&madt(&entries), ApicId::new(2)).unwrap();

    assert_eq!(topology.cpus().len(), 3);
    assert_eq!(topology.cpus().bsp().index(), CpuIndex::BSP);
    assert_eq!(topology.cpus().bsp().processor().uid(), 5);
    assert_eq!(
        topology
            .cpus()
            .get(CpuIndex::new(1).unwrap())
            .unwrap()
            .processor()
            .uid(),
        7
    );
    assert_eq!(
        topology
            .cpus()
            .get(CpuIndex::new(2).unwrap())
            .unwrap()
            .processor()
            .source(),
        ProcessorSource::LocalX2Apic
    );
    assert_eq!(topology.local_apic_address(), 0xfee0_1000);
    assert!(topology.supports_legacy_pic());
    assert_eq!(topology.io_apics().len(), 1);
    assert_eq!(topology.io_apics()[0].id(), 3);
    assert_eq!(topology.io_apics()[0].address(), 0xfec0_0000);
    assert_eq!(topology.io_apics()[0].gsi_base(), 0);
    assert_eq!(topology.interrupt_overrides().len(), 1);
    let override_zero = topology.interrupt_overrides()[0];
    assert_eq!(override_zero.source_irq(), 0);
    assert_eq!(override_zero.gsi(), 2);
    assert_eq!(override_zero.polarity(), InterruptPolarity::ActiveHigh);
    assert_eq!(override_zero.trigger(), InterruptTrigger::Edge);
}

#[test]
fn madt_rejects_header_checksum_and_length_corruption() {
    let mut entries = Vec::new();
    append(&mut entries, local_apic(1, 2, 1));
    append(&mut entries, io_apic(3, 0xfec0_0000, 0));
    let valid = madt(&entries);

    let mut signature = valid.clone();
    signature[0] = b'X';
    assert_eq!(
        parse_madt::<4>(&signature, ApicId::new(2)),
        Err(AcpiTopologyError::InvalidSignature)
    );

    let mut checksum = valid.clone();
    checksum[20] ^= 1;
    assert_eq!(
        parse_madt::<4>(&checksum, ApicId::new(2)),
        Err(AcpiTopologyError::InvalidChecksum)
    );

    let mut short_length = valid.clone();
    short_length[4..8].copy_from_slice(&40u32.to_le_bytes());
    repair_checksum(&mut short_length);
    assert_eq!(
        parse_madt::<4>(&short_length, ApicId::new(2)),
        Err(AcpiTopologyError::TableTooShort)
    );

    let mut long_length = valid.clone();
    long_length[4..8].copy_from_slice(&((valid.len() + 1) as u32).to_le_bytes());
    assert_eq!(
        parse_madt::<4>(&long_length, ApicId::new(2)),
        Err(AcpiTopologyError::LengthOutOfBounds {
            declared: valid.len() + 1,
            available: valid.len(),
        })
    );
}

#[test]
fn madt_rejects_zero_short_and_overrunning_entries() {
    for malformed in [[0x80, 0], [0, 7]] {
        let mut entries = Vec::from(malformed);
        append(&mut entries, local_apic(1, 2, 1));
        append(&mut entries, io_apic(3, 0xfec0_0000, 0));
        assert!(matches!(
            parse_madt::<4>(&madt(&entries), ApicId::new(2)),
            Err(AcpiTopologyError::MalformedEntry { offset: 44, .. })
        ));
    }

    let table = madt(&[0x80, 8, 1, 2]);
    assert_eq!(
        parse_madt::<4>(&table, ApicId::new(2)),
        Err(AcpiTopologyError::EntryOutOfBounds {
            offset: 44,
            length: 8,
            table_length: 48,
        })
    );
}

#[test]
fn madt_propagates_cpu_identity_and_capacity_failures() {
    let mut duplicate = Vec::new();
    append(&mut duplicate, local_apic(1, 2, 1));
    append(&mut duplicate, local_x2apic(2, 2, 1));
    append(&mut duplicate, io_apic(3, 0xfec0_0000, 0));
    assert_eq!(
        parse_madt::<4>(&madt(&duplicate), ApicId::new(2)),
        Err(AcpiTopologyError::Cpu(TopologyError::DuplicateApicId(
            ApicId::new(2)
        )))
    );

    let mut overflow = Vec::new();
    append(&mut overflow, local_apic(1, 2, 1));
    append(&mut overflow, local_apic(2, 3, 1));
    append(&mut overflow, io_apic(3, 0xfec0_0000, 0));
    assert_eq!(
        parse_madt::<1>(&madt(&overflow), ApicId::new(2)),
        Err(AcpiTopologyError::Cpu(TopologyError::CapacityExceeded))
    );
}

#[test]
fn madt_rejects_ambiguous_or_unsupported_interrupt_hardware() {
    let mut no_io_apic = Vec::new();
    append(&mut no_io_apic, local_apic(1, 2, 1));
    assert_eq!(
        parse_madt::<4>(&madt(&no_io_apic), ApicId::new(2)),
        Err(AcpiTopologyError::MissingIoApic)
    );

    let mut duplicate_io = Vec::new();
    append(&mut duplicate_io, local_apic(1, 2, 1));
    append(&mut duplicate_io, io_apic(3, 0xfec0_0000, 0));
    append(&mut duplicate_io, io_apic(3, 0xfec0_1000, 24));
    assert_eq!(
        parse_madt::<4>(&madt(&duplicate_io), ApicId::new(2)),
        Err(AcpiTopologyError::DuplicateIoApicId(3))
    );

    let mut sapic = Vec::new();
    append(&mut sapic, local_apic(1, 2, 1));
    append(&mut sapic, io_apic(3, 0xfec0_0000, 0));
    sapic.extend_from_slice(&[6, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(
        parse_madt::<4>(&madt(&sapic), ApicId::new(2)),
        Err(AcpiTopologyError::UnsupportedIoSapic)
    );
}

#[test]
fn madt_validates_interrupt_overrides_and_capacity() {
    let mut invalid_bus = Vec::new();
    append(&mut invalid_bus, local_apic(1, 2, 1));
    append(&mut invalid_bus, io_apic(3, 0xfec0_0000, 0));
    append(&mut invalid_bus, interrupt_override(1, 0, 2, 0));
    assert_eq!(
        parse_madt::<4>(&madt(&invalid_bus), ApicId::new(2)),
        Err(AcpiTopologyError::InvalidInterruptBus(1))
    );

    let mut invalid_flags = Vec::new();
    append(&mut invalid_flags, local_apic(1, 2, 1));
    append(&mut invalid_flags, io_apic(3, 0xfec0_0000, 0));
    append(&mut invalid_flags, interrupt_override(0, 0, 2, 0b0010));
    assert_eq!(
        parse_madt::<4>(&madt(&invalid_flags), ApicId::new(2)),
        Err(AcpiTopologyError::InvalidInterruptFlags(0b0010))
    );

    let mut duplicate_irq = Vec::new();
    append(&mut duplicate_irq, local_apic(1, 2, 1));
    append(&mut duplicate_irq, io_apic(3, 0xfec0_0000, 0));
    append(&mut duplicate_irq, interrupt_override(0, 0, 2, 0));
    append(&mut duplicate_irq, interrupt_override(0, 0, 3, 0));
    assert_eq!(
        parse_madt::<4>(&madt(&duplicate_irq), ApicId::new(2)),
        Err(AcpiTopologyError::DuplicateSourceIrq(0))
    );

    let mut overflow = Vec::new();
    append(&mut overflow, local_apic(1, 2, 1));
    append(&mut overflow, io_apic(3, 0xfec0_0000, 0));
    for irq in 0..=MAX_INTERRUPT_OVERRIDES as u8 {
        append(&mut overflow, interrupt_override(0, irq, irq as u32, 0));
    }
    assert_eq!(
        parse_madt::<4>(&madt(&overflow), ApicId::new(2)),
        Err(AcpiTopologyError::InterruptOverrideCapacity)
    );
}
