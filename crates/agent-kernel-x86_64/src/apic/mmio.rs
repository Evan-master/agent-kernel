//! Restricted Local APIC and I/O APIC MMIO execution.
//!
//! Controllers consume validated register, command, and redirection values.
//! I/O APIC selector access requires `&mut self` for caller-visible
//! serialization; a shared architecture lock is still required across owners.

use crate::cpu::ApicId;

use super::{
    ApicVector, IcrCommand, IoApicRedirectionEntry, IoApicRedirectionIndex, IoApicVersion,
    LocalApicBase, LocalApicRegister,
};

const APIC_SOFTWARE_ENABLE: u32 = 1 << 8;
const IO_APIC_SELECTOR_OFFSET: u64 = 0;
const IO_APIC_WINDOW_OFFSET: u64 = 0x10;
const IO_APIC_VERSION_REGISTER: u8 = 1;

/// Raw 32-bit access to an already mapped APIC MMIO address.
///
/// # Safety
///
/// Implementations must perform exactly one 32-bit access at `address`, retain
/// volatile device semantics, and never redirect the operation to unrelated
/// memory or I/O authority.
pub unsafe trait Mmio32 {
    /// # Safety
    ///
    /// `address` must be a readable, aligned 32-bit register in this backend's
    /// mapped device domain.
    unsafe fn read32(&self, address: u64) -> u32;

    /// # Safety
    ///
    /// `address` must be a writable, aligned 32-bit register in this backend's
    /// mapped device domain and `value` must satisfy that register's contract.
    unsafe fn write32(&self, address: u64, value: u32);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApicMmioError {
    IcrBusy,
}

pub struct LocalApicMmio<B> {
    virtual_base: u64,
    backend: B,
}

impl<B: Mmio32> LocalApicMmio<B> {
    pub fn new(base: LocalApicBase, physical_offset: u64, backend: B) -> Option<Self> {
        Some(Self {
            virtual_base: base.virtual_address(physical_offset)?,
            backend,
        })
    }

    pub fn id(&self) -> ApicId {
        ApicId::new(self.read(LocalApicRegister::Id) >> 24)
    }

    pub fn version_raw(&self) -> u32 {
        self.read(LocalApicRegister::Version)
    }

    pub fn enable(&mut self, spurious_vector: ApicVector) {
        self.write(LocalApicRegister::TaskPriority, 0);
        self.write(
            LocalApicRegister::Spurious,
            APIC_SOFTWARE_ENABLE | spurious_vector.get() as u32,
        );
    }

    pub fn try_send(&mut self, command: IcrCommand) -> Result<(), ApicMmioError> {
        let current = self.read(LocalApicRegister::InterruptCommandLow);
        if IcrCommand::delivery_pending(current) {
            return Err(ApicMmioError::IcrBusy);
        }
        self.write(LocalApicRegister::InterruptCommandHigh, command.high());
        self.write(LocalApicRegister::InterruptCommandLow, command.low());
        Ok(())
    }

    pub fn delivery_pending(&self) -> bool {
        IcrCommand::delivery_pending(self.read(LocalApicRegister::InterruptCommandLow))
    }

    pub fn end_of_interrupt(&mut self) {
        self.write(LocalApicRegister::EndOfInterrupt, 0);
    }

    pub const fn virtual_base(&self) -> u64 {
        self.virtual_base
    }

    pub const fn backend(&self) -> &B {
        &self.backend
    }

    fn read(&self, register: LocalApicRegister) -> u32 {
        let address = self.virtual_base + register.offset() as u64;
        // SAFETY: LocalApicBase and fixed register offsets stay within the
        // mapped Local APIC page and all registers are 16-byte aligned.
        unsafe { self.backend.read32(address) }
    }

    fn write(&mut self, register: LocalApicRegister, value: u32) {
        let address = self.virtual_base + register.offset() as u64;
        // SAFETY: value construction is owned by the controller methods and
        // the fixed register address is writable in the Local APIC page.
        unsafe { self.backend.write32(address, value) };
    }
}

pub struct IoApicMmio<B> {
    virtual_base: u64,
    backend: B,
}

impl<B: Mmio32> IoApicMmio<B> {
    pub fn new(physical_base: u64, physical_offset: u64, backend: B) -> Option<Self> {
        let base = LocalApicBase::new(physical_base)?;
        Some(Self {
            virtual_base: base.virtual_address(physical_offset)?,
            backend,
        })
    }

    pub fn version(&mut self) -> IoApicVersion {
        IoApicVersion::from_raw(self.read_indirect(IO_APIC_VERSION_REGISTER))
    }

    pub fn write_redirection(
        &mut self,
        index: IoApicRedirectionIndex,
        entry: IoApicRedirectionEntry,
    ) {
        self.write_indirect(index.low_register(), entry.with_masked(true).low());
        self.write_indirect(index.high_register(), entry.high());
        self.write_indirect(index.low_register(), entry.low());
    }

    pub const fn virtual_base(&self) -> u64 {
        self.virtual_base
    }

    pub const fn backend(&self) -> &B {
        &self.backend
    }

    fn read_indirect(&mut self, register: u8) -> u32 {
        self.write_selector(register);
        // SAFETY: the I/O APIC window is a mapped aligned 32-bit register.
        unsafe {
            self.backend
                .read32(self.virtual_base + IO_APIC_WINDOW_OFFSET)
        }
    }

    fn write_indirect(&mut self, register: u8, value: u32) {
        self.write_selector(register);
        // SAFETY: selector serialization is enforced by `&mut self`; values
        // come from canonical redirection entries.
        unsafe {
            self.backend
                .write32(self.virtual_base + IO_APIC_WINDOW_OFFSET, value)
        };
    }

    fn write_selector(&mut self, register: u8) {
        // SAFETY: selector accepts one 8-bit I/O APIC register index.
        unsafe {
            self.backend
                .write32(self.virtual_base + IO_APIC_SELECTOR_OFFSET, register as u32)
        };
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct VolatileMmio;

// SAFETY: each method executes exactly one volatile operation at the caller's
// validated address. Controller construction owns mapping validity.
unsafe impl Mmio32 for VolatileMmio {
    unsafe fn read32(&self, address: u64) -> u32 {
        unsafe { core::ptr::read_volatile(address as *const u32) }
    }

    unsafe fn write32(&self, address: u64, value: u32) {
        unsafe { core::ptr::write_volatile(address as *mut u32, value) };
    }
}
