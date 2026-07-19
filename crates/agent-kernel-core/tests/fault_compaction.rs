mod fault_compaction_support;

use agent_kernel_core::{
    AgentEntryKind, AgentId, CapabilityId, EventKind, FaultId, FaultKind, KernelError, MessageKind,
    MessagePayload, MessageReceiveOutcome, MessageStatus, Operation, OperationSet, SignalKey,
    TaskStatus,
};

use fault_compaction_support::{dispatch, fault, recover, running_fixture, TestCore};

#[test]
fn recovered_fault_compacts_clears_task_reference_and_reuses_slot() {
    let (mut core, fixture) = running_fixture::<96, 0, 1>(AgentEntryKind::Supervisor, false);
    let first = fault(&mut core, fixture, 11);
    recover(&mut core, fixture);
    let event_start = core.events().len();

    let receipt = core
        .compact_fault_prefix(fixture.actor, fixture.root_authority, first)
        .expect("recovered fault compacts");

    assert_eq!(receipt.first(), first);
    assert_eq!(receipt.through(), first);
    assert_eq!(receipt.count(), 1);
    assert!(core.faults().is_empty());
    assert_eq!(core.tasks()[0].last_fault, None);
    let event = core.events()[event_start];
    assert_eq!(event.kind, EventKind::FaultCompacted);
    assert_eq!(event.agent, fixture.actor);
    assert_eq!(event.target_agent, Some(fixture.actor));
    assert_eq!(event.resource, Some(fixture.resource));
    assert_eq!(event.capability, Some(fixture.root_authority));
    assert_eq!(event.operation, Some(Operation::Rollback));
    assert_eq!(event.task, Some(fixture.task));
    assert_eq!(event.fault, Some(first));
    assert_eq!(event.fault_kind, Some(FaultKind::ExecutionTrap));
    assert_eq!(event.fault_detail, Some(11));

    dispatch(&mut core, fixture.actor, fixture.task);
    assert_eq!(fault(&mut core, fixture, 12), FaultId::new(2));
    assert_eq!(core.faults()[0].id, FaultId::new(2));
}

#[test]
fn prefix_compaction_preserves_suffix_and_latest_fault_reference() {
    let (mut core, fixture) = running_fixture::<112, 0, 2>(AgentEntryKind::Supervisor, false);
    let first = fault(&mut core, fixture, 21);
    recover(&mut core, fixture);
    dispatch(&mut core, fixture.actor, fixture.task);
    let second = fault(&mut core, fixture, 22);
    recover(&mut core, fixture);
    let retained = core.faults()[1];

    let receipt = core
        .compact_fault_prefix(fixture.actor, fixture.root_authority, first)
        .expect("oldest recovered fault compacts");

    assert_eq!(receipt.count(), 1);
    assert_eq!(core.faults(), [retained]);
    assert_eq!(core.tasks()[0].last_fault, Some(second));
    dispatch(&mut core, fixture.actor, fixture.task);
    assert_eq!(fault(&mut core, fixture, 23), FaultId::new(3));
    assert_eq!(core.faults()[0].id, second);
    assert_eq!(core.faults()[1].id, FaultId::new(3));
}

#[test]
fn active_fault_and_unknown_id_fail_without_mutation() {
    let (mut core, fixture) = running_fixture::<72, 0, 1>(AgentEntryKind::Supervisor, false);
    let fault = fault(&mut core, fixture, 31);
    let faults = core.faults().to_vec();
    let events = core.events().len();

    assert_eq!(
        core.compact_fault_prefix(fixture.actor, fixture.root_authority, FaultId::new(99)),
        Err(KernelError::FaultNotFound)
    );
    assert_eq!(
        core.compact_fault_prefix(fixture.actor, fixture.root_authority, fault),
        Err(KernelError::FaultCompactionNotReady)
    );
    assert_eq!(core.faults(), faults.as_slice());
    assert_eq!(core.tasks()[0].status, TaskStatus::Faulted);
    assert_eq!(core.tasks()[0].last_fault, Some(fault));
    assert_eq!(core.events().len(), events);
}

