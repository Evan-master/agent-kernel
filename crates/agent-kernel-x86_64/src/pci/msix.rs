//! PCI MSI-X capability decoding and function-level control.
//!
//! The x86 architecture layer validates table descriptors and BAR-relative
//! bounds, then preserves unrelated configuration bits during mask and enable
//! transitions. Table MMIO lives in the sibling table adapter.

use super::{
    config_field,
    msix_error::{MsixDescriptor, MsixError},
    msix_table::{MsixTableRegion, MSIX_TABLE_ENTRY_BYTES},
    PciBarIndex, PciCapability, PciConfigAccess, PciConfigMutationAccess, PciFunctionAddress,
    PCI_CAPABILITY_ID_MSIX,
};

const TABLE_SIZE_MASK: u16 = 0x07ff;
const FUNCTION_MASK: u16 = 1 << 14;
const MSIX_ENABLE: u16 = 1 << 15;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct MsixCapability {
    offset: u8,
    table_size: u16,
    table_bar: PciBarIndex,
    table_offset: u32,
    pba_bar: PciBarIndex,
    pba_offset: u32,
}

impl MsixCapability {
    pub fn decode<A: PciConfigAccess>(
        access: &mut A,
        address: PciFunctionAddress,
        record: PciCapability,
    ) -> Result<Self, MsixError> {
        if record.id() != PCI_CAPABILITY_ID_MSIX {
            return Err(MsixError::WrongCapabilityId {
                actual: record.id(),
            });
        }
        if record.offset() > 0xf4 {
            return Err(MsixError::CapabilityOutOfRange {
                offset: record.offset(),
            });
        }
        let offset = u16::from(record.offset());
        let header = config_field::read_u32(access, address, offset);
        if header as u8 != PCI_CAPABILITY_ID_MSIX {
            return Err(MsixError::WrongCapabilityId {
                actual: header as u8,
            });
        }
        let control = (header >> 16) as u16;
        let table = config_field::read_u32(access, address, offset + 4);
        let pba = config_field::read_u32(access, address, offset + 8);
        let table_bir = table as u8 & 7;
        let pba_bir = pba as u8 & 7;
        let table_bar = PciBarIndex::new(table_bir).ok_or(MsixError::InvalidBarIndicator {
            field: MsixDescriptor::Table,
            bir: table_bir,
        })?;
        let pba_bar = PciBarIndex::new(pba_bir).ok_or(MsixError::InvalidBarIndicator {
            field: MsixDescriptor::PendingBitArray,
            bir: pba_bir,
        })?;
        Ok(Self {
            offset: record.offset(),
            table_size: (control & TABLE_SIZE_MASK) + 1,
            table_bar,
            table_offset: table & !7,
            pba_bar,
            pba_offset: pba & !7,
        })
    }

    pub const fn table_size(self) -> u16 {
        self.table_size
    }

    pub const fn table_bar(self) -> PciBarIndex {
        self.table_bar
    }

    pub const fn table_offset(self) -> u32 {
        self.table_offset
    }

    pub const fn pba_bar(self) -> PciBarIndex {
        self.pba_bar
    }

    pub const fn pba_offset(self) -> u32 {
        self.pba_offset
    }

    pub fn table_region(
        self,
        bar: PciBarIndex,
        bar_size: u64,
    ) -> Result<MsixTableRegion, MsixError> {
        if bar != self.table_bar {
            return Err(MsixError::TableBarMismatch {
                expected: self.table_bar,
                actual: bar,
            });
        }
        let byte_len = u32::from(self.table_size) * MSIX_TABLE_ENTRY_BYTES;
        let end = u64::from(self.table_offset) + u64::from(byte_len);
        if end > bar_size {
            return Err(MsixError::TableOutsideBar {
                offset: self.table_offset,
                byte_len,
                bar_size,
            });
        }
        Ok(MsixTableRegion::new(
            bar,
            self.table_offset,
            byte_len,
            self.table_size,
        ))
    }

    pub fn prepare<A: PciConfigMutationAccess>(
        self,
        access: &mut A,
        address: PciFunctionAddress,
    ) -> Result<(), MsixError> {
        self.update_control(access, address, false, true)
    }

    pub fn enable<A: PciConfigMutationAccess>(
        self,
        access: &mut A,
        address: PciFunctionAddress,
    ) -> Result<(), MsixError> {
        self.update_control(access, address, true, false)
    }

    pub fn disable<A: PciConfigMutationAccess>(
        self,
        access: &mut A,
        address: PciFunctionAddress,
    ) -> Result<(), MsixError> {
        self.update_control(access, address, false, true)
    }

    fn update_control<A: PciConfigMutationAccess>(
        self,
        access: &mut A,
        address: PciFunctionAddress,
        enabled: bool,
        masked: bool,
    ) -> Result<(), MsixError> {
        let offset = u16::from(self.offset) + 2;
        let original = config_field::read_u16(access, address, offset);
        let mut expected = original & !(MSIX_ENABLE | FUNCTION_MASK);
        if enabled {
            expected |= MSIX_ENABLE;
        }
        if masked {
            expected |= FUNCTION_MASK;
        }
        config_field::write_u16(access, address, offset, expected);
        let actual = config_field::read_u16(access, address, offset);
        if actual != expected {
            return Err(MsixError::FunctionVerificationFailed { expected, actual });
        }
        Ok(())
    }
}
