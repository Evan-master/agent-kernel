//! Errors from the native ATA PIO transport.

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum AtaPioError {
    DeviceNotIdentified,
    NoDevice,
    BusyTimeout,
    DataRequestTimeout,
    CommandCompletionTimeout,
    DeviceFault { status: u8, error: u8 },
    DeviceSignatureUnsupported { lba_mid: u8, lba_high: u8 },
    Lba48Unsupported,
    ZeroSectorCapacity,
    Lba48CapacityInvalid { sector_count: u64 },
    LogicalSectorSizeUnsupported { bytes: u32 },
    LbaOutOfRange { lba: u64, sector_count: u64 },
}
