//! Fixed native ABI adapter for an externally provisioned signing provider.
//!
//! The provider entry owns key access. This adapter supplies bounded stack
//! buffers and converts its status code into the portable State Signer trait.

use agent_kernel_core::{
    DurableArchiveSignature, DurableSignatureAlgorithm, DurableStateSignerId,
    DURABLE_ARCHIVE_MANIFEST_BYTES, DURABLE_ARCHIVE_SIGNATURE_BYTES,
};

use crate::StateSignerProvider;

pub const NATIVE_STATE_SIGNER_PROVIDER_STATUS_OK: u32 = 0;

/// Native x86_64 providers use the platform C ABI, which is System V on the
/// freestanding target. Both pointers remain valid only for this invocation.
pub type NativeStateSignerProviderEntry =
    unsafe extern "C" fn(manifest: *const u8, signature: *mut u8, policy_generation: u64) -> u32;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum NativeStateSignerProviderError {
    ZeroSignerId,
    ZeroPolicyGeneration,
    ProviderStatus(u32),
}

#[derive(Copy, Clone)]
pub struct NativeStateSignerProvider {
    signer_id: DurableStateSignerId,
    signature_algorithm: DurableSignatureAlgorithm,
    policy_generation: u64,
    entry: NativeStateSignerProviderEntry,
}

impl NativeStateSignerProvider {
    /// Creates an adapter around one externally linked provider entry.
    ///
    /// # Safety
    ///
    /// `entry` must obey the native provider ABI: it may read exactly 285
    /// manifest bytes, write at most 64 signature bytes, retain neither
    /// pointer, preserve the platform's nonvolatile registers, and never
    /// unwind across the ABI boundary.
    pub unsafe fn new(
        signer_id: DurableStateSignerId,
        policy_generation: u64,
        entry: NativeStateSignerProviderEntry,
    ) -> Result<Self, NativeStateSignerProviderError> {
        // SAFETY: the caller supplies the same ABI-compatible entry contract.
        unsafe {
            Self::new_with_algorithm(
                signer_id,
                DurableSignatureAlgorithm::Ed25519,
                policy_generation,
                entry,
            )
        }
    }

    /// Creates an adapter with one explicit durable signature algorithm.
    ///
    /// # Safety
    ///
    /// `entry` must obey the same buffer, register, lifetime, and unwind
    /// constraints documented by [`Self::new`].
    pub unsafe fn new_with_algorithm(
        signer_id: DurableStateSignerId,
        signature_algorithm: DurableSignatureAlgorithm,
        policy_generation: u64,
        entry: NativeStateSignerProviderEntry,
    ) -> Result<Self, NativeStateSignerProviderError> {
        if signer_id.is_zero() {
            return Err(NativeStateSignerProviderError::ZeroSignerId);
        }
        if policy_generation == 0 {
            return Err(NativeStateSignerProviderError::ZeroPolicyGeneration);
        }
        Ok(Self {
            signer_id,
            signature_algorithm,
            policy_generation,
            entry,
        })
    }

    pub const fn policy_generation(&self) -> u64 {
        self.policy_generation
    }
}

impl StateSignerProvider for NativeStateSignerProvider {
    type Error = NativeStateSignerProviderError;

    fn signature_algorithm(&self) -> DurableSignatureAlgorithm {
        self.signature_algorithm
    }

    fn signer_id(&self) -> DurableStateSignerId {
        self.signer_id
    }

    fn sign_manifest(
        &mut self,
        manifest: &[u8; DURABLE_ARCHIVE_MANIFEST_BYTES],
    ) -> Result<DurableArchiveSignature, Self::Error> {
        let mut signature = [0; DURABLE_ARCHIVE_SIGNATURE_BYTES];
        // SAFETY: construction fixes one ABI-compatible provider entry. Both
        // pointers name exact live stack buffers for the duration of the call.
        let status = unsafe {
            (self.entry)(
                manifest.as_ptr(),
                signature.as_mut_ptr(),
                self.policy_generation,
            )
        };
        if status != NATIVE_STATE_SIGNER_PROVIDER_STATUS_OK {
            return Err(NativeStateSignerProviderError::ProviderStatus(status));
        }
        Ok(DurableArchiveSignature::new(signature))
    }
}
