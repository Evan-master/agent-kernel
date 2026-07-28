use agent_kernel_x86_64::pci::{
    PciBarIndex, PciCapability, PciConfigAccess, PciConfigRegister, PciFunctionAddress,
    VirtioPciCapability, VirtioPciCapabilityError, VirtioPciCapabilityKind,
    PCI_CAPABILITY_ID_VENDOR_SPECIFIC,
};

const ADDRESS: PciFunctionAddress = match PciFunctionAddress::new(0, 6, 0) {
    Some(address) => address,
    None => panic!("fixed PCI address"),
};

#[test]
fn decodes_common_notify_and_isr_vendor_capabilities() {
    let mut config = Config::default();
    config.set_virtio_capability(0x60, 16, 1, 2, 0, 0x1000, 0x100, None);
    config.set_virtio_capability(0x70, 20, 2, 4, 0, 0x2000, 0x1000, Some(4));
    config.set_virtio_capability(0x90, 16, 3, 1, 0, 0x3000, 1, None);

    let common = VirtioPciCapability::decode(
        &mut config,
        ADDRESS,
        PciCapability::new(PCI_CAPABILITY_ID_VENDOR_SPECIFIC, 0x60).unwrap(),
    )
    .unwrap();
    assert_eq!(common.kind(), VirtioPciCapabilityKind::CommonConfiguration);
    assert_eq!(common.bar().number(), 2);
    assert_eq!(common.offset(), 0x1000);
    assert_eq!(common.length(), 0x100);
    assert_eq!(common.notify_offset_multiplier(), None);

    let notify = VirtioPciCapability::decode(
        &mut config,
        ADDRESS,
        PciCapability::new(PCI_CAPABILITY_ID_VENDOR_SPECIFIC, 0x70).unwrap(),
    )
    .unwrap();
    assert_eq!(notify.kind(), VirtioPciCapabilityKind::Notify);
    assert_eq!(notify.notify_offset_multiplier(), Some(4));

    let isr = VirtioPciCapability::decode(
        &mut config,
        ADDRESS,
        PciCapability::new(PCI_CAPABILITY_ID_VENDOR_SPECIFIC, 0x90).unwrap(),
    )
    .unwrap();
    assert_eq!(isr.kind(), VirtioPciCapabilityKind::Isr);
}

#[test]
fn rejects_short_notify_and_invalid_bar_descriptors() {
    let mut short = Config::default();
    short.set_virtio_capability(0x70, 16, 2, 4, 0, 0x2000, 0x1000, None);
    let record = PciCapability::new(PCI_CAPABILITY_ID_VENDOR_SPECIFIC, 0x70).unwrap();
    assert_eq!(
        VirtioPciCapability::decode(&mut short, ADDRESS, record),
        Err(VirtioPciCapabilityError::CapabilityTooShort {
            kind: VirtioPciCapabilityKind::Notify,
            required: 20,
            actual: 16,
        })
    );

    let mut short_pci_cfg = Config::default();
    short_pci_cfg.set_virtio_capability(0x70, 16, 5, 0, 0, 0, 0, None);
    assert_eq!(
        VirtioPciCapability::decode(&mut short_pci_cfg, ADDRESS, record),
        Err(VirtioPciCapabilityError::CapabilityTooShort {
            kind: VirtioPciCapabilityKind::PciConfiguration,
            required: 20,
            actual: 16,
        })
    );

    let mut invalid_bar = Config::default();
    invalid_bar.set_virtio_capability(0x70, 20, 2, 6, 0, 0x2000, 0x1000, Some(4));
    assert_eq!(
        VirtioPciCapability::decode(&mut invalid_bar, ADDRESS, record),
        Err(VirtioPciCapabilityError::InvalidBar { bar: 6 })
    );
}

#[test]
fn validates_the_capability_region_against_the_selected_bar() {
    let mut config = Config::default();
    config.set_virtio_capability(0x60, 16, 1, 2, 0, 0x1000, 0x100, None);
    let capability = VirtioPciCapability::decode(
        &mut config,
        ADDRESS,
        PciCapability::new(PCI_CAPABILITY_ID_VENDOR_SPECIFIC, 0x60).unwrap(),
    )
    .unwrap();

    let region = capability
        .bar_region(PciBarIndex::new(2).unwrap(), 0x1100)
        .unwrap();
    assert_eq!(region.offset(), 0x1000);
    assert_eq!(region.length(), 0x100);

    assert_eq!(
        capability.bar_region(PciBarIndex::new(2).unwrap(), 0x10ff),
        Err(VirtioPciCapabilityError::RegionOutsideBar {
            offset: 0x1000,
            length: 0x100,
            bar_size: 0x10ff,
        })
    );
}

#[test]
fn accepts_an_unprogrammed_pci_cfg_descriptor_without_exposing_a_bar_region() {
    let mut config = Config::default();
    config.set_virtio_capability(0x60, 20, 5, 0, 0, 0, 0, None);
    let capability = VirtioPciCapability::decode(
        &mut config,
        ADDRESS,
        PciCapability::new(PCI_CAPABILITY_ID_VENDOR_SPECIFIC, 0x60).unwrap(),
    )
    .unwrap();

    assert_eq!(capability.kind(), VirtioPciCapabilityKind::PciConfiguration);
    assert_eq!(
        capability.bar_region(PciBarIndex::new(0).unwrap(), 0x1000),
        Err(VirtioPciCapabilityError::EmptyRegion {
            kind: VirtioPciCapabilityKind::PciConfiguration,
        })
    );
}

struct Config {
    registers: [u32; 64],
}

impl Default for Config {
    fn default() -> Self {
        Self { registers: [0; 64] }
    }
}

impl Config {
    #[allow(clippy::too_many_arguments)]
    fn set_virtio_capability(
        &mut self,
        offset: u8,
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
        self.registers[usize::from(register.offset()) / 4]
    }
}
