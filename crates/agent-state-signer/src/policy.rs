//! Fixed durable-state authority accepted by one State Signer instance.

use agent_kernel_core::{DurableSignatureAlgorithm, DurableStateSignerId, ResourceId};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum StateSignerPolicyError {
    ZeroRoot,
    ZeroStorage,
    ZeroSignerId,
    ZeroGeneration,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct StateSignerPolicy {
    root: ResourceId,
    storage: ResourceId,
    signer_id: DurableStateSignerId,
    signature_algorithm: DurableSignatureAlgorithm,
    generation: u64,
}

impl StateSignerPolicy {
    pub fn new(
        root: ResourceId,
        storage: ResourceId,
        signer_id: DurableStateSignerId,
        generation: u64,
    ) -> Result<Self, StateSignerPolicyError> {
        Self::new_with_algorithm(
            root,
            storage,
            signer_id,
            DurableSignatureAlgorithm::Ed25519,
            generation,
        )
    }

    pub fn new_with_algorithm(
        root: ResourceId,
        storage: ResourceId,
        signer_id: DurableStateSignerId,
        signature_algorithm: DurableSignatureAlgorithm,
        generation: u64,
    ) -> Result<Self, StateSignerPolicyError> {
        if root.raw() == 0 {
            return Err(StateSignerPolicyError::ZeroRoot);
        }
        if storage.raw() == 0 {
            return Err(StateSignerPolicyError::ZeroStorage);
        }
        if signer_id.is_zero() {
            return Err(StateSignerPolicyError::ZeroSignerId);
        }
        if generation == 0 {
            return Err(StateSignerPolicyError::ZeroGeneration);
        }
        Ok(Self {
            root,
            storage,
            signer_id,
            signature_algorithm,
            generation,
        })
    }

    pub const fn root(self) -> ResourceId {
        self.root
    }

    pub const fn storage(self) -> ResourceId {
        self.storage
    }

    pub const fn signer_id(self) -> DurableStateSignerId {
        self.signer_id
    }

    pub const fn signature_algorithm(self) -> DurableSignatureAlgorithm {
        self.signature_algorithm
    }

    pub const fn generation(self) -> u64 {
        self.generation
    }
}
