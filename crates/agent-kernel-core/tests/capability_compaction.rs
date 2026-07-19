mod capability_compaction_support;

use agent_kernel_core::{
    ActionId, AgentEntryKind, CapabilityId, CheckpointId, EventKind, KernelError, MessageKind,
    MessagePayload, NamespaceKey, NamespaceObject, Operation, OperationSet, ResourceKind,
    RuntimeAdmissionFailure,
};

use capability_compaction_support::{
    all_operations, cancelled_delegated_task, prepared, register_image,
};

#[test]
fn revoked_leaf_compaction_reuses_a_sparse_slot_with_monotonic_ids() {
    let (mut core, fixture) = prepared::<64>();
    let target = core
        .derive_capability(
            fixture.supervisor,
            fixture.authority,
            fixture.worker,
            OperationSet::only(Operation::Rollback),
        )
        .expect("target derives");
    let target_record = core.capability(target).expect("target exists");
    core.revoke_derived_capability(fixture.supervisor, fixture.authority, target)
        .expect("target revokes");
    let event_start = core.events().len();

    let receipt = core
        .compact_capability(fixture.supervisor, fixture.authority, target)
        .expect("revoked leaf compacts");

    assert_eq!(receipt.capability(), target);
    assert_eq!(core.capability_capacity(), 10);
    assert_eq!(core.capability_count(), 1);
    assert_eq!(
        core.capability(target),
        Err(KernelError::CapabilityNotFound)
    );
    let event = core.events().get(event_start).expect("event exists");
    assert_eq!(event.kind, EventKind::CapabilityCompacted);
    assert_eq!(event.agent, fixture.supervisor);
    assert_eq!(event.capability, Some(target));
    assert_eq!(event.source_capability, Some(fixture.authority));
    assert_eq!(event.operation, Some(Operation::Rollback));
    assert_eq!(event.resource, Some(target_record.resource));
    assert_eq!(event.operations, target_record.operations);
    assert_eq!(event.task, target_record.task);
    assert_eq!(event.target_agent, Some(target_record.agent));

    let replacement = core
        .derive_capability(
            fixture.supervisor,
            fixture.authority,
            fixture.worker,
            OperationSet::only(Operation::Observe),
        )
        .expect("vacated slot reuses");
    assert_eq!(replacement, CapabilityId::new(3));
    assert_eq!(core.capability_count(), 2);
}

#[test]
fn retained_children_and_tasks_require_leaf_first_retirement() {
    let (mut core, fixture) = prepared::<96>();
    let parent = core
        .derive_capability(
            fixture.supervisor,
            fixture.authority,
            fixture.worker,
            OperationSet::only(Operation::Observe).with(Operation::Delegate),
        )
        .expect("parent derives");
    let child = core
        .derive_capability(
            fixture.worker,
            parent,
            fixture.other,
            OperationSet::only(Operation::Observe),
        )
        .expect("child derives");
    core.revoke_capability(parent).expect("parent revokes");
    assert_eq!(
        core.compact_capability(fixture.supervisor, fixture.authority, parent),
        Err(KernelError::CapabilityCompactionReferenced)
    );
    core.revoke_capability(child).expect("child revokes");
    core.compact_capability(fixture.supervisor, fixture.authority, child)
        .expect("child compacts first");
    core.compact_capability(fixture.supervisor, fixture.authority, parent)
        .expect("parent compacts second");

    let (task, task_capability) = cancelled_delegated_task(&mut core, fixture);
    core.revoke_capability(task_capability)
        .expect("task capability revokes");
    assert_eq!(
        core.compact_capability(fixture.supervisor, fixture.authority, task_capability),
        Err(KernelError::CapabilityCompactionReferenced)
    );
    core.compact_task_prefix(fixture.supervisor, fixture.authority, task)
        .expect("task compacts first");
    core.compact_capability(fixture.supervisor, fixture.authority, task_capability)
        .expect("task capability compacts after task");
}

