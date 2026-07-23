use agent_kernel::AgentKernel;
use agent_kernel_core::{
    AgentId, DurableArchiveAnchor, DurableArchiveManifest, DurableArchiveManifestFields,
    DurableArchiveReceipt, DurableArchiveRecoveryVerificationError,
    DurableArchiveRecoveryVerificationRequest, DurableArchiveRecoveryVerifier,
    DurableRecoveredHead, DurableSlot, DurableStateDigest, DurableStateSignerId,
    EventArchiveDigest, KernelError, Operation, OperationSet, ResourceKind,
};

type TestKernel = AgentKernel<1, 1, 2, 8, 0, 0, 0, 0, 0, 0>;

struct RecoveryVerifier {
    expected: DurableRecoveredHead,
    accepted: bool,
    calls: usize,
}

impl RecoveryVerifier {
    const fn accepting(expected: DurableRecoveredHead) -> Self {
        Self {
            expected,
            accepted: true,
            calls: 0,
        }
    }

    const fn rejecting(expected: DurableRecoveredHead) -> Self {
        Self {
            expected,
            accepted: false,
            calls: 0,
        }
    }
}

impl DurableArchiveRecoveryVerifier for RecoveryVerifier {
    fn verify(
        &mut self,
        request: DurableArchiveRecoveryVerificationRequest,
    ) -> Result<(), DurableArchiveRecoveryVerificationError> {
        self.calls += 1;
        if request.head() != self.expected {
            return Err(DurableArchiveRecoveryVerificationError::HeadMismatch);
        }
        if !self.accepted {
            return Err(DurableArchiveRecoveryVerificationError::Rejected);
        }
        self.accepted = false;
        Ok(())
    }
}

#[test]
fn verified_recovery_seeds_checkpoint_receipt_and_next_sequence() {
    let head = recovered_head();
    let through = head.through_sequence();
    let mut verifier = RecoveryVerifier::accepting(head);
    let mut kernel = TestKernel::new();

    let checkpoint = kernel
        .recover_verified_event_archive(head, &mut verifier)
        .expect("verified recovery");

    assert_eq!(verifier.calls, 1);
    assert_eq!(checkpoint.generation(), head.generation());
    assert_eq!(checkpoint.through_sequence(), through);
    assert_eq!(checkpoint.digest(), head.archive_digest());
    assert_eq!(kernel.event_archive_checkpoint(), Some(checkpoint));
    assert_eq!(kernel.durable_archive_receipt(), Some(head.receipt()));
    assert_eq!(kernel.next_event_sequence(), through + 1);
    assert!(kernel.events().is_empty());

    kernel.sys_register_agent(AgentId::new(1)).unwrap();
    assert_eq!(kernel.events()[0].sequence, through + 1);
}

#[test]
fn rejected_or_repeated_recovery_is_atomic() {
    let head = recovered_head();
    let mut rejected = RecoveryVerifier::rejecting(head);
    let mut kernel = TestKernel::new();

    assert_eq!(
        kernel.recover_verified_event_archive(head, &mut rejected),
        Err(KernelError::EventArchiveRecoveryVerificationFailed)
    );
    assert_eq!(rejected.calls, 1);
    assert!(kernel.events().is_empty());
    assert_eq!(kernel.event_archive_checkpoint(), None);
    assert_eq!(kernel.durable_archive_receipt(), None);
    assert_eq!(kernel.next_event_sequence(), 1);

    let mut accepted = RecoveryVerifier::accepting(head);
    kernel
        .recover_verified_event_archive(head, &mut accepted)
        .unwrap();
    assert_eq!(
        kernel.recover_verified_event_archive(head, &mut accepted),
        Err(KernelError::EventArchiveRecoveryStateNotVirgin)
    );
    assert_eq!(accepted.calls, 1);
}

