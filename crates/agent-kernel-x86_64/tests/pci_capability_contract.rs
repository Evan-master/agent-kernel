use agent_kernel_x86_64::pci::{
    discover_pci_capabilities, discover_pci_capabilities_bounded, PciCapabilityError,
    PciConfigAccess, PciConfigRegister, PciFunctionAddress, PCI_CAPABILITY_ID_MSI,
    PCI_CAPABILITY_ID_MSIX, PCI_CAPABILITY_ID_VENDOR_SPECIFIC,
};

const ADDRESS: PciFunctionAddress = match PciFunctionAddress::new(0, 5, 0) {
    Some(address) => address,
    None => panic!("fixed PCI address"),
};

#[test]
fn discovers_a_bounded_conventional_capability_chain() {
    let mut config = Config::with_capability_list(0x50);
    config.set_capability(0x50, PCI_CAPABILITY_ID_MSI, 0x60);
    config.set_capability(0x60, PCI_CAPABILITY_ID_MSIX, 0x70);
    config.set_capability(0x70, PCI_CAPABILITY_ID_VENDOR_SPECIFIC, 0);

    let capabilities = discover_pci_capabilities(&mut config, ADDRESS).unwrap();

    assert_eq!(capabilities.len(), 3);
    assert_eq!(
        capabilities.find(PCI_CAPABILITY_ID_MSI).unwrap().offset(),
        0x50
    );
    assert_eq!(
        capabilities.find(PCI_CAPABILITY_ID_MSIX).unwrap().offset(),
        0x60
    );
    assert_eq!(
        capabilities
            .find(PCI_CAPABILITY_ID_VENDOR_SPECIFIC)
            .unwrap()
            .offset(),
        0x70
    );
}

#[test]
fn requires_the_status_bit_and_a_nonzero_first_pointer() {
    let mut absent = Config::default();
    assert_eq!(
        discover_pci_capabilities(&mut absent, ADDRESS),
        Err(PciCapabilityError::ListUnavailable)
    );

    let mut missing_pointer = Config::with_capability_list(0);
    assert_eq!(
        discover_pci_capabilities(&mut missing_pointer, ADDRESS),
        Err(PciCapabilityError::MissingFirstPointer)
    );
}

#[test]
fn rejects_unaligned_or_out_of_range_pointers() {
    let mut invalid_first = Config::with_capability_list(0x42);
    assert_eq!(
        discover_pci_capabilities(&mut invalid_first, ADDRESS),
        Err(PciCapabilityError::InvalidPointer { pointer: 0x42 })
    );

    let mut invalid_next = Config::with_capability_list(0x50);
    invalid_next.set_capability(0x50, PCI_CAPABILITY_ID_MSI, 0x3c);
    assert_eq!(
        discover_pci_capabilities(&mut invalid_next, ADDRESS),
        Err(PciCapabilityError::InvalidPointer { pointer: 0x3c })
    );
}

#[test]
fn rejects_cycles_before_reading_a_capability_twice() {
    let mut config = Config::with_capability_list(0x50);
    config.set_capability(0x50, PCI_CAPABILITY_ID_MSI, 0x60);
    config.set_capability(0x60, PCI_CAPABILITY_ID_MSIX, 0x50);

    assert_eq!(
        discover_pci_capabilities(&mut config, ADDRESS),
        Err(PciCapabilityError::CycleDetected { offset: 0x50 })
    );
    assert_eq!(config.reads_of(0x50), 1);
}

#[test]
fn rejects_a_chain_larger_than_the_caller_limit() {
    let mut config = Config::with_capability_list(0x50);
    config.set_capability(0x50, PCI_CAPABILITY_ID_MSI, 0x60);
    config.set_capability(0x60, PCI_CAPABILITY_ID_MSIX, 0);

    assert_eq!(
        discover_pci_capabilities_bounded::<1, _>(&mut config, ADDRESS),
        Err(PciCapabilityError::CapacityExceeded { capacity: 1 })
    );
}

struct Config {
    registers: [u32; 64],
    reads: [u8; 64],
}

impl Default for Config {
    fn default() -> Self {
        Self {
            registers: [0; 64],
            reads: [0; 64],
        }
    }
}

impl Config {
    fn with_capability_list(pointer: u8) -> Self {
        let mut config = Self::default();
        config.registers[1] = 1 << 20;
        config.registers[0x34 / 4] = u32::from(pointer);
        config
    }

    fn set_capability(&mut self, offset: u8, id: u8, next: u8) {
        self.registers[usize::from(offset) / 4] = u32::from(id) | (u32::from(next) << 8);
    }

    fn reads_of(&self, offset: u8) -> u8 {
        self.reads[usize::from(offset) / 4]
    }
}

impl PciConfigAccess for Config {
    fn read_u32(&mut self, _address: PciFunctionAddress, register: PciConfigRegister) -> u32 {
        let index = usize::from(register.offset()) / 4;
        self.reads[index] += 1;
        self.registers[index]
    }
}
