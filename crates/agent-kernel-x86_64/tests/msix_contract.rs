use agent_kernel_x86_64::{
    apic::ApicVector,
    cpu::ApicId,
    pci::{
        program_msix_table_entry, MsixCapability, MsixError, MsixTableAccess, MsixTableField,
        PciBarIndex, PciCapability, PciConfigAccess, PciConfigMutationAccess, PciConfigRegister,
        PciFunctionAddress, VolatileMsixTable, XapicMsiMessage, PCI_CAPABILITY_ID_MSIX,
    },
};

const ADDRESS: PciFunctionAddress = match PciFunctionAddress::new(0, 6, 0) {
    Some(address) => address,
    None => panic!("fixed PCI address"),
};

#[test]
fn decodes_table_and_pending_bit_array_descriptors() {
    let mut config = Config::with_msix(3, 0x0000_1002, 0x0000_2004);
    let record = PciCapability::new(PCI_CAPABILITY_ID_MSIX, 0x60).unwrap();

    let capability = MsixCapability::decode(&mut config, ADDRESS, record).unwrap();

    assert_eq!(capability.table_size(), 4);
    assert_eq!(capability.table_bar().number(), 2);
    assert_eq!(capability.table_offset(), 0x1000);
    assert_eq!(capability.pba_bar().number(), 4);
    assert_eq!(capability.pba_offset(), 0x2000);
    let region = capability
        .table_region(PciBarIndex::new(2).unwrap(), 0x1100)
        .unwrap();
    assert_eq!(region.offset(), 0x1000);
    assert_eq!(region.byte_len(), 64);
}

#[test]
fn rejects_a_table_outside_the_declared_bar_before_mmio() {
    let mut config = Config::with_msix(3, 0x0000_1002, 0x0000_2004);
    let record = PciCapability::new(PCI_CAPABILITY_ID_MSIX, 0x60).unwrap();
    let capability = MsixCapability::decode(&mut config, ADDRESS, record).unwrap();

    assert_eq!(
        capability.table_region(PciBarIndex::new(1).unwrap(), 0x2000),
        Err(MsixError::TableBarMismatch {
            expected: PciBarIndex::new(2).unwrap(),
            actual: PciBarIndex::new(1).unwrap(),
        })
    );
    assert_eq!(
        capability.table_region(PciBarIndex::new(2).unwrap(), 0x103f),
        Err(MsixError::TableOutsideBar {
            offset: 0x1000,
            byte_len: 64,
            bar_size: 0x103f,
        })
    );
}

#[test]
fn programs_each_entry_masked_verifies_it_then_unmasks_it() {
    let mut config = Config::with_msix(3, 0x0000_1002, 0x0000_2004);
    let record = PciCapability::new(PCI_CAPABILITY_ID_MSIX, 0x60).unwrap();
    let capability = MsixCapability::decode(&mut config, ADDRESS, record).unwrap();
    let mut table = Table::new(64);
    let message = XapicMsiMessage::new(ApicId::new(0), ApicVector::new(0xd1).unwrap()).unwrap();

    program_msix_table_entry(&mut table, capability, 1, message).unwrap();

    assert_eq!(
        table.writes.first().copied(),
        Some(TableWrite {
            offset: 28,
            value: 1,
        })
    );
    assert_eq!(table.words[16 / 4], 0xfee0_0000);
    assert_eq!(table.words[20 / 4], 0);
    assert_eq!(table.words[24 / 4], 0xd1);
    assert_eq!(table.words[28 / 4], 0);
    assert_eq!(
        table.writes.last().copied(),
        Some(TableWrite {
            offset: 28,
            value: 0,
        })
    );
}

#[test]
fn rejects_an_entry_outside_the_capability_without_mmio() {
    let mut config = Config::with_msix(0, 0x0000_1002, 0x0000_2002);
    let record = PciCapability::new(PCI_CAPABILITY_ID_MSIX, 0x60).unwrap();
    let capability = MsixCapability::decode(&mut config, ADDRESS, record).unwrap();
    let mut table = Table::new(16);
    let message = XapicMsiMessage::new(ApicId::new(0), ApicVector::new(0xd1).unwrap()).unwrap();

    assert_eq!(
        program_msix_table_entry(&mut table, capability, 1, message),
        Err(MsixError::EntryOutOfRange {
            entry: 1,
            table_size: 1,
        })
    );
    assert!(table.writes.is_empty());
}

#[test]
fn keeps_a_failed_entry_masked() {
    let mut config = Config::with_msix(0, 0x0000_1002, 0x0000_2002);
    let record = PciCapability::new(PCI_CAPABILITY_ID_MSIX, 0x60).unwrap();
    let capability = MsixCapability::decode(&mut config, ADDRESS, record).unwrap();
    let mut table = Table::new(16);
    table.ignore_offset = Some(0);
    let message = XapicMsiMessage::new(ApicId::new(0), ApicVector::new(0xd1).unwrap()).unwrap();

    assert_eq!(
        program_msix_table_entry(&mut table, capability, 0, message),
        Err(MsixError::TableVerificationFailed {
            entry: 0,
            field: MsixTableField::MessageAddressLow,
            expected: 0xfee0_0000,
            actual: 0,
        })
    );
    assert_eq!(table.words[12 / 4], 1);
}

