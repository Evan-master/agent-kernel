//! Deterministic fixed-capacity PCI function discovery.
//!
//! The scanner reads only the four common-header DWORDs needed to identify and
//! classify functions. It publishes no partial inventory after a failure.

use super::{PciConfigAccess, PciConfigRegister, PciFunction, PciFunctionAddress};

const ABSENT_VENDOR_ID: u16 = u16::MAX;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PciDiscoveryError {
    NoFunctions,
    InventoryFull {
        capacity: usize,
        address: PciFunctionAddress,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PciInventory<const CAPACITY: usize> {
    functions: [PciFunction; CAPACITY],
    len: usize,
}

impl<const CAPACITY: usize> PciInventory<CAPACITY> {
    const fn new() -> Self {
        Self {
            functions: [PciFunction::EMPTY; CAPACITY],
            len: 0,
        }
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn functions(&self) -> &[PciFunction] {
        &self.functions[..self.len]
    }

    pub fn find(&self, address: PciFunctionAddress) -> Option<PciFunction> {
        self.functions()
            .iter()
            .copied()
            .find(|function| function.address() == address)
    }

    fn push(&mut self, function: PciFunction) -> Result<(), PciDiscoveryError> {
        let Some(slot) = self.functions.get_mut(self.len) else {
            return Err(PciDiscoveryError::InventoryFull {
                capacity: CAPACITY,
                address: function.address(),
            });
        };
        *slot = function;
        self.len += 1;
        Ok(())
    }
}

pub fn discover_pci_functions<A: PciConfigAccess, const CAPACITY: usize>(
    access: &mut A,
) -> Result<PciInventory<CAPACITY>, PciDiscoveryError> {
    let mut inventory = PciInventory::new();
    for bus in 0..=u8::MAX {
        for device in 0..32 {
            let function_zero = PciFunctionAddress::from_scan(bus, device, 0);
            let identity = access.read_u32(function_zero, PciConfigRegister::IDENTITY);
            if identity as u16 == ABSENT_VENDOR_ID {
                continue;
            }
            let function = read_present_function(access, function_zero, identity);
            let multifunction = function.multifunction();
            inventory.push(function)?;
            if !multifunction {
                continue;
            }
            for number in 1..8 {
                let address = PciFunctionAddress::from_scan(bus, device, number);
                let identity = access.read_u32(address, PciConfigRegister::IDENTITY);
                if identity as u16 == ABSENT_VENDOR_ID {
                    continue;
                }
                inventory.push(read_present_function(access, address, identity))?;
            }
        }
    }
    if inventory.is_empty() {
        return Err(PciDiscoveryError::NoFunctions);
    }
    Ok(inventory)
}

fn read_present_function<A: PciConfigAccess>(
    access: &mut A,
    address: PciFunctionAddress,
    identity: u32,
) -> PciFunction {
    let command_status = access.read_u32(address, PciConfigRegister::COMMAND_STATUS);
    let class_revision = access.read_u32(address, PciConfigRegister::CLASS_REVISION);
    let header = access.read_u32(address, PciConfigRegister::HEADER);
    PciFunction::from_common_header(address, identity, command_status, class_revision, header)
}
