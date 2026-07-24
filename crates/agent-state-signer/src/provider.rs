//! Injected signing capability owned outside the kernel and policy engine.

use agent_kernel_core::{
    DurableArchiveSignature, DurableSignatureAlgorithm, DurableStateSignerId,
    DURABLE_ARCHIVE_MANIFEST_BYTES,
};

pub trait StateSignerProvider {
    type Error;

    fn signature_algorithm(&self) -> DurableSignatureAlgorithm;

    fn signer_id(&self) -> DurableStateSignerId;

    fn sign_manifest(
        &mut self,
        manifest: &[u8; DURABLE_ARCHIVE_MANIFEST_BYTES],
    ) -> Result<DurableArchiveSignature, Self::Error>;
}