#[test]
fn masks_the_function_during_setup_then_enables_and_disables_it() {
    let mut config = Config::with_msix(0, 0x0000_1002, 0x0000_2002);
    let record = PciCapability::new(PCI_CAPABILITY_ID_MSIX, 0x60).unwrap();
    let capability = MsixCapability::decode(&mut config, ADDRESS, record).unwrap();

    capability.prepare(&mut config, ADDRESS).unwrap();
    assert_eq!(config.control() & (1 << 15), 0);
    assert_ne!(config.control() & (1 << 14), 0);

    capability.enable(&mut config, ADDRESS).unwrap();
    assert_ne!(config.control() & (1 << 15), 0);
    assert_eq!(config.control() & (1 << 14), 0);

    capability.disable(&mut config, ADDRESS).unwrap();
    assert_eq!(config.control() & (1 << 15), 0);
    assert_ne!(config.control() & (1 << 14), 0);
}

#[test]
fn binds_a_volatile_table_only_inside_the_mapped_bar_window() {
    let mut config = Config::with_msix(0, 0x0000_1002, 0x0000_2002);
    let record = PciCapability::new(PCI_CAPABILITY_ID_MSIX, 0x60).unwrap();
    let capability = MsixCapability::decode(&mut config, ADDRESS, record).unwrap();
    let region = capability
        .table_region(PciBarIndex::new(2).unwrap(), 0x1100)
        .unwrap();
    let mut mapped_bar = [0u32; 1088];
    let mut table = unsafe {
        VolatileMsixTable::bind(
            mapped_bar.as_mut_ptr().cast::<u8>(),
            mapped_bar.len() * 4,
            region,
        )
        .unwrap()
    };
    let message = XapicMsiMessage::new(ApicId::new(0), ApicVector::new(0xd1).unwrap()).unwrap();

    program_msix_table_entry(&mut table, capability, 0, message).unwrap();

    assert_eq!(mapped_bar[0x1000 / 4], 0xfee0_0000);
    assert_eq!(mapped_bar[(0x1000 + 8) / 4], 0xd1);
    assert_eq!(mapped_bar[(0x1000 + 12) / 4], 0);

    assert_eq!(
        unsafe { VolatileMsixTable::bind(mapped_bar.as_mut_ptr().cast::<u8>(), 0x100f, region,) },
        Err(MsixError::MappedBarTooSmall {
            required: 0x1010,
            actual: 0x100f,
        })
    );
}

#[test]
#[should_panic]
fn safe_volatile_table_rejects_access_past_its_region_in_release_builds() {
    let mut config = Config::with_msix(0, 0x0000_0002, 0x0000_0002);
    let record = PciCapability::new(PCI_CAPABILITY_ID_MSIX, 0x60).unwrap();
    let capability = MsixCapability::decode(&mut config, ADDRESS, record).unwrap();
    let region = capability
        .table_region(PciBarIndex::new(2).unwrap(), 32)
        .unwrap();
    let mut mapped_bar = [0u32; 8];
    let mut table = unsafe {
        VolatileMsixTable::bind(
            mapped_bar.as_mut_ptr().cast::<u8>(),
            mapped_bar.len() * 4,
            region,
        )
        .unwrap()
    };

    let _ = table.read_u32(16);
}

struct Config {
    registers: [u32; 64],
}

impl Config {
    fn with_msix(table_size_field: u16, table: u32, pba: u32) -> Self {
        let mut registers = [0; 64];
        registers[0x60 / 4] =
            u32::from(PCI_CAPABILITY_ID_MSIX) | (u32::from(table_size_field) << 16);
        registers[0x64 / 4] = table;
        registers[0x68 / 4] = pba;
        Self { registers }
    }

    fn control(&self) -> u16 {
        (self.registers[0x60 / 4] >> 16) as u16
    }
}

impl PciConfigAccess for Config {
    fn read_u32(&mut self, _address: PciFunctionAddress, register: PciConfigRegister) -> u32 {
        self.registers[usize::from(register.offset()) / 4]
    }
}

impl PciConfigMutationAccess for Config {
    fn write_u32(&mut self, _address: PciFunctionAddress, register: PciConfigRegister, value: u32) {
        self.registers[usize::from(register.offset()) / 4] = value;
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
struct TableWrite {
    offset: u32,
    value: u32,
}

struct Table {
    words: std::vec::Vec<u32>,
    writes: std::vec::Vec<TableWrite>,
    ignore_offset: Option<u32>,
}

impl Table {
    fn new(byte_len: usize) -> Self {
        Self {
            words: std::vec![0; byte_len / 4],
            writes: std::vec::Vec::new(),
            ignore_offset: None,
        }
    }
}

impl MsixTableAccess for Table {
    fn read_u32(&mut self, byte_offset: u32) -> u32 {
        self.words[byte_offset as usize / 4]
    }

    fn write_u32(&mut self, byte_offset: u32, value: u32) {
        self.writes.push(TableWrite {
            offset: byte_offset,
            value,
        });
        if self.ignore_offset != Some(byte_offset) {
            self.words[byte_offset as usize / 4] = value;
        }
    }
}
