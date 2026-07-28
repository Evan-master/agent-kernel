//! PCI command-register activation gate.
//!
//! This architecture module keeps memory decoding and bus mastering disabled
//! during DMA setup, then enables both through one verified mutation. Status
//! bits are never written back as ones.

use super::{PciConfigMutationAccess, PciConfigRegister, PciFunctionAddress};

const IO_SPACE: u16 = 1;
const MEMORY_SPACE: u16 = 1 << 1;
const BUS_MASTER: u16 = 1 << 2;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct PciCommandState {
    io_space: bool,
    memory_space: bool,
    bus_master: bool,
}

impl PciCommandState {
    pub const fn new(io_space: bool, memory_space: bool, bus_master: bool) -> Self {
        Self {
            io_space,
            memory_space,
            bus_master,
        }
    }

    pub const fn io_space(self) -> bool {
        self.io_space
    }

    pub const fn memory_space(self) -> bool {
        self.memory_space
    }

    pub const fn bus_master(self) -> bool {
        self.bus_master
    }

    const fn from_raw(raw: u16) -> Self {
        Self::new(
            raw & IO_SPACE != 0,
            raw & MEMORY_SPACE != 0,
            raw & BUS_MASTER != 0,
        )
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PciCommandGateError {
    VerificationFailed {
        expected: PciCommandState,
        actual: PciCommandState,
    },
}

pub struct PciCommandGate<A> {
    access: A,
    address: PciFunctionAddress,
}

impl<A: PciConfigMutationAccess> PciCommandGate<A> {
    pub const fn bind(access: A, address: PciFunctionAddress) -> Self {
        Self { access, address }
    }

    pub fn quiesce(&mut self) -> Result<PciCommandState, PciCommandGateError> {
        self.update(false)
    }

    pub fn enable_memory_and_bus_master(&mut self) -> Result<PciCommandState, PciCommandGateError> {
        self.update(true)
    }

    pub fn state(&mut self) -> PciCommandState {
        PciCommandState::from_raw(self.read_command())
    }

    pub fn into_access(self) -> A {
        self.access
    }

    fn update(&mut self, enabled: bool) -> Result<PciCommandState, PciCommandGateError> {
        let original = self.read_command();
        let next = if enabled {
            original | MEMORY_SPACE | BUS_MASTER
        } else {
            original & !(MEMORY_SPACE | BUS_MASTER)
        };
        let register = PciConfigRegister::new(0x04).expect("fixed command register");
        self.access
            .write_u32(self.address, register, u32::from(next));
        let actual = self.state();
        let expected = PciCommandState::new(original & IO_SPACE != 0, enabled, enabled);
        if actual != expected {
            return Err(PciCommandGateError::VerificationFailed { expected, actual });
        }
        Ok(actual)
    }

    fn read_command(&mut self) -> u16 {
        let register = PciConfigRegister::new(0x04).expect("fixed command register");
        self.access.read_u32(self.address, register) as u16
    }
}
