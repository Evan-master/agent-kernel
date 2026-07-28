//! Bounded MSI-X table MMIO and masked entry programming.
//!
//! This x86 architecture adapter binds a capability-validated BAR region,
//! prevents undersized mappings, and verifies each volatile table mutation
//! before an interrupt entry becomes unmasked.

use super::{
    msix_error::{MsixError, MsixTableField},
    MsixCapability, PciBarIndex, XapicMsiMessage,
};

pub(super) const MSIX_TABLE_ENTRY_BYTES: u32 = 16;
const VECTOR_MASK: u32 = 1;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct MsixTableRegion {
    bar: PciBarIndex,
    offset: u32,
    byte_len: u32,
    table_size: u16,
}

impl MsixTableRegion {
    pub const fn bar(self) -> PciBarIndex {
        self.bar
    }

    pub const fn offset(self) -> u32 {
        self.offset
    }

    pub const fn byte_len(self) -> u32 {
        self.byte_len
    }

    pub const fn table_size(self) -> u16 {
        self.table_size
    }

    pub(super) const fn new(bar: PciBarIndex, offset: u32, byte_len: u32, table_size: u16) -> Self {
        Self {
            bar,
            offset,
            byte_len,
            table_size,
        }
    }
}

pub trait MsixTableAccess {
    fn read_u32(&mut self, byte_offset: u32) -> u32;

    fn write_u32(&mut self, byte_offset: u32, value: u32);

    fn byte_len(&self) -> Option<u32> {
        None
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct VolatileMsixTable {
    base: *mut u32,
    byte_len: u32,
}

impl VolatileMsixTable {
    /// Binds one validated MSI-X table region within an existing BAR mapping.
    ///
    /// # Safety
    ///
    /// `mapped_bar` must remain exclusively writable and valid for
    /// `mapped_bytes` throughout this adapter's lifetime.
    pub unsafe fn bind(
        mapped_bar: *mut u8,
        mapped_bytes: usize,
        region: MsixTableRegion,
    ) -> Result<Self, MsixError> {
        if mapped_bar.is_null() {
            return Err(MsixError::NullMappedBar);
        }
        let required = usize::try_from(region.offset)
            .ok()
            .and_then(|offset| {
                usize::try_from(region.byte_len)
                    .ok()
                    .and_then(|byte_len| offset.checked_add(byte_len))
            })
            .ok_or(MsixError::MappedAddressOverflow)?;
        if required > mapped_bytes {
            return Err(MsixError::MappedBarTooSmall {
                required,
                actual: mapped_bytes,
            });
        }
        let table_address = (mapped_bar as usize)
            .checked_add(region.offset as usize)
            .ok_or(MsixError::MappedAddressOverflow)?;
        if table_address & 3 != 0 {
            return Err(MsixError::MappedTableUnaligned {
                address: table_address,
            });
        }
        Ok(Self {
            base: table_address as *mut u32,
            byte_len: region.byte_len,
        })
    }
}

impl MsixTableAccess for VolatileMsixTable {
    fn read_u32(&mut self, byte_offset: u32) -> u32 {
        assert!(
            byte_offset & 3 == 0
                && byte_offset
                    .checked_add(4)
                    .is_some_and(|end| end <= self.byte_len)
        );
        unsafe { core::ptr::read_volatile(self.base.add(byte_offset as usize / 4)) }
    }

    fn write_u32(&mut self, byte_offset: u32, value: u32) {
        assert!(
            byte_offset & 3 == 0
                && byte_offset
                    .checked_add(4)
                    .is_some_and(|end| end <= self.byte_len)
        );
        unsafe {
            core::ptr::write_volatile(self.base.add(byte_offset as usize / 4), value);
        }
    }

    fn byte_len(&self) -> Option<u32> {
        Some(self.byte_len)
    }
}

pub fn program_msix_table_entry<T: MsixTableAccess>(
    table: &mut T,
    capability: MsixCapability,
    entry: u16,
    message: XapicMsiMessage,
) -> Result<(), MsixError> {
    let required = u32::from(capability.table_size()) * MSIX_TABLE_ENTRY_BYTES;
    if let Some(actual) = table.byte_len() {
        if actual < required {
            return Err(MsixError::TableAccessTooSmall { required, actual });
        }
    }
    if entry >= capability.table_size() {
        return Err(MsixError::EntryOutOfRange {
            entry,
            table_size: capability.table_size(),
        });
    }
    let base = u32::from(entry) * MSIX_TABLE_ENTRY_BYTES;
    let control_offset = base + 12;
    let original_control = table.read_u32(control_offset);
    let masked_control = original_control | VECTOR_MASK;
    table.write_u32(control_offset, masked_control);
    verify_table_field(
        table,
        entry,
        control_offset,
        masked_control,
        MsixTableField::VectorControl,
    )?;

    write_and_verify(
        table,
        entry,
        base,
        message.address() as u32,
        MsixTableField::MessageAddressLow,
    )?;
    write_and_verify(
        table,
        entry,
        base + 4,
        (message.address() >> 32) as u32,
        MsixTableField::MessageAddressHigh,
    )?;
    write_and_verify(
        table,
        entry,
        base + 8,
        message.data(),
        MsixTableField::MessageData,
    )?;

    let unmasked_control = original_control & !VECTOR_MASK;
    table.write_u32(control_offset, unmasked_control);
    verify_table_field(
        table,
        entry,
        control_offset,
        unmasked_control,
        MsixTableField::VectorControl,
    )
}

fn write_and_verify<T: MsixTableAccess>(
    table: &mut T,
    entry: u16,
    offset: u32,
    value: u32,
    field: MsixTableField,
) -> Result<(), MsixError> {
    table.write_u32(offset, value);
    verify_table_field(table, entry, offset, value, field)
}

fn verify_table_field<T: MsixTableAccess>(
    table: &mut T,
    entry: u16,
    offset: u32,
    expected: u32,
    field: MsixTableField,
) -> Result<(), MsixError> {
    let actual = table.read_u32(offset);
    if actual != expected {
        return Err(MsixError::TableVerificationFailed {
            entry,
            field,
            expected,
            actual,
        });
    }
    Ok(())
}
