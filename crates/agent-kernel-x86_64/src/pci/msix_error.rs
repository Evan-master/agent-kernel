//! Explicit MSI-X validation and hardware readback failures.
//!
//! Error variants retain fixed-width evidence suitable for boot diagnostics
//! without allocation or host-specific formatting.

use super::PciBarIndex;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum MsixDescriptor {
    Table,
    PendingBitArray,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum MsixTableField {
    MessageAddressLow,
    MessageAddressHigh,
    MessageData,
    VectorControl,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum MsixError {
    WrongCapabilityId {
        actual: u8,
    },
    CapabilityOutOfRange {
        offset: u8,
    },
    InvalidBarIndicator {
        field: MsixDescriptor,
        bir: u8,
    },
    TableBarMismatch {
        expected: PciBarIndex,
        actual: PciBarIndex,
    },
    TableOutsideBar {
        offset: u32,
        byte_len: u32,
        bar_size: u64,
    },
    EntryOutOfRange {
        entry: u16,
        table_size: u16,
    },
    TableVerificationFailed {
        entry: u16,
        field: MsixTableField,
        expected: u32,
        actual: u32,
    },
    FunctionVerificationFailed {
        expected: u16,
        actual: u16,
    },
    NullMappedBar,
    MappedAddressOverflow,
    MappedBarTooSmall {
        required: usize,
        actual: usize,
    },
    MappedTableUnaligned {
        address: usize,
    },
    TableAccessTooSmall {
        required: u32,
        actual: u32,
    },
}
