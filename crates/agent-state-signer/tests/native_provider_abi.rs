use core::sync::atomic::{AtomicU64, Ordering};

use agent_kernel_core::{
    DurableStateSignerId, DURABLE_ARCHIVE_MANIFEST_BYTES, DURABLE_ARCHIVE_SIGNATURE_BYTES,
};
use agent_state_signer::{
    NativeStateSignerProvider, NativeStateSignerProviderError, StateSignerProvider,
};

static LAST_GENERATION: AtomicU64 = AtomicU64::new(0);

#[test]
fn native_provider_adapter_passes_fixed_buffers_and_generation() {
    let signer_id = DurableStateSignerId::new([0x53; 32]);
    // SAFETY: the test provider obeys the fixed buffer and lifetime contract.
    let mut provider = unsafe { NativeStateSignerProvider::new(signer_id, 17, fill_signature) }
        .expect("valid provider");
    let manifest = [0x29; DURABLE_ARCHIVE_MANIFEST_BYTES];

    let signature = provider.sign_manifest(&manifest).unwrap();

    assert_eq!(provider.signer_id(), signer_id);
    assert_eq!(signature.bytes(), [0xa7; DURABLE_ARCHIVE_SIGNATURE_BYTES]);
    assert_eq!(LAST_GENERATION.load(Ordering::Acquire), 17);
}

#[test]
fn native_provider_status_is_typed_and_returns_no_signature() {
    let signer_id = DurableStateSignerId::new([0x54; 32]);
    // SAFETY: the test provider returns a status without touching either buffer.
    let mut provider = unsafe { NativeStateSignerProvider::new(signer_id, 18, reject_signature) }
        .expect("valid provider");

    assert_eq!(
        provider.sign_manifest(&[0; DURABLE_ARCHIVE_MANIFEST_BYTES]),
        Err(NativeStateSignerProviderError::ProviderStatus(9))
    );
}

#[test]
fn native_provider_rejects_zero_signer_identity() {
    assert!(matches!(
        // SAFETY: the test provider obeys the ABI; construction fails on the
        // signer identity before it can be invoked.
        unsafe {
            NativeStateSignerProvider::new(DurableStateSignerId::new([0; 32]), 17, fill_signature)
        },
        Err(NativeStateSignerProviderError::ZeroSignerId)
    ));
    assert!(matches!(
        // SAFETY: the test provider obeys the ABI; construction fails on the
        // policy generation before it can be invoked.
        unsafe {
            NativeStateSignerProvider::new(DurableStateSignerId::new([0x55; 32]), 0, fill_signature)
        },
        Err(NativeStateSignerProviderError::ZeroPolicyGeneration)
    ));
}

unsafe extern "C" fn fill_signature(
    manifest: *const u8,
    signature: *mut u8,
    policy_generation: u64,
) -> u32 {
    assert!(!manifest.is_null());
    assert!(!signature.is_null());
    // SAFETY: the adapter supplies fixed live buffers for the duration of the
    // provider call.
    let manifest = unsafe { core::slice::from_raw_parts(manifest, DURABLE_ARCHIVE_MANIFEST_BYTES) };
    assert_eq!(manifest, [0x29; DURABLE_ARCHIVE_MANIFEST_BYTES]);
    // SAFETY: the output pointer names exactly one fixed signature buffer.
    let signature =
        unsafe { core::slice::from_raw_parts_mut(signature, DURABLE_ARCHIVE_SIGNATURE_BYTES) };
    signature.fill(0xa7);
    LAST_GENERATION.store(policy_generation, Ordering::Release);
    0
}

unsafe extern "C" fn reject_signature(
    _manifest: *const u8,
    _signature: *mut u8,
    _policy_generation: u64,
) -> u32 {
    9
}
