mod event_archive_checkpoint_support;

use agent_kernel_core::{
    AgentId, EventArchiveProposal, EventKind, KernelError, Operation, OperationSet, ResourceKind,
    VerificationRequirement,
};

use event_archive_checkpoint_support::{all_operations, complete_event, emit, fixture, TestCore};

#[test]
fn canonical_digest_is_deterministic_and_covers_every_event_field() {
    let event = complete_event();
    let proposal = EventArchiveProposal::from_segment(None, &[event]).unwrap();
    assert_eq!(
        proposal.digest().bytes,
        [
            14, 180, 227, 157, 45, 192, 110, 192, 205, 95, 84, 90, 43, 50, 82, 27, 210, 210, 46,
            169, 67, 54, 246, 13, 93, 138, 72, 254, 244, 28, 234, 119,
        ]
    );
    assert_eq!(
        proposal,
        EventArchiveProposal::from_segment(None, &[event]).unwrap()
    );

    macro_rules! changed {
        ($field:ident, $value:expr) => {{
            let mut changed = event;
            changed.$field = $value;
            changed
        }};
    }
    let variants = [
        changed!(agent, AgentId::new(99)),
        changed!(kind, EventKind::TaskCompleted),
        changed!(resource, None),
        changed!(capability, None),
        changed!(source_capability, None),
        changed!(intent, None),
        changed!(intent_kind, None),
        changed!(action, None),
        changed!(observation, None),
        changed!(message, None),
        changed!(message_kind, None),
        changed!(memory_cell, None),
        changed!(namespace_entry, None),
        changed!(namespace_key, None),
        changed!(namespace_object, None),
        changed!(operation, None),
        changed!(operations, OperationSet::empty()),
        changed!(verification, VerificationRequirement::Optional),
        changed!(checkpoint, None),
        changed!(task, None),
        changed!(runtime_admission, None),
        changed!(task_result, None),
        changed!(task_ticks, None),
        changed!(task_quantum, None),
        changed!(fault, None),
        changed!(fault_kind, None),
        changed!(fault_detail, None),
        changed!(fault_policy, None),
        changed!(fault_policy_action, None),
        changed!(waiter, None),
        changed!(waiter_kind, None),
        changed!(signal, None),
        changed!(target_agent, None),
        changed!(driver_binding, None),
        changed!(device_event, None),
        changed!(device_event_kind, None),
        changed!(device_event_payload, None),
        changed!(driver_command, None),
        changed!(driver_command_kind, None),
        changed!(driver_command_payload, None),
        changed!(driver_command_result, None),
        changed!(driver_invocation, None),
        changed!(driver_invocation_ticks, None),
        changed!(driver_invocation_quantum, None),
        changed!(agent_image, None),
        changed!(agent_image_kind, None),
        changed!(agent_image_digest, None),
        changed!(agent_image_abi_version, None),
        changed!(agent_image_entry_version, None),
        changed!(agent_image_signer, None),
    ];
    assert!(variants.iter().all(|variant| {
        EventArchiveProposal::from_segment(None, core::slice::from_ref(variant))
            .unwrap()
            .digest()
            != proposal.digest()
    }));
}

#[test]
fn commit_reclaims_dense_prefix_and_preserves_monotonic_sequence() {
    let (mut core, fixture) = fixture::<32>(agent_kernel_core::AgentEntryKind::Supervisor);
    emit(&mut core, fixture, 10);
    emit(&mut core, fixture, 11);
    let through = core.events()[3].sequence;
    let retained = core.events()[4..].to_vec();
    let proposal = core.prepare_event_archive(through).unwrap();

    let checkpoint = core
        .commit_event_archive(fixture.actor, fixture.authority, proposal)
        .unwrap();

    assert_eq!(checkpoint.proposal(), proposal);
    assert_eq!(checkpoint.actor(), fixture.actor);
    assert_eq!(checkpoint.authority(), fixture.authority);
    assert_eq!(checkpoint.root(), fixture.root);
    assert_eq!(core.event_archive_checkpoint(), Some(checkpoint));
    assert_eq!(core.events(), retained.as_slice());
    assert_eq!(core.events()[0].sequence, through + 1);
    let next = emit(&mut core, fixture, 12);
    assert_eq!(next.sequence, retained.last().unwrap().sequence + 1);
}

