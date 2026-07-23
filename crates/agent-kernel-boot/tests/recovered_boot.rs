use agent_kernel::AgentKernel;
use agent_kernel_boot::{BootConfig, BootedKernel};
use agent_kernel_core::{
    AgentId, DurableArchiveAnchor, DurableArchiveManifest, DurableArchiveReceipt,
    DurableArchiveRecoveryVerificationError, DurableArchiveRecoveryVerificationRequest,
    DurableArchiveRecoveryVerifier, DurableRecoveredHead, DurableSlot, DurableStateDigest,
    DurableStateSignerId, EventKind, Operation, OperationSet, ResourceKind,
};

type SourceKernel = AgentKernel<1, 1, 2, 8, 0, 0, 0, 0, 0, 0>;
type RecoveredBoot = BootedKernel<2, 2, 4, 16, 4, 4, 0, 0, 0, 0>;

struct ExactRecovery {
    head: DurableRecoveredHead,
    consumed: bool,
}

impl DurableArchiveRecoveryVerifier for ExactRecovery {
    fn verify(
        &mut self,
        request: DurableArchiveRecoveryVerificationRequest,
    ) -> Result<(), DurableArchiveRecoveryVerificationError> {
        if self.consumed {
            return Err(DurableArchiveRecoveryVerificationError::AlreadyConsumed);
        }
        if request.head() != self.head {
            return Err(DurableArchiveRecoveryVerificationError::HeadMismatch);
        }
        self.consumed = true;
        Ok(())
    }
}

#[test]
fn recovered_boot_continues_the_persisted_event_sequence() {
    let head = recovered_head();
    let mut verifier = ExactRecovery {
        head,
        consumed: false,
    };

    let booted = RecoveredBoot::boot_recovered(BootConfig::default(), head, &mut verifier)
        .expect("recovered boot");

    assert!(verifier.consumed);
    assert_eq!(
        booted.kernel().event_archive_checkpoint().unwrap().digest(),
        head.archive_digest()
    );
    assert_eq!(
        booted.kernel().durable_archive_receipt(),
        Some(head.receipt())
    );
    assert_eq!(booted.kernel().events().len(), 8);
    assert_eq!(
        booted.kernel().events()[0].sequence,
        head.through_sequence() + 1
    );
    assert_eq!(booted.kernel().events()[0].kind, EventKind::AgentRegistered);
    assert_eq!(
        booted.kernel().next_event_sequence(),
        head.through_sequence() + 9
    );
}

fn recovered_head() -> DurableRecoveredHead {
    let mut source = SourceKernel::new();
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
    let proposal = source
        .sys_prepare_event_archive(source.events().last().unwrap().sequence)
        .unwrap();
    let manifest = DurableArchiveManifest::new(
        proposal,
        actor,
        authority,
        root,
        root,
        128,
        DurableStateDigest::from_archive(proposal.digest()),
        DurableStateSignerId::new([0x71; 32]),
        1,
        DurableArchiveAnchor::unanchored(),
    )
    .unwrap();
    let receipt = DurableArchiveReceipt::new(
        DurableSlot::A,
        root,
        proposal.generation(),
        proposal.digest(),
        DurableStateDigest::new([0x72; 32]),
        DurableStateDigest::new([0x73; 32]),
        1,
        DurableArchiveAnchor::unanchored(),
    )
    .unwrap();
    DurableRecoveredHead::from_verified(manifest, receipt).unwrap()
}