#[test]
fn agent_entries_and_runtime_admissions_hold_live_capability_references() {
    let (mut core, fixture) = prepared::<128>();
    let entry_capability = core
        .derive_capability(
            fixture.supervisor,
            fixture.authority,
            fixture.worker,
            OperationSet::only(Operation::Act),
        )
        .expect("entry capability derives");
    let image = register_image(
        &mut core,
        fixture.supervisor,
        fixture.authority,
        fixture.resource,
        agent_kernel_core::AgentImageKind::Worker,
        2,
    );
    core.launch_agent(
        fixture.worker,
        entry_capability,
        fixture.resource,
        image,
        AgentEntryKind::Worker,
        None,
    )
    .expect("worker launches");
    core.revoke_derived_capability(fixture.supervisor, fixture.authority, entry_capability)
        .expect("entry capability revokes");
    assert_eq!(
        core.compact_capability(fixture.supervisor, fixture.authority, entry_capability),
        Err(KernelError::CapabilityCompactionReferenced)
    );

    let admission_authority = core
        .derive_capability(
            fixture.supervisor,
            fixture.authority,
            fixture.supervisor,
            OperationSet::only(Operation::Delegate).with(Operation::Rollback),
        )
        .expect("admission authority derives");
    let intent = core
        .declare_intent(
            fixture.supervisor,
            fixture.authority,
            fixture.resource,
            agent_kernel_core::IntentKind::Act,
            agent_kernel_core::VerificationRequirement::Required,
        )
        .expect("intent declares");
    let task = core
        .create_task(fixture.supervisor, fixture.authority, intent)
        .expect("task creates");
    core.delegate_task(fixture.supervisor, fixture.authority, task, fixture.other)
        .expect("task delegates");
    let task_capability = core.task(task).unwrap().delegated_capability.unwrap();
    let target_image = register_image(
        &mut core,
        fixture.supervisor,
        fixture.authority,
        fixture.resource,
        agent_kernel_core::AgentImageKind::Worker,
        3,
    );
    core.launch_task_agent(
        fixture.other,
        task_capability,
        task,
        target_image,
        AgentEntryKind::Worker,
    )
    .expect("target launches");
    core.accept_task(fixture.other, task)
        .expect("target accepts task");
    let admission = core
        .request_runtime_admission(fixture.supervisor, admission_authority, fixture.other, task)
        .expect("admission requests");
    let permit = core
        .prepare_next_runtime_admission()
        .expect("admission prepares");
    core.revoke_derived_capability(fixture.supervisor, fixture.authority, admission_authority)
        .expect("admission authority revokes");
    assert_eq!(
        core.compact_capability(fixture.supervisor, fixture.authority, admission_authority,),
        Err(KernelError::CapabilityCompactionReferenced)
    );
    core.reject_runtime_admission(permit, RuntimeAdmissionFailure::AllocationUnavailable)
        .expect("admission rejects");
    core.compact_runtime_admission_prefix(fixture.supervisor, fixture.authority, admission)
        .expect("admission compacts first");
    core.compact_capability(fixture.supervisor, fixture.authority, admission_authority)
        .expect("admission authority compacts");
}

