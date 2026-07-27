use agent_kernel_x86_64::pci::{
    discover_pci_functions, PciClassCode, PciConfigAccess, PciConfigRegister, PciDiscoveryError,
    PciFunctionAddress,
};

#[derive(Default)]
struct ScriptedConfig {
    values: Vec<(PciFunctionAddress, u8, u32)>,
    reads: Vec<(PciFunctionAddress, u8)>,
}

impl ScriptedConfig {
    fn install_function(
        &mut self,
        address: PciFunctionAddress,
        identity: (u16, u16),
        command_status: (u16, u16),
        class: PciClassCode,
        revision: u8,
        header: u8,
    ) {
        self.set(
            address,
            0x00,
            u32::from(identity.0) | (u32::from(identity.1) << 16),
        );
        self.set(
            address,
            0x04,
            u32::from(command_status.0) | (u32::from(command_status.1) << 16),
        );
        self.set(
            address,
            0x08,
            u32::from(revision)
                | (u32::from(class.programming_interface()) << 8)
                | (u32::from(class.subclass()) << 16)
                | (u32::from(class.base()) << 24),
        );
        self.set(address, 0x0c, u32::from(header) << 16);
    }

    fn set(&mut self, address: PciFunctionAddress, register: u8, value: u32) {
        self.values.push((address, register, value));
    }

    fn was_read(&self, address: PciFunctionAddress, register: u8) -> bool {
        self.reads.contains(&(address, register))
    }
}

impl PciConfigAccess for ScriptedConfig {
    fn read_u32(&mut self, address: PciFunctionAddress, register: PciConfigRegister) -> u32 {
        let offset = register.offset();
        self.reads.push((address, offset));
        self.values
            .iter()
            .find_map(|(candidate, candidate_offset, value)| {
                (*candidate == address && *candidate_offset == offset).then_some(*value)
            })
            .unwrap_or(if offset == 0 { u32::MAX } else { 0 })
    }
}

#[test]
fn discovery_decodes_functions_in_stable_bdf_order() {
    let mut config = representative_fabric();

    let inventory = discover_pci_functions::<_, 8>(&mut config).unwrap();

    assert_eq!(inventory.len(), 4);
    let functions = inventory.functions();
    assert_eq!(
        functions
            .iter()
            .map(|function| function.address().coordinates())
            .collect::<Vec<_>>(),
        [(0, 0, 0), (0, 1, 0), (0, 1, 2), (2, 5, 0)]
    );

    let host = functions[0];
    assert_eq!((host.vendor_id(), host.device_id()), (0x8086, 0x1237));
    assert_eq!((host.command(), host.status()), (0x0007, 0x0210));
    assert_eq!(host.revision_id(), 2);
    assert_eq!(host.header_type(), 0);
    assert!(!host.multifunction());

    let network = functions[1];
    assert!(network.class().is_network_controller());
    assert!(network.multifunction());

    let usb = functions[2];
    assert!(usb.class().is_usb_controller());
    assert_eq!(usb.class().programming_interface(), 0x20);

    let display = functions[3];
    assert!(display.class().is_display_controller());
    assert_eq!(
        inventory.find(PciFunctionAddress::new(2, 5, 0).unwrap()),
        Some(display)
    );
    assert!(!config.was_read(PciFunctionAddress::new(2, 5, 1).unwrap(), 0x00));
}

#[test]
fn discovery_fails_closed_when_no_function_exists() {
    assert_eq!(
        discover_pci_functions::<_, 4>(&mut ScriptedConfig::default()),
        Err(PciDiscoveryError::NoFunctions)
    );
}

#[test]
fn discovery_rejects_an_inventory_that_cannot_hold_every_function() {
    let mut config = representative_fabric();

    assert_eq!(
        discover_pci_functions::<_, 3>(&mut config),
        Err(PciDiscoveryError::InventoryFull {
            capacity: 3,
            address: PciFunctionAddress::new(2, 5, 0).unwrap(),
        })
    );
}

fn representative_fabric() -> ScriptedConfig {
    let mut config = ScriptedConfig::default();
    config.install_function(
        PciFunctionAddress::new(0, 0, 0).unwrap(),
        (0x8086, 0x1237),
        (0x0007, 0x0210),
        PciClassCode::new(0x06, 0x00, 0x00),
        2,
        0,
    );
    config.install_function(
        PciFunctionAddress::new(0, 1, 0).unwrap(),
        (0x1af4, 0x1000),
        (0x0003, 0x0010),
        PciClassCode::new(0x02, 0x00, 0x00),
        1,
        0x80,
    );
    config.install_function(
        PciFunctionAddress::new(0, 1, 2).unwrap(),
        (0x8086, 0x7020),
        (0x0001, 0x0080),
        PciClassCode::new(0x0c, 0x03, 0x20),
        4,
        0,
    );
    config.install_function(
        PciFunctionAddress::new(2, 5, 0).unwrap(),
        (0x1234, 0x1111),
        (0x0002, 0x0000),
        PciClassCode::new(0x03, 0x00, 0x00),
        3,
        0,
    );
    config
}
