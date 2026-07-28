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

#[test]
fn command_gate_disables_intx_without_changing_decode_or_bus_master_bits() {
    let address = PciFunctionAddress::new(0, 5, 0).unwrap();
    let mut gate = PciCommandGate::bind(Config { command: 0x0007 }, address);

    let state = gate.disable_intx().unwrap();

    assert!(state.io_space());
    assert!(state.memory_space());
    assert!(state.bus_master());
    assert!(state.intx_disabled());
}

#[test]
fn command_gate_can_open_mmio_while_bus_master_remains_quiesced() {
    let address = PciFunctionAddress::new(0, 6, 0).unwrap();
    let mut gate = PciCommandGate::bind(Config { command: 0x0005 }, address);

    let state = gate.enable_memory_decode().unwrap();

    assert!(state.io_space());
    assert!(state.memory_space());
    assert!(!state.bus_master());
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