#[test]
fn recovery_after_a_live_event_is_rejected_before_verification() {
    let head = recovered_head();
    let mut verifier = RecoveryVerifier::accepting(head);
    let mut kernel = TestKernel::new();
    kernel.sys_register_agent(AgentId::new(1)).unwrap();

    assert_eq!(
        kernel.recover_verified_event_archive(head, &mut verifier),
        Err(KernelError::EventArchiveRecoveryStateNotVirgin)
    );
    assert_eq!(verifier.calls, 0);
    assert_eq!(kernel.events().len(), 1);
    assert_eq!(kernel.next_event_sequence(), 2);
}

#[test]
fn exhausted_sequence_is_rejected_before_verification() {
    let head = exhausted_recovered_head();
    let mut verifier = RecoveryVerifier::accepting(head);
    let mut kernel = TestKernel::new();

    assert_eq!(
        kernel.recover_verified_event_archive(head, &mut verifier),
        Err(KernelError::EventArchiveRecoverySequenceExhausted)
    );
    assert_eq!(verifier.calls, 0);
    assert_eq!(kernel.event_archive_checkpoint(), None);
    assert_eq!(kernel.durable_archive_receipt(), None);
    assert_eq!(kernel.next_event_sequence(), 1);
}

fn recovered_head() -> DurableRecoveredHead {
    let mut source = TestKernel::new();
    let actor = AgentId::new(1);
    source.sys_register_agent(actor).unwrap();
    let root = source
        .sys_register_resource(ResourceKind::Workspace, None)
        .unwrap();
    let authority = source
        .sys_grant(
            actor,
            root,
            OperationSet::only(Operation::Rollback).with(Operation::Checkpoint),
        )
        .unwrap();
    let through = source.events().last().unwrap().sequence;
    let proposal = source.sys_prepare_event_archive(through).unwrap();
    let manifest = DurableArchiveManifest::new(
        proposal,
        actor,
        authority,
        root,
        root,
        128,
        DurableStateDigest::from_archive(proposal.digest()),
        DurableStateSignerId::new([0x61; 32]),
        1,
        DurableArchiveAnchor::unanchored(),
    )
    .unwrap();
    let receipt = DurableArchiveReceipt::new(
        DurableSlot::A,
        root,
        proposal.generation(),
        proposal.digest(),
        DurableStateDigest::new([0x62; 32]),
        DurableStateDigest::new([0x63; 32]),
        1,
        DurableArchiveAnchor::unanchored(),
    )
    .unwrap();
    DurableRecoveredHead::from_verified(manifest, receipt).unwrap()
}

fn exhausted_recovered_head() -> DurableRecoveredHead {
    let archive_digest = EventArchiveDigest::new([0x71; 32]);
    let manifest = DurableArchiveManifest::from_fields(DurableArchiveManifestFields {
        generation: 2,
        first_sequence: u64::MAX,
        through_sequence: u64::MAX,
        event_count: 1,
        previous_digest: EventArchiveDigest::new([0x70; 32]),
        archive_digest,
        actor: AgentId::new(1),
        archive_authority: agent_kernel_core::CapabilityId::new(1),
        root: agent_kernel_core::ResourceId::new(1),
        storage: agent_kernel_core::ResourceId::new(2),
        payload_length: 1,
        payload_digest: DurableStateDigest::from_archive(archive_digest),
        signer_id: DurableStateSignerId::new([0x72; 32]),
        signer_policy_generation: 1,
        anchor: DurableArchiveAnchor::unanchored(),
    })
    .unwrap();
    let receipt = DurableArchiveReceipt::new(
        DurableSlot::B,
        manifest.storage(),
        manifest.generation(),
        manifest.archive_digest(),
        DurableStateDigest::new([0x73; 32]),
        DurableStateDigest::new([0x74; 32]),
        1,
        manifest.anchor(),
    )
    .unwrap();
    DurableRecoveredHead::from_verified(manifest, receipt).unwrap()
}
