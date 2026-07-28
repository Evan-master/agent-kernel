//! Width-aware PCI configuration-space field access.
//!
//! This private architecture helper keeps sub-dword capability mutations from
//! changing adjacent fields. Callers validate offsets before using it.

use super::{PciConfigAccess, PciConfigMutationAccess, PciConfigRegister, PciFunctionAddress};

pub(super) fn read_u16<A: PciConfigAccess>(
    access: &mut A,
    address: PciFunctionAddress,
    offset: u16,
) -> u16 {
    let aligned = offset & !3;
    let shift = u32::from(offset & 2) * 8;
    let register = PciConfigRegister::new(aligned).expect("validated PCI field offset");
    (access.read_u32(address, register) >> shift) as u16
}

pub(super) fn write_u16<A: PciConfigMutationAccess>(
    access: &mut A,
    address: PciFunctionAddress,
    offset: u16,
    value: u16,
) {
    let aligned = offset & !3;
    let shift = u32::from(offset & 2) * 8;
    let register = PciConfigRegister::new(aligned).expect("validated PCI field offset");
    let original = access.read_u32(address, register);
    let mask = u32::from(u16::MAX) << shift;
    let next = (original & !mask) | (u32::from(value) << shift);
    access.write_u32(address, register, next);
}

pub(super) fn read_u32<A: PciConfigAccess>(
    access: &mut A,
    address: PciFunctionAddress,
    offset: u16,
) -> u32 {
    let register = PciConfigRegister::new(offset).expect("validated PCI field offset");
    access.read_u32(address, register)
}

pub(super) fn write_u32<A: PciConfigMutationAccess>(
    access: &mut A,
    address: PciFunctionAddress,
    offset: u16,
    value: u32,
) {
    let register = PciConfigRegister::new(offset).expect("validated PCI field offset");
    access.write_u32(address, register, value);
}