#[test]
fn chained_checkpoints_commit_the_previous_digest() {
    let (mut core, fixture) = fixture::<40>(agent_kernel_core::AgentEntryKind::Supervisor);
    emit(&mut core, fixture, 20);
    let first_through = core.events()[2].sequence;
    let first = core.prepare_event_archive(first_through).unwrap();
    let first_checkpoint = core
        .commit_event_archive(fixture.actor, fixture.authority, first)
        .unwrap();
    emit(&mut core, fixture, 21);
    emit(&mut core, fixture, 22);
    let second_through = core.events()[1].sequence;

    let second = core.prepare_event_archive(second_through).unwrap();

    assert_eq!(second.generation(), 2);
    assert_eq!(
        second.first_sequence(),
        first_checkpoint.through_sequence() + 1
    );
    assert_eq!(second.previous_digest(), first_checkpoint.digest());
    let second_checkpoint = core
        .commit_event_archive(fixture.actor, fixture.authority, second)
        .unwrap();
    assert_eq!(core.event_archive_checkpoint(), Some(second_checkpoint));
    assert_ne!(second_checkpoint.digest(), first_checkpoint.digest());
}

#[test]
fn full_log_can_commit_archive_and_accept_the_next_event() {
    let (mut core, fixture) = fixture::<16>(agent_kernel_core::AgentEntryKind::Supervisor);
    while core.events().len() < 16 {
        let detail = core.events().len() as u64 + 100;
        emit(&mut core, fixture, detail);
    }
    assert!(!core.has_event_capacity(1));
    let through = core.events()[5].sequence;
    let proposal = core.prepare_event_archive(through).unwrap();

    core.commit_event_archive(fixture.actor, fixture.authority, proposal)
        .unwrap();

    assert_eq!(core.events().len(), 10);
    assert_eq!(emit(&mut core, fixture, 200).sequence, 17);
}

#[test]
fn stale_foreign_and_unknown_proposals_fail_atomically() {
    let empty = TestCore::<8>::new();
    assert_eq!(
        empty.prepare_event_archive(1),
        Err(KernelError::EventArchiveSequenceNotFound)
    );

    let (mut first, first_fixture) = fixture::<32>(agent_kernel_core::AgentEntryKind::Supervisor);
    let (mut second, second_fixture) = fixture::<32>(agent_kernel_core::AgentEntryKind::Supervisor);
    emit(&mut first, first_fixture, 301);
    emit(&mut second, second_fixture, 302);
    let through = first.events().last().unwrap().sequence;
    let foreign = second.prepare_event_archive(through).unwrap();
    let before = first.events().to_vec();

    assert_eq!(
        first.commit_event_archive(first_fixture.actor, first_fixture.authority, foreign),
        Err(KernelError::EventArchiveProposalMismatch)
    );
    assert_eq!(first.events(), before.as_slice());
    assert_eq!(first.event_archive_checkpoint(), None);
    assert_eq!(
        first.prepare_event_archive(through + 99),
        Err(KernelError::EventArchiveSequenceNotFound)
    );
}

#[test]
fn commit_requires_supervisor_and_root_rollback_authority() {
    let (mut worker, worker_fixture) = fixture::<32>(agent_kernel_core::AgentEntryKind::Worker);
    let worker_proposal = worker
        .prepare_event_archive(worker.events()[1].sequence)
        .unwrap();
    assert_eq!(
        worker.commit_event_archive(
            worker_fixture.actor,
            worker_fixture.authority,
            worker_proposal,
        ),
        Err(KernelError::AgentEntryKindMismatch)
    );

    let (mut core, fixture) = fixture::<48>(agent_kernel_core::AgentEntryKind::Supervisor);
    let observe = core
        .derive_capability(
            fixture.actor,
            fixture.authority,
            fixture.actor,
            OperationSet::only(Operation::Observe),
        )
        .unwrap();
    let child = core
        .create_resource(
            fixture.actor,
            ResourceKind::Service,
            Some((fixture.root, fixture.authority)),
            all_operations(),
        )
        .unwrap();
    let proposal = core
        .prepare_event_archive(core.events()[2].sequence)
        .unwrap();
    let before = core.events().to_vec();

    assert_eq!(
        core.commit_event_archive(fixture.actor, observe, proposal),
        Err(KernelError::OperationDenied)
    );
    assert_eq!(
        core.commit_event_archive(fixture.actor, child.capability, proposal),
        Err(KernelError::EventArchiveAuthorityScopeMismatch)
    );
    assert_eq!(core.events(), before.as_slice());
}