#[test]
fn unacknowledged_messages_block_while_historical_records_remain_valid() {
    let (mut core, fixture) = prepared::<96>();
    let target = core
        .derive_capability(
            fixture.supervisor,
            fixture.authority,
            fixture.supervisor,
            all_operations(),
        )
        .expect("target derives");
    core.observe(fixture.supervisor, target, fixture.resource)
        .expect("observation records");
    core.act(
        fixture.supervisor,
        target,
        ActionId::new(1),
        fixture.resource,
    )
    .expect("action records");
    core.checkpoint(
        fixture.supervisor,
        target,
        CheckpointId::new(1),
        fixture.resource,
    )
    .expect("checkpoint records");
    core.bind_namespace_entry(
        fixture.supervisor,
        target,
        fixture.resource,
        NamespaceKey::new(1),
        NamespaceObject::Resource(fixture.resource),
    )
    .expect("namespace entry binds");
    core.revoke_derived_capability(fixture.supervisor, fixture.authority, target)
        .expect("target revokes");
    let message = core
        .send_message(
            fixture.supervisor,
            fixture.worker,
            MessageKind::Notify,
            MessagePayload {
                capability: Some(target),
                ..MessagePayload::empty()
            },
        )
        .expect("message sends");
    assert_eq!(
        core.compact_capability(fixture.supervisor, fixture.authority, target),
        Err(KernelError::CapabilityCompactionReferenced)
    );
    core.receive_message(fixture.worker)
        .expect("message receives");
    assert_eq!(
        core.compact_capability(fixture.supervisor, fixture.authority, target),
        Err(KernelError::CapabilityCompactionReferenced)
    );
    core.acknowledge_message(fixture.worker, message)
        .expect("message acknowledges");
    core.compact_capability(fixture.supervisor, fixture.authority, target)
        .expect("historical records permit compaction");
    assert_eq!(core.actions()[0].capability, target);
    assert_eq!(core.observations()[0].capability, target);
    assert_eq!(core.checkpoints()[0].capability, target);
    assert_eq!(core.namespace_entries()[0].capability, target);
}

#[test]
fn retired_descendant_resources_use_active_ancestor_cleanup_authority() {
    let (mut core, fixture) = prepared::<96>();
    let child = core
        .create_resource(
            fixture.supervisor,
            ResourceKind::Service,
            Some((fixture.resource, fixture.authority)),
            OperationSet::only(Operation::Rollback).with(Operation::Delegate),
        )
        .expect("child resource creates");
    let leaf = core
        .derive_capability(
            fixture.supervisor,
            child.capability,
            fixture.worker,
            OperationSet::only(Operation::Rollback),
        )
        .expect("child leaf derives");
    core.revoke_derived_capability(fixture.supervisor, child.capability, leaf)
        .expect("child leaf revokes");
    core.retire_resource(fixture.supervisor, child.capability, child.resource)
        .expect("child resource retires");
    core.compact_capability(fixture.supervisor, fixture.authority, leaf)
        .expect("ancestor authority compacts retired child leaf");
    core.revoke_capability(child.capability)
        .expect("child root capability revokes");
    core.compact_capability(fixture.supervisor, fixture.authority, child.capability)
        .expect("ancestor authority compacts retired child root");
}

#[test]
fn nonterminal_unauthorized_and_event_full_failures_are_atomic() {
    let (mut core, fixture) = prepared::<16>();
    let target = core
        .derive_capability(
            fixture.supervisor,
            fixture.authority,
            fixture.worker,
            OperationSet::only(Operation::Observe),
        )
        .expect("target derives");
    assert_eq!(
        core.compact_capability(fixture.supervisor, fixture.authority, target),
        Err(KernelError::CapabilityCompactionNotReady)
    );
    core.revoke_derived_capability(fixture.supervisor, fixture.authority, target)
        .expect("target revokes");
    let observe_only = core
        .derive_capability(
            fixture.supervisor,
            fixture.authority,
            fixture.supervisor,
            OperationSet::only(Operation::Observe),
        )
        .expect("observe authority derives");
    let event_count = core.events().len();
    assert_eq!(
        core.compact_capability(fixture.supervisor, observe_only, target),
        Err(KernelError::OperationDenied)
    );
    assert_eq!(core.events().len(), event_count);
    assert!(core.capability(target).is_ok());

    while core.events().len() < 16 {
        core.observe(fixture.supervisor, fixture.authority, fixture.resource)
            .expect("filler observation fits");
    }
    assert_eq!(
        core.compact_capability(fixture.supervisor, fixture.authority, target),
        Err(KernelError::EventLogFull)
    );
    assert_eq!(core.events().len(), 16);
    assert!(core.capability(target).is_ok());
}
