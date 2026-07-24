//! Explicit machine profile for a provisioned native TPM State Signer.
//!
//! The x86 boot layer owns this public-only configuration. It selects the
//! CRB signer without storing key secrets or exposing a TPM command channel.

use crate::tpm2::ProvisionedTpmSignerConfig;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum NativeTpmSignerProfile {
    Disabled,
    Crb(ProvisionedTpmSignerConfig),
}

impl NativeTpmSignerProfile {
    pub const fn config(self) -> Option<ProvisionedTpmSignerConfig> {
        match self {
            Self::Disabled => None,
            Self::Crb(config) => Some(config),
        }
    }

    pub const fn is_enabled(self) -> bool {
        matches!(self, Self::Crb(_))
    }
}
