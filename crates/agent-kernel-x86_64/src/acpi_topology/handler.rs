//! Bounded direct-map handler for read-only ACPI table discovery.
//!
//! The handler translates physical addresses through one caller-provided
//! virtual offset and rejects mappings outside the declared physical window.
//! AML, port, PCI, timing, and arbitrary register operations are unavailable.

use core::{mem, ptr::NonNull};

use acpi::{Handler, PciAddress, PhysicalMapping};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DirectAcpiHandler {
    virtual_offset: usize,
    physical_length: usize,
}

impl DirectAcpiHandler {
    /// Construct a direct physical mapping window.
    ///
    /// # Safety
    ///
    /// For the handler's complete lifetime, `virtual_offset + physical` must
    /// address readable memory for every physical byte below `physical_length`
    /// that firmware tables can name.
    pub const unsafe fn new(virtual_offset: usize, physical_length: usize) -> Self {
        Self {
            virtual_offset,
            physical_length,
        }
    }

    pub const fn virtual_offset(self) -> usize {
        self.virtual_offset
    }

    pub const fn physical_length(self) -> usize {
        self.physical_length
    }
}

impl Handler for DirectAcpiHandler {
    unsafe fn map_physical_region<T>(
        &self,
        physical_address: usize,
        size: usize,
    ) -> PhysicalMapping<Self, T> {
        assert!(
            size >= mem::size_of::<T>(),
            "ACPI mapping is smaller than its value type"
        );
        let physical_end = physical_address
            .checked_add(size)
            .expect("ACPI physical mapping overflow");
        assert!(
            physical_end <= self.physical_length,
            "ACPI physical mapping exceeds the declared window"
        );
        let virtual_address = self
            .virtual_offset
            .checked_add(physical_address)
            .expect("ACPI virtual mapping overflow");
        let virtual_start = NonNull::new(virtual_address as *mut T)
            .expect("ACPI physical mapping resolved to null");
        PhysicalMapping {
            physical_start: physical_address,
            virtual_start,
            region_length: size,
            mapped_length: size,
            handler: *self,
        }
    }

    fn unmap_physical_region<T>(_region: &PhysicalMapping<Self, T>) {}

    fn read_u8(&self, _address: usize) -> u8 {
        unsupported()
    }

    fn read_u16(&self, _address: usize) -> u16 {
        unsupported()
    }

    fn read_u32(&self, _address: usize) -> u32 {
        unsupported()
    }

    fn read_u64(&self, _address: usize) -> u64 {
        unsupported()
    }

    fn write_u8(&self, _address: usize, _value: u8) {
        unsupported()
    }

    fn write_u16(&self, _address: usize, _value: u16) {
        unsupported()
    }

    fn write_u32(&self, _address: usize, _value: u32) {
        unsupported()
    }

    fn write_u64(&self, _address: usize, _value: u64) {
        unsupported()
    }

    fn read_io_u8(&self, _port: u16) -> u8 {
        unsupported()
    }

    fn read_io_u16(&self, _port: u16) -> u16 {
        unsupported()
    }

    fn read_io_u32(&self, _port: u16) -> u32 {
        unsupported()
    }

    fn write_io_u8(&self, _port: u16, _value: u8) {
        unsupported()
    }

    fn write_io_u16(&self, _port: u16, _value: u16) {
        unsupported()
    }

    fn write_io_u32(&self, _port: u16, _value: u32) {
        unsupported()
    }

    fn read_pci_u8(&self, _address: PciAddress, _offset: u16) -> u8 {
        unsupported()
    }

    fn read_pci_u16(&self, _address: PciAddress, _offset: u16) -> u16 {
        unsupported()
    }

    fn read_pci_u32(&self, _address: PciAddress, _offset: u16) -> u32 {
        unsupported()
    }

    fn write_pci_u8(&self, _address: PciAddress, _offset: u16, _value: u8) {
        unsupported()
    }

    fn write_pci_u16(&self, _address: PciAddress, _offset: u16, _value: u16) {
        unsupported()
    }

    fn write_pci_u32(&self, _address: PciAddress, _offset: u16, _value: u32) {
        unsupported()
    }

    fn nanos_since_boot(&self) -> u64 {
        unsupported()
    }

    fn stall(&self, _microseconds: u64) {
        unsupported()
    }

    fn sleep(&self, _milliseconds: u64) {
        unsupported()
    }
}

fn unsupported() -> ! {
    panic!("operation unavailable in read-only ACPI table discovery")
}
