use agent_kernel_core::{
    AgentId, CapabilityId, DurableArchiveAnchor, DurableArchiveManifest, DurableArchiveSignature,
    DurableSignatureAlgorithm, DurableStateDigest, DurableStateSignerId, EventArchiveProposal,
    KernelCore, ResourceId, DURABLE_ARCHIVE_MANIFEST_BYTES,
};
use agent_kernel_x86_64::{
    durable_archive_request::{
        encode_unsigned_durable_archive_request, DurableArchiveRequest,
        DURABLE_ARCHIVE_REQUEST_SIGNATURE_OFFSET,
    },
    tpm2::{
        sign_retained_durable_request, KernelStateSigner, KernelStateSignerError,
        KernelStateSignerServiceError,
    },
};

const GENERATION: u64 = 7;
const POLICY_GENERATION: u64 = 23;
const SIGNER_ID: DurableStateSignerId = DurableStateSignerId::new([0x51; 32]);

struct FakeSigner {
    calls: u8,
}

impl KernelStateSigner for FakeSigner {
    fn signature_algorithm(&self) -> DurableSignatureAlgorithm {
        DurableSignatureAlgorithm::EcdsaP256Sha256
    }

    fn signer_id(&self) -> DurableStateSignerId {
        SIGNER_ID
    }

    fn policy_generation(&self) -> u64 {
        POLICY_GENERATION
    }

    fn sign_manifest(
        &mut self,
        manifest: &[u8; DURABLE_ARCHIVE_MANIFEST_BYTES],
    ) -> Result<DurableArchiveSignature, KernelStateSignerError> {
        assert!(manifest.iter().any(|byte| *byte != 0));
        self.calls += 1;
        Ok(DurableArchiveSignature::new([0x3c; 64]))
    }
}

#[test]
fn service_signs_only_the_retained_unsigned_p256_request() {
    let manifest = manifest();
    let retained =
        encode_unsigned_durable_archive_request(GENERATION, CapabilityId::new(8), manifest)
            .unwrap();
    let mut signer = FakeSigner { calls: 0 };

    let signed =
        sign_retained_durable_request(&retained, &retained, manifest, GENERATION, &mut signer)
            .unwrap();

    assert_eq!(signer.calls, 1);
    assert_eq!(
        &signed[DURABLE_ARCHIVE_REQUEST_SIGNATURE_OFFSET
            ..DURABLE_ARCHIVE_REQUEST_SIGNATURE_OFFSET + 64],
        &[0x3c; 64]
    );
    let decoded = DurableArchiveRequest::decode(&signed, GENERATION).unwrap();
    assert_eq!(decoded.manifest(), manifest);
    assert_eq!(decoded.signature().bytes(), [0x3c; 64]);
}

#[test]
fn service_rejects_page_changes_before_invoking_hardware() {
    let manifest = manifest();
    let retained =
        encode_unsigned_durable_archive_request(GENERATION, CapabilityId::new(8), manifest)
            .unwrap();
    let mut changed = retained;
    changed[40] ^= 1;
    let mut signer = FakeSigner { calls: 0 };

    assert_eq!(
        sign_retained_durable_request(&retained, &changed, manifest, GENERATION, &mut signer),
        Err(KernelStateSignerServiceError::RequestChanged)
    );
    assert_eq!(signer.calls, 0);
}

fn manifest() -> DurableArchiveManifest {
    type Core = KernelCore<1, 0, 0, 2, 0, 0, 0, 0, 0, 0>;
    let mut core = Core::new();
    let actor = AgentId::new(1);
    core.register_agent(actor).unwrap();
    let proposal = EventArchiveProposal::from_segment(None, core.events()).unwrap();
    DurableArchiveManifest::new_algorithm_bound(
        proposal,
        actor,
        CapabilityId::new(3),
        ResourceId::new(4),
        ResourceId::new(5),
        128,
        DurableStateDigest::from_archive(proposal.digest()),
        SIGNER_ID,
        DurableSignatureAlgorithm::EcdsaP256Sha256,
        POLICY_GENERATION,
        DurableArchiveAnchor::unanchored(),
    )
    .unwrap()
}
