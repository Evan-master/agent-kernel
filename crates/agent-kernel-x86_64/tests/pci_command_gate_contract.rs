use agent_kernel_x86_64::pci::{
    PciCommandGate, PciCommandState, PciConfigAccess, PciConfigMutationAccess, PciConfigRegister,
    PciFunctionAddress,
};

#[test]
fn command_gate_keeps_bus_master_off_until_explicit_activation() {
    let address = PciFunctionAddress::new(0, 5, 0).unwrap();
    let mut gate = PciCommandGate::bind(Config { command: 0x0007 }, address);

    assert_eq!(
        gate.quiesce().unwrap(),
        PciCommandState::new(true, false, false)
    );
    assert_eq!(
        gate.enable_memory_and_bus_master().unwrap(),
        PciCommandState::new(true, true, true)
    );
    assert_eq!(
        gate.quiesce().unwrap(),
        PciCommandState::new(true, false, false)
    );
}

struct Config {
    command: u16,
}

impl PciConfigAccess for Config {
    fn read_u32(&mut self, _address: PciFunctionAddress, _register: PciConfigRegister) -> u32 {
        u32::from(self.command)
    }
}

impl PciConfigMutationAccess for Config {
    fn write_u32(
        &mut self,
        _address: PciFunctionAddress,
        _register: PciConfigRegister,
        value: u32,
    ) {
        self.command = value as u16;
    }
}
