//! Static machine policy for one native ATA durable archive service.

use agent_kernel_core::{
    durable_state_signer_id, DurableStateSignerRecord, DurableStateSignerStatus, ResourceId,
};

use crate::ata::{AtaPioConfig, ATA_DURABLE_SLOT_SECTORS};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum NativeAtaDurableConfigError {
    ZeroRoot,
    ZeroStorage,
    AliasedRootAndStorage,
    BaseLbaUnaligned,
    ZeroPolicyGeneration,
    SignerInactive,
    SignerIdentityMismatch,
    SignerRootMismatch,
    SignerGenerationMismatch,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct NativeAtaDurableConfig {
    pio: AtaPioConfig,
    root: ResourceId,
    storage: ResourceId,
    base_lba: u64,
    signer: DurableStateSignerRecord,
    policy_generation: u64,
}

impl NativeAtaDurableConfig {
    pub fn new(
        pio: AtaPioConfig,
        root: ResourceId,
        storage: ResourceId,
        base_lba: u64,
        signer: DurableStateSignerRecord,
        policy_generation: u64,
    ) -> Result<Self, NativeAtaDurableConfigError> {
        if root.raw() == 0 {
            return Err(NativeAtaDurableConfigError::ZeroRoot);
        }
        if storage.raw() == 0 {
            return Err(NativeAtaDurableConfigError::ZeroStorage);
        }
        if root == storage {
            return Err(NativeAtaDurableConfigError::AliasedRootAndStorage);
        }
        if !base_lba.is_multiple_of(ATA_DURABLE_SLOT_SECTORS) {
            return Err(NativeAtaDurableConfigError::BaseLbaUnaligned);
        }
        if policy_generation == 0 {
            return Err(NativeAtaDurableConfigError::ZeroPolicyGeneration);
        }
        if signer.status != DurableStateSignerStatus::Active {
            return Err(NativeAtaDurableConfigError::SignerInactive);
        }
        if durable_state_signer_id(signer.public_key) != signer.signer_id {
            return Err(NativeAtaDurableConfigError::SignerIdentityMismatch);
        }
        if signer.root != root {
            return Err(NativeAtaDurableConfigError::SignerRootMismatch);
        }
        if signer.generation != policy_generation {
            return Err(NativeAtaDurableConfigError::SignerGenerationMismatch);
        }
        Ok(Self {
            pio,
            root,
            storage,
            base_lba,
            signer,
            policy_generation,
        })
    }

    pub const fn pio(self) -> AtaPioConfig {
        self.pio
    }

    pub const fn root(self) -> ResourceId {
        self.root
    }

    pub const fn storage(self) -> ResourceId {
        self.storage
    }

    pub const fn base_lba(self) -> u64 {
        self.base_lba
    }

    pub const fn signer(self) -> DurableStateSignerRecord {
        self.signer
    }

    pub const fn policy_generation(self) -> u64 {
        self.policy_generation
    }
}
