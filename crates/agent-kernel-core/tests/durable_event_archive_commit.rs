#[allow(dead_code, unused_imports)]
mod event_archive_checkpoint_support;

use agent_kernel_core::{
    DurableArchiveReceipt, DurableArchiveVerificationError, DurableArchiveVerificationRequest,
    DurableArchiveVerifier, DurableSlot, DurableStateDigest, KernelError,
};

use event_archive_checkpoint_support::fixture;

#[derive(Default)]
struct RecordingVerifier {
    request: Option<DurableArchiveVerificationRequest>,
    reject: bool,
}

impl DurableArchiveVerifier for RecordingVerifier {
    fn verify(
        &mut self,
        request: DurableArchiveVerificationRequest,
    ) -> Result<(), DurableArchiveVerificationError> {
        self.request = Some(request);
        if self.reject {
            Err(DurableArchiveVerificationError::Rejected)
        } else {
            Ok(())
        }
    }
}

fn receipt(
    fixture: event_archive_checkpoint_support::Fixture,
    proposal: agent_kernel_core::EventArchiveProposal,
    seed: u8,
) -> DurableArchiveReceipt {
    DurableArchiveReceipt::new(
        DurableSlot::for_generation(proposal.generation()).unwrap(),
        fixture.root,
        proposal.generation(),
        proposal.digest(),
        DurableStateDigest::new([seed; 32]),
        DurableStateDigest::new([seed.wrapping_add(1); 32]),
        proposal.generation(),
        agent_kernel_core::DurableArchiveAnchor::unanchored(),
    )
    .unwrap()
}

#[test]
fn verified_receipt_releases_the_prefix_and_records_replay_state() {
    let (mut core, fixture) = fixture::<32>(agent_kernel_core::AgentEntryKind::Supervisor);
    let through = core.events().last().unwrap().sequence;
    let proposal = core.prepare_event_archive(through).unwrap();
    let receipt = receipt(fixture, proposal, 0x31);
    let mut verifier = RecordingVerifier::default();

    let checkpoint = core
        .commit_durable_event_archive(
            fixture.actor,
            fixture.authority,
            fixture.authority,
            proposal,
            receipt,
            &mut verifier,
        )
        .unwrap();

    assert_eq!(checkpoint.proposal(), proposal);
    assert_eq!(core.durable_archive_receipt(), Some(receipt));
    assert!(core.events().is_empty());
    let request = verifier.request.unwrap();
    assert_eq!(request.proposal(), proposal);
    assert_eq!(request.actor(), fixture.actor);
    assert_eq!(request.archive_authority(), fixture.authority);
    assert_eq!(request.storage_authority(), fixture.authority);
    assert_eq!(request.root(), fixture.root);
    assert_eq!(request.receipt(), receipt);
}

#[test]
fn rejected_and_mismatched_receipts_leave_all_core_state_unchanged() {
    let (mut core, fixture) = fixture::<32>(agent_kernel_core::AgentEntryKind::Supervisor);
    let through = core.events().last().unwrap().sequence;
    let proposal = core.prepare_event_archive(through).unwrap();
    let valid_receipt = receipt(fixture, proposal, 0x41);
    let events = core.events().to_vec();
    let mut rejected = RecordingVerifier {
        reject: true,
        ..RecordingVerifier::default()
    };

    assert_eq!(
        core.commit_durable_event_archive(
            fixture.actor,
            fixture.authority,
            fixture.authority,
            proposal,
            valid_receipt,
            &mut rejected,
        ),
        Err(KernelError::EventArchiveVerificationFailed)
    );
    assert_eq!(core.events(), events);
    assert_eq!(core.event_archive_checkpoint(), None);
    assert_eq!(core.durable_archive_receipt(), None);

    let foreign_generation = DurableArchiveReceipt::new(
        DurableSlot::B,
        fixture.root,
        2,
        proposal.digest(),
        DurableStateDigest::new([0x51; 32]),
        DurableStateDigest::new([0x52; 32]),
        2,
        agent_kernel_core::DurableArchiveAnchor::unanchored(),
    )
    .unwrap();
    let mut verifier = RecordingVerifier::default();
    assert_eq!(
        core.commit_durable_event_archive(
            fixture.actor,
            fixture.authority,
            fixture.authority,
            proposal,
            foreign_generation,
            &mut verifier,
        ),
        Err(KernelError::EventArchiveReceiptMismatch)
    );
    assert_eq!(verifier.request, None);
    assert_eq!(core.events(), events);
}

#[test]
fn a_consumed_receipt_is_rejected_before_verification_runs_again() {
    let (mut core, fixture) = fixture::<32>(agent_kernel_core::AgentEntryKind::Supervisor);
    let through = core.events().last().unwrap().sequence;
    let proposal = core.prepare_event_archive(through).unwrap();
    let receipt = receipt(fixture, proposal, 0x61);
    core.commit_durable_event_archive(
        fixture.actor,
        fixture.authority,
        fixture.authority,
        proposal,
        receipt,
        &mut RecordingVerifier::default(),
    )
    .unwrap();
    let mut replay_verifier = RecordingVerifier::default();

    assert_eq!(
        core.commit_durable_event_archive(
            fixture.actor,
            fixture.authority,
            fixture.authority,
            proposal,
            receipt,
            &mut replay_verifier,
        ),
        Err(KernelError::EventArchiveReceiptReplay)
    );
    assert_eq!(replay_verifier.request, None);
}
