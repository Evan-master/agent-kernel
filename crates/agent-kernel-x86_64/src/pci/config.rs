//! Exclusive PCI Configuration Access Mechanism 1 adapter.
//!
//! The adapter emits one validated 32-bit selector immediately before each
//! data read. Its I/O owner must not be shared with another configuration
//! accessor while a transaction is in progress.

use super::{PciConfigRegister, PciFunctionAddress};

const CONFIG_ENABLE: u32 = 1 << 31;
const PROBE_SELECTOR: u32 = CONFIG_ENABLE | 0xfc;

pub trait PciConfigIo {
    fn read_address(&mut self) -> u32;

    fn write_address(&mut self, value: u32);

    fn read_data(&mut self) -> u32;
}

pub trait PciConfigAccess {
    fn read_u32(&mut self, address: PciFunctionAddress, register: PciConfigRegister) -> u32;
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PciConfigMechanismOneError {
    AddressLatchUnavailable { expected: u32, actual: u32 },
}

pub struct PciConfigMechanismOne<I> {
    io: I,
}

impl<I> PciConfigMechanismOne<I> {
    pub const fn new(io: I) -> Self {
        Self { io }
    }

    pub fn into_io(self) -> I {
        self.io
    }
}

impl<I: PciConfigIo> PciConfigMechanismOne<I> {
    pub fn probe(&mut self) -> Result<(), PciConfigMechanismOneError> {
        let saved = self.io.read_address();
        self.io.write_address(PROBE_SELECTOR);
        let actual = self.io.read_address();
        self.io.write_address(saved);
        if actual != PROBE_SELECTOR {
            return Err(PciConfigMechanismOneError::AddressLatchUnavailable {
                expected: PROBE_SELECTOR,
                actual,
            });
        }
        Ok(())
    }
}

impl<I: PciConfigIo> PciConfigAccess for PciConfigMechanismOne<I> {
    fn read_u32(&mut self, address: PciFunctionAddress, register: PciConfigRegister) -> u32 {
        self.io
            .write_address(CONFIG_ENABLE | address.selector_bits() | u32::from(register.offset()));
        self.io.read_data()
    }
}
