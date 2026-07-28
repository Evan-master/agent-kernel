use agent_kernel_x86_64::{
    pci::{
        discover_pci_capabilities, probe_pci_function_bars, PciBarIndex, PciBarKind,
        PciConfigAccess, PciConfigMutationAccess, PciConfigRegister, PciFunctionAddress,
        VirtioPciCapabilityKind, PCI_CAPABILITY_ID_VENDOR_SPECIFIC,
    },
    virtio_rng::{
        virtio_rng_selector, VirtioRngPciCapabilities, VirtioRngPciCapabilityError,
        VirtioRngRequiredCapability, VIRTIO_RNG_DEVICE_ID, VIRTIO_RNG_VENDOR_ID,
    },
};

const ADDRESS: PciFunctionAddress = match PciFunctionAddress::new(0, 6, 0) {
    Some(address) => address,
    None => panic!("fixed PCI address"),
};

#[test]
fn selects_one_common_notify_and_isr_capability_for_the_exact_rng_identity() {
    let mut config = Config::standard();
    let list = discover_pci_capabilities(&mut config, ADDRESS).unwrap();

    let capabilities = VirtioRngPciCapabilities::decode(&mut config, ADDRESS, &list).unwrap();
    let bars = probe_pci_function_bars(&mut config, ADDRESS, 0).unwrap();
    let regions = capabilities.resolve_regions(bars).unwrap();

    assert_eq!(
        capabilities.common().kind(),
        VirtioPciCapabilityKind::CommonConfiguration
    );
    assert_eq!(
        capabilities.notify().kind(),
        VirtioPciCapabilityKind::Notify
    );
    assert_eq!(capabilities.notify().notify_offset_multiplier(), Some(4));
    assert_eq!(capabilities.isr().kind(), VirtioPciCapabilityKind::Isr);
    assert_eq!(regions.common().bar().index(), PciBarIndex::new(4).unwrap());
    assert_eq!(
        regions.common().bar().kind(),
        PciBarKind::Memory64 {
            prefetchable: false
        }
    );
    assert_eq!(regions.common().bar().base(), 0x8000_0000);
    assert_eq!(regions.common().region().offset(), 0);
    assert_eq!(regions.notify().region().offset(), 0x100);
    assert_eq!(regions.isr().region().offset(), 0x200);
    let selector = virtio_rng_selector(ADDRESS);
    assert_eq!(selector.address(), ADDRESS);
    assert_eq!(selector.vendor_id(), VIRTIO_RNG_VENDOR_ID);
    assert_eq!(selector.device_id(), VIRTIO_RNG_DEVICE_ID);
}

#[test]
fn rejects_missing_and_duplicate_required_capabilities() {
    let mut missing = Config::default();
    missing.begin(0x50);
    missing.set_capability(0x50, 0x70, 16, 1, 4, 0, 0, 0x38, None);
    missing.set_capability(0x70, 0, 20, 2, 4, 0, 0x100, 0x40, Some(4));
    let list = discover_pci_capabilities(&mut missing, ADDRESS).unwrap();
    assert_eq!(
        VirtioRngPciCapabilities::decode(&mut missing, ADDRESS, &list),
        Err(VirtioRngPciCapabilityError::Missing(
            VirtioRngRequiredCapability::Isr
        ))
    );

    let mut duplicate = Config::default();
    duplicate.begin(0x50);
    duplicate.set_capability(0x50, 0x60, 16, 1, 4, 0, 0, 0x38, None);
    duplicate.set_capability(0x60, 0x70, 16, 1, 4, 1, 0x40, 0x38, None);
    duplicate.set_capability(0x70, 0x90, 20, 2, 4, 0, 0x100, 0x40, Some(4));
    duplicate.set_capability(0x90, 0, 16, 3, 4, 0, 0x200, 1, None);
    let list = discover_pci_capabilities(&mut duplicate, ADDRESS).unwrap();
    assert_eq!(
        VirtioRngPciCapabilities::decode(&mut duplicate, ADDRESS, &list),
        Err(VirtioRngPciCapabilityError::Duplicate(
            VirtioRngRequiredCapability::Common
        ))
    );
}

struct Config {
    registers: [u32; 64],
    masks: [u32; 6],
}

impl Default for Config {
    fn default() -> Self {
        Self {
            registers: [0; 64],
            masks: [0; 6],
        }
    }
}

impl Config {
    fn standard() -> Self {
        let mut config = Self::default();
        config.registers[0x20 / 4] = 0x8000_0004;
        config.registers[0x24 / 4] = 0;
        config.masks[4] = 0xffff_c004;
        config.masks[5] = u32::MAX;
        config.begin(0x50);
        config.set_capability(0x50, 0x70, 16, 1, 4, 0, 0, 0x38, None);
        config.set_capability(0x70, 0x90, 20, 2, 4, 0, 0x100, 0x40, Some(4));
        config.set_capability(0x90, 0, 16, 3, 4, 0, 0x200, 1, None);
        config
    }

    fn begin(&mut self, first: u8) {
        self.registers[1] = 1 << 20;
        self.registers[0x34 / 4] = u32::from(first);
    }

    #[allow(clippy::too_many_arguments)]
    fn set_capability(
        &mut self,
        offset: u8,
        next: u8,
        capability_len: u8,
        configuration_type: u8,
        bar: u8,
        id: u8,
        region_offset: u32,
        region_length: u32,
        notify_multiplier: Option<u32>,
    ) {
        let index = usize::from(offset) / 4;
        self.registers[index] = u32::from(PCI_CAPABILITY_ID_VENDOR_SPECIFIC)
            | (u32::from(next) << 8)
            | (u32::from(capability_len) << 16)
            | (u32::from(configuration_type) << 24);
        self.registers[index + 1] = u32::from(bar) | (u32::from(id) << 8);
        self.registers[index + 2] = region_offset;
        self.registers[index + 3] = region_length;
        if let Some(multiplier) = notify_multiplier {
            self.registers[index + 4] = multiplier;
        }
    }
}

impl PciConfigAccess for Config {
    fn read_u32(&mut self, _address: PciFunctionAddress, register: PciConfigRegister) -> u32 {
        let offset = register.offset();
        let index = usize::from(offset) / 4;
        if (0x10..=0x24).contains(&offset) && self.registers[index] == u32::MAX {
            self.masks[usize::from((offset - 0x10) / 4)]
        } else {
            self.registers[index]
        }
    }
}

impl PciConfigMutationAccess for Config {
    fn write_u32(&mut self, _address: PciFunctionAddress, register: PciConfigRegister, value: u32) {
        let offset = register.offset();
        let index = usize::from(offset) / 4;
        if offset == 0x04 {
            let status = self.registers[index] & 0xffff_0000;
            self.registers[index] = status | (value & 0xffff);
        } else {
            self.registers[index] = value;
        }
    }
}
