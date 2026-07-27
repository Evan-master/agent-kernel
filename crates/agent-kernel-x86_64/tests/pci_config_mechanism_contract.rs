use agent_kernel_x86_64::pci::{
    PciConfigAccess, PciConfigIo, PciConfigMechanismOne, PciConfigMechanismOneError,
    PciConfigRegister, PciFunctionAddress,
};

#[derive(Default)]
struct RecordingConfigIo {
    address: u32,
    forced_address_read: Option<u32>,
    address_read_count: usize,
    data: u32,
    address_writes: Vec<u32>,
}

impl PciConfigIo for RecordingConfigIo {
    fn read_address(&mut self) -> u32 {
        self.address_read_count += 1;
        if self.address_read_count > 1 {
            self.forced_address_read.unwrap_or(self.address)
        } else {
            self.address
        }
    }

    fn write_address(&mut self, value: u32) {
        self.address = value;
        self.address_writes.push(value);
    }

    fn read_data(&mut self) -> u32 {
        self.data
    }
}

#[test]
fn pci_coordinates_and_registers_enforce_mechanism_one_bounds() {
    assert_eq!(
        PciFunctionAddress::new(0x5a, 0x1f, 7)
            .unwrap()
            .coordinates(),
        (0x5a, 0x1f, 7)
    );
    assert!(PciFunctionAddress::new(0, 32, 0).is_none());
    assert!(PciFunctionAddress::new(0, 0, 8).is_none());

    assert_eq!(PciConfigRegister::new(0xfc).unwrap().offset(), 0xfc);
    assert!(PciConfigRegister::new(0x02).is_none());
    assert!(PciConfigRegister::new(0x100).is_none());
}

#[test]
fn configuration_read_emits_one_exact_selector_then_reads_data() {
    let io = RecordingConfigIo {
        data: 0x1234_8086,
        ..RecordingConfigIo::default()
    };
    let mut config = PciConfigMechanismOne::new(io);
    let address = PciFunctionAddress::new(0x5a, 0x1b, 6).unwrap();
    let register = PciConfigRegister::new(0xdc).unwrap();

    assert_eq!(config.read_u32(address, register), 0x1234_8086);

    let io = config.into_io();
    assert_eq!(io.address_writes, [0x805a_dedc]);
}

#[test]
fn address_latch_probe_restores_the_previous_selector() {
    let saved = 0x8123_4000;
    let io = RecordingConfigIo {
        address: saved,
        ..RecordingConfigIo::default()
    };
    let mut config = PciConfigMechanismOne::new(io);

    config.probe().unwrap();

    let io = config.into_io();
    assert_eq!(io.address, saved);
    assert_eq!(io.address_writes, [0x8000_00fc, saved]);
}

#[test]
fn failed_address_latch_probe_also_restores_the_previous_selector() {
    let saved = 0x8abc_d000;
    let io = RecordingConfigIo {
        address: saved,
        forced_address_read: Some(0),
        ..RecordingConfigIo::default()
    };
    let mut config = PciConfigMechanismOne::new(io);

    assert_eq!(
        config.probe(),
        Err(PciConfigMechanismOneError::AddressLatchUnavailable {
            expected: 0x8000_00fc,
            actual: 0,
        })
    );

    let io = config.into_io();
    assert_eq!(io.address, saved);
    assert_eq!(io.address_writes, [0x8000_00fc, saved]);
}