#[test]
fn live_message_blocks_fault_compaction_atomically() {
    let (mut core, fixture) = running_fixture::<96, 1, 1>(AgentEntryKind::Supervisor, false);
    let fault = fault(&mut core, fixture, 41);
    recover(&mut core, fixture);
    let message = core
        .send_message(
            fixture.actor,
            fixture.actor,
            MessageKind::Fault,
            MessagePayload {
                fault: Some(fault),
                ..MessagePayload::empty()
            },
        )
        .expect("fault message sends");
    let events = core.events().len();

    assert_eq!(
        core.compact_fault_prefix(fixture.actor, fixture.root_authority, fault),
        Err(KernelError::FaultCompactionReferenced)
    );
    assert_eq!(core.faults().len(), 1);
    assert_eq!(core.tasks()[0].last_fault, Some(fault));
    assert_eq!(core.events().len(), events);

    dispatch(&mut core, fixture.actor, fixture.task);
    assert_eq!(
        core.receive_or_wait_message(fixture.actor, fixture.task_authority, fixture.task),
        Ok(MessageReceiveOutcome::Received(message))
    );
    core.acknowledge_message(fixture.actor, message)
        .expect("message acknowledges");
    assert_eq!(core.messages()[0].status, MessageStatus::Acknowledged);
    core.compact_fault_prefix(fixture.actor, fixture.root_authority, fault)
        .expect("acknowledged message keeps only historical reference");
    assert!(core.faults().is_empty());
    assert_eq!(core.tasks()[0].last_fault, None);
}

#[test]
fn supervisor_identity_and_cleanup_authority_are_required() {
    let actor = AgentId::new(1);
    let mut unlaunched = TestCore::<8, 0, 1>::new();
    unlaunched.register_agent(actor).unwrap();
    assert_eq!(
        unlaunched.compact_fault_prefix(actor, CapabilityId::new(1), FaultId::new(1)),
        Err(KernelError::AgentNotLaunched)
    );

    let (mut worker, fixture) = running_fixture::<72, 0, 1>(AgentEntryKind::Worker, false);
    let target = fault(&mut worker, fixture, 51);
    recover(&mut worker, fixture);
    assert_eq!(
        worker.compact_fault_prefix(fixture.actor, fixture.root_authority, target),
        Err(KernelError::AgentEntryKindMismatch)
    );

    let (mut core, fixture) = running_fixture::<80, 0, 1>(AgentEntryKind::Supervisor, false);
    let target = fault(&mut core, fixture, 52);
    recover(&mut core, fixture);
    let observe_only = core
        .derive_capability(
            fixture.actor,
            fixture.root_authority,
            fixture.actor,
            OperationSet::only(Operation::Observe),
        )
        .unwrap();
    assert_eq!(
        core.compact_fault_prefix(fixture.actor, observe_only, target),
        Err(KernelError::OperationDenied)
    );
    assert_eq!(core.tasks()[0].last_fault, Some(target));
}

#[test]
fn retired_child_fault_accepts_active_ancestor_cleanup_authority() {
    let (mut core, fixture) = running_fixture::<96, 0, 1>(AgentEntryKind::Supervisor, true);
    let target = fault(&mut core, fixture, 61);
    recover(&mut core, fixture);

    assert_eq!(
        core.compact_fault_prefix(fixture.actor, fixture.root_authority, target),
        Err(KernelError::ResourceMismatch)
    );
    core.retire_resource(fixture.actor, fixture.authority, fixture.resource)
        .expect("child resource retires");
    core.compact_fault_prefix(fixture.actor, fixture.root_authority, target)
        .expect("ancestor authority cleans retired child Fault");
    assert!(core.faults().is_empty());
    assert_eq!(fixture.root.raw(), 1);
}

#[test]
fn full_event_log_rejects_fault_compaction_atomically() {
    let (mut core, fixture) = running_fixture::<32, 0, 1>(AgentEntryKind::Supervisor, false);
    let target = fault(&mut core, fixture, 71);
    recover(&mut core, fixture);
    while core.events().len() < 32 {
        core.emit_signal(
            fixture.actor,
            fixture.root_authority,
            fixture.root,
            SignalKey::new(999),
        )
        .expect("filler Event fits");
    }
    let faults = core.faults().to_vec();

    assert_eq!(
        core.compact_fault_prefix(fixture.actor, fixture.root_authority, target),
        Err(KernelError::EventLogFull)
    );
    assert_eq!(core.faults(), faults.as_slice());
    assert_eq!(core.tasks()[0].last_fault, Some(target));
    assert_eq!(core.events().len(), 32);
}
