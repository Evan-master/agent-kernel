//! Explicit machine profile for durable storage during native boot.
//!
//! The profile keeps disk-free boot deterministic while allowing platform code
//! to select one fully validated ATA durable configuration. Device authority
//! remains architecture-owned.

use crate::ata::NativeAtaDurableConfig;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum NativeDurableStorageProfile {
    Disabled,
    Ata(NativeAtaDurableConfig),
}

impl NativeDurableStorageProfile {
    pub const fn ata(self) -> Option<NativeAtaDurableConfig> {
        match self {
            Self::Disabled => None,
            Self::Ata(config) => Some(config),
        }
    }

    pub const fn is_enabled(self) -> bool {
        matches!(self, Self::Ata(_))
    }
}
