use agent_kernel_x86_64::{
    apic::ApicVector,
    cpu::ApicId,
    pci::{
        MsiCapability, MsiError, MsiRegister, PciCapability, PciConfigAccess,
        PciConfigMutationAccess, PciConfigRegister, PciFunctionAddress, XapicMsiMessage,
        XapicMsiMessageError, PCI_CAPABILITY_ID_MSI,
    },
};

const ADDRESS: PciFunctionAddress = match PciFunctionAddress::new(0, 5, 0) {
    Some(address) => address,
    None => panic!("fixed PCI address"),
};

#[test]
fn encodes_the_v28_physical_xapic_message_profile() {
    let bsp = XapicMsiMessage::new(ApicId::new(0), ApicVector::new(0xd0).unwrap()).unwrap();
    assert_eq!(bsp.address(), 0xfee0_0000);
    assert_eq!(bsp.data(), 0x0000_00d0);

    let ap2 = XapicMsiMessage::new(ApicId::new(2), ApicVector::new(0xd1).unwrap()).unwrap();
    assert_eq!(ap2.address(), 0xfee0_2000);
    assert_eq!(ap2.data(), 0x0000_00d1);
}

#[test]
fn rejects_destinations_and_vectors_outside_the_v28_profile() {
    assert_eq!(
        XapicMsiMessage::new(ApicId::new(256), ApicVector::new(0xd0).unwrap()),
        Err(XapicMsiMessageError::DestinationOutOfRange { apic_id: 256 })
    );
    assert_eq!(
        XapicMsiMessage::new(ApicId::new(0), ApicVector::new(0xe0).unwrap()),
        Err(XapicMsiMessageError::VectorOutOfRange { vector: 0xe0 })
    );
}

#[test]
fn programs_a_64_bit_single_message_while_disabled_then_enables_it() {
    let mut config = Config::with_msi_control(0x00b5);
    config.registers[0x5c / 4] = 0xa5a5_0000;
    let record = PciCapability::new(PCI_CAPABILITY_ID_MSI, 0x50).unwrap();
    let capability = MsiCapability::decode(&mut config, ADDRESS, record).unwrap();
    let message = XapicMsiMessage::new(ApicId::new(2), ApicVector::new(0xd0).unwrap()).unwrap();

    capability.configure(&mut config, ADDRESS, message).unwrap();

    assert_eq!(config.writes[0].offset, 0x50);
    assert_eq!(config.writes[0].value >> 16 & 1, 0);
    assert_eq!(config.writes[0].value >> 20 & 0x7, 0);
    assert_eq!(config.registers[0x54 / 4], 0xfee0_2000);
    assert_eq!(config.registers[0x58 / 4], 0);
    assert_eq!(config.registers[0x5c / 4], 0xa5a5_00d0);
    assert_eq!(config.registers[0x50 / 4] >> 16 & 1, 1);
    assert_eq!(config.writes.last().unwrap().offset, 0x50);

    capability.disable(&mut config, ADDRESS).unwrap();
    assert_eq!(config.registers[0x50 / 4] >> 16 & 1, 0);
}

#[test]
fn rejects_a_32_bit_layout_before_any_configuration_write() {
    let mut config = Config::with_msi_control(0);
    let record = PciCapability::new(PCI_CAPABILITY_ID_MSI, 0x50).unwrap();

    assert_eq!(
        MsiCapability::decode(&mut config, ADDRESS, record),
        Err(MsiError::UnsupportedAddressWidth)
    );
    assert!(config.writes.is_empty());
}

#[test]
fn leaves_msi_disabled_when_message_readback_fails() {
    let mut config = Config::with_msi_control(1 << 7);
    config.ignore_offset = Some(0x54);
    let record = PciCapability::new(PCI_CAPABILITY_ID_MSI, 0x50).unwrap();
    let capability = MsiCapability::decode(&mut config, ADDRESS, record).unwrap();
    let message = XapicMsiMessage::new(ApicId::new(0), ApicVector::new(0xd0).unwrap()).unwrap();

    assert_eq!(
        capability.configure(&mut config, ADDRESS, message),
        Err(MsiError::VerificationFailed {
            register: MsiRegister::MessageAddressLow,
            expected: 0xfee0_0000,
            actual: 0,
        })
    );
    assert_eq!(config.registers[0x50 / 4] >> 16 & 1, 0);
}

#[test]
fn unmasks_vector_zero_before_enabling_a_per_vector_masking_layout() {
    let mut config = Config::with_msi_control((1 << 7) | (1 << 8));
    config.registers[0x60 / 4] = u32::MAX;
    let record = PciCapability::new(PCI_CAPABILITY_ID_MSI, 0x50).unwrap();
    let capability = MsiCapability::decode(&mut config, ADDRESS, record).unwrap();
    let message = XapicMsiMessage::new(ApicId::new(0), ApicVector::new(0xd0).unwrap()).unwrap();

    capability.configure(&mut config, ADDRESS, message).unwrap();

    assert!(capability.per_vector_masking());
    assert_eq!(config.registers[0x60 / 4], !1u32);
    assert_eq!(config.registers[0x50 / 4] >> 16 & 1, 1);
}

#[test]
fn rejects_a_per_vector_masking_layout_that_overruns_configuration_space() {
    let mut config = Config::with_msi_control(0);
    config.registers[0xec / 4] =
        u32::from(PCI_CAPABILITY_ID_MSI) | (u32::from((1_u16 << 7) | (1_u16 << 8)) << 16);
    let record = PciCapability::new(PCI_CAPABILITY_ID_MSI, 0xec).unwrap();

    assert_eq!(
        MsiCapability::decode(&mut config, ADDRESS, record),
        Err(MsiError::CapabilityOutOfRange { offset: 0xec })
    );
}

#[derive(Copy, Clone, Debug)]
struct Write {
    offset: u8,
    value: u32,
}

struct Config {
    registers: [u32; 64],
    writes: std::vec::Vec<Write>,
    ignore_offset: Option<u8>,
}

impl Config {
    fn with_msi_control(control: u16) -> Self {
        let mut registers = [0; 64];
        registers[0x50 / 4] = u32::from(PCI_CAPABILITY_ID_MSI) | (u32::from(control) << 16);
        Self {
            registers,
            writes: std::vec::Vec::new(),
            ignore_offset: None,
        }
    }
}

impl PciConfigAccess for Config {
    fn read_u32(&mut self, _address: PciFunctionAddress, register: PciConfigRegister) -> u32 {
        self.registers[usize::from(register.offset()) / 4]
    }
}

impl PciConfigMutationAccess for Config {
    fn write_u32(&mut self, _address: PciFunctionAddress, register: PciConfigRegister, value: u32) {
        self.writes.push(Write {
            offset: register.offset(),
            value,
        });
        if self.ignore_offset != Some(register.offset()) {
            self.registers[usize::from(register.offset()) / 4] = value;
        }
    }
}
