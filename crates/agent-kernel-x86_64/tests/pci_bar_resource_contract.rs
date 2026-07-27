use agent_kernel_x86_64::pci::{
    probe_pci_function_bars, PciBarIndex, PciBarKind, PciBarProbeError, PciConfigAccess,
    PciConfigMutationAccess, PciConfigRegister, PciFunctionAddress,
};

const FUNCTION: PciFunctionAddress = match PciFunctionAddress::new(0, 2, 0) {
    Some(address) => address,
    None => panic!("test address must be valid"),
};

struct BarConfig {
    command_status: u32,
    bars: [u32; 6],
    masks: [u32; 6],
    writes: Vec<(u8, u32)>,
    ignore_decode_disable: bool,
    corrupt_restore: Option<u8>,
}

impl BarConfig {
    fn representative() -> Self {
        Self {
            command_status: 0xa5a5_0007,
            bars: [
                0x8000_0008,
                0x0000_c001,
                0x0000_000c,
                0x0000_0001,
                0x000c_0002,
                0,
            ],
            masks: [
                0xffff_f008,
                0xffff_ff01,
                0xffe0_000c,
                0xffff_ffff,
                0x000f_0002,
                0,
            ],
            writes: Vec::new(),
            ignore_decode_disable: false,
            corrupt_restore: None,
        }
    }

    fn read_register(&self, offset: u8) -> u32 {
        match offset {
            0x04 => self.command_status,
            0x10..=0x24 if offset & 3 == 0 => {
                let index = usize::from((offset - 0x10) / 4);
                if self.bars[index] == u32::MAX {
                    self.masks[index]
                } else {
                    self.bars[index]
                }
            }
            _ => 0,
        }
    }
}

impl PciConfigAccess for BarConfig {
    fn read_u32(&mut self, address: PciFunctionAddress, register: PciConfigRegister) -> u32 {
        assert_eq!(address, FUNCTION);
        self.read_register(register.offset())
    }
}

impl PciConfigMutationAccess for BarConfig {
    fn write_u32(&mut self, address: PciFunctionAddress, register: PciConfigRegister, value: u32) {
        assert_eq!(address, FUNCTION);
        let offset = register.offset();
        self.writes.push((offset, value));
        match offset {
            0x04 => {
                if self.ignore_decode_disable && value & 0x7 == 0 {
                    return;
                }
                let status = (self.command_status >> 16) as u16 & !((value >> 16) as u16);
                self.command_status = (u32::from(status) << 16) | u32::from(value as u16);
            }
            0x10..=0x24 if offset & 3 == 0 => {
                let index = usize::from((offset - 0x10) / 4);
                self.bars[index] = if self.corrupt_restore == Some(offset) && value != u32::MAX {
                    value ^ 0x1000
                } else {
                    value
                };
            }
            _ => panic!("unexpected configuration write"),
        }
    }
}

#[test]
fn bar_probe_sizes_all_supported_shapes_and_restores_hardware_state() {
    let mut config = BarConfig::representative();
    let original_command_status = config.command_status;
    let original_bars = config.bars;

    let bars = probe_pci_function_bars(&mut config, FUNCTION, 0).unwrap();

    assert_eq!(bars.len(), 4);
    let memory32 = bars.get(PciBarIndex::new(0).unwrap()).unwrap();
    assert_eq!(memory32.kind(), PciBarKind::Memory32 { prefetchable: true });
    assert_eq!((memory32.base(), memory32.size()), (0x8000_0000, 0x1000));

    let io = bars.get(PciBarIndex::new(1).unwrap()).unwrap();
    assert_eq!(io.kind(), PciBarKind::Io);
    assert_eq!((io.base(), io.size()), (0xc000, 0x100));

    let memory64 = bars.get(PciBarIndex::new(2).unwrap()).unwrap();
    assert_eq!(memory64.kind(), PciBarKind::Memory64 { prefetchable: true });
    assert_eq!(
        (memory64.base(), memory64.size()),
        (0x1_0000_0000, 0x20_0000)
    );
    assert!(bars.get(PciBarIndex::new(3).unwrap()).is_none());

    let legacy = bars.get(PciBarIndex::new(4).unwrap()).unwrap();
    assert_eq!(
        legacy.kind(),
        PciBarKind::MemoryBelowOneMegabyte {
            prefetchable: false
        }
    );
    assert_eq!((legacy.base(), legacy.size()), (0xc_0000, 0x1_0000));

    assert_eq!(config.command_status, original_command_status);
    assert_eq!(config.bars, original_bars);
    assert_eq!(config.writes.first(), Some(&(0x04, 0)));
    assert_eq!(config.writes.last(), Some(&(0x04, 0x0007)));
    assert!(config
        .writes
        .iter()
        .filter(|(offset, _)| *offset == 0x04)
        .all(|(_, value)| value >> 16 == 0));
}

#[test]
fn decode_disable_failure_restores_command_and_never_touches_a_bar() {
    let mut config = BarConfig::representative();
    config.ignore_decode_disable = true;
    let original = config.command_status;

    assert_eq!(
        probe_pci_function_bars(&mut config, FUNCTION, 0),
        Err(PciBarProbeError::CommandDecodeDisableMismatch {
            expected: 0,
            actual: 7,
        })
    );

    assert_eq!(config.command_status, original);
    assert_eq!(config.writes, [(0x04, 0), (0x04, original & 0xffff)]);
}

#[test]
fn malformed_bar_and_restore_failure_abort_after_restoring_command_state() {
    let mut reserved = BarConfig::representative();
    reserved.bars[0] = 0x8000_0006;
    reserved.masks[0] = 0xffff_f006;
    let original_command = reserved.command_status;

    assert_eq!(
        probe_pci_function_bars(&mut reserved, FUNCTION, 0),
        Err(PciBarProbeError::ReservedMemoryType {
            index: PciBarIndex::new(0).unwrap(),
        })
    );
    assert_eq!(reserved.command_status, original_command);
    assert_eq!(reserved.bars[0], 0x8000_0006);

    let mut corrupt = BarConfig::representative();
    corrupt.corrupt_restore = Some(0x10);
    let original_command = corrupt.command_status;
    assert_eq!(
        probe_pci_function_bars(&mut corrupt, FUNCTION, 0),
        Err(PciBarProbeError::RestoreMismatch {
            register: PciConfigRegister::new(0x10).unwrap(),
            expected: 0x8000_0008,
            actual: 0x8000_1008,
        })
    );
    assert_eq!(corrupt.command_status, original_command);
    assert_eq!(corrupt.writes.last(), Some(&(0x04, 0x0007)));
}

#[test]
fn non_endpoint_headers_and_unpaired_64_bit_bars_fail_closed() {
    let mut bridge = BarConfig::representative();
    assert_eq!(
        probe_pci_function_bars(&mut bridge, FUNCTION, 1),
        Err(PciBarProbeError::UnsupportedHeaderType { header_type: 1 })
    );
    assert!(bridge.writes.is_empty());

    let mut unpaired = BarConfig::representative();
    unpaired.bars = [0, 0, 0, 0, 0, 0x0000_0004];
    unpaired.masks = [0, 0, 0, 0, 0, 0xffff_f004];
    let original_command = unpaired.command_status;
    assert_eq!(
        probe_pci_function_bars(&mut unpaired, FUNCTION, 0),
        Err(PciBarProbeError::Unpaired64BitBar {
            index: PciBarIndex::new(5).unwrap(),
        })
    );
    assert_eq!(unpaired.command_status, original_command);
    assert_eq!(unpaired.bars[5], 0x0000_0004);
}
