mod agent_entry_retirement_support;

use agent_kernel_core::{
    AgentEntryKind, AgentId, AgentImageKind, AgentStatus, EventKind, FaultKind, KernelError,
    MessageKind, MessagePayload, Operation, OperationSet, ResourceKind, RuntimeAdmissionStatus,
};

use agent_entry_retirement_support::{
    all_operations, complete_and_verify, launch_task, prepared, register_image,
};

#[test]
fn terminal_entry_retirement_records_identity_shifts_dense_store_and_allows_relaunch() {
    let (mut core, fixture) = prepared::<128>();
    let first = launch_task(
        &mut core,
        fixture,
        fixture.worker,
        fixture.resource,
        fixture.authority,
        2,
    );
    let retained = launch_task(
        &mut core,
        fixture,
        fixture.other,
        fixture.resource,
        fixture.authority,
        3,
    );
    complete_and_verify(&mut core, fixture, first, fixture.authority);
    complete_and_verify(&mut core, fixture, retained, fixture.authority);
    let retired = core.agent_entry(fixture.worker).unwrap();
    let retained_record = core.agent_entry(fixture.other).unwrap();
    let event_start = core.events().len();

    let receipt = core
        .retire_agent_entry(fixture.supervisor, fixture.authority, fixture.worker)
        .expect("terminal entry retires");

    assert_eq!(receipt.entry(), retired);
    assert_eq!(receipt.agent(), fixture.worker);
    assert_eq!(core.agent_entry_capacity(), 6);
    assert_eq!(core.agent_entry_count(), 2);
    assert_eq!(
        core.agent_entry(fixture.worker),
        Err(KernelError::AgentEntryNotFound)
    );
    assert_eq!(
        core.agent_entries(),
        &[
            core.agent_entry(fixture.supervisor).unwrap(),
            retained_record
        ]
    );
    let event = core.events()[event_start];
    assert_eq!(event.kind, EventKind::AgentEntryRetired);
    assert_eq!(event.agent, fixture.supervisor);
    assert_eq!(event.target_agent, Some(fixture.worker));
    assert_eq!(event.resource, Some(retired.resource));
    assert_eq!(event.capability, Some(retired.capability));
    assert_eq!(event.source_capability, Some(fixture.authority));
    assert_eq!(event.operation, Some(Operation::Rollback));
    assert_eq!(event.agent_image, Some(retired.image));
    assert_eq!(event.agent_image_kind, Some(AgentImageKind::Worker));
    assert_eq!(event.intent, retired.intent);
    assert_eq!(event.task, retired.task);

    let second = launch_task(
        &mut core,
        fixture,
        fixture.worker,
        fixture.resource,
        fixture.authority,
        4,
    );
    assert_eq!(core.agent_entry_count(), 3);
    assert_eq!(
        core.agent_entry(fixture.worker).unwrap().task,
        Some(second.task)
    );
    assert_ne!(second.task, first.task);
    assert_ne!(second.capability, first.capability);
}

#[test]
fn task_scope_and_open_ended_entries_require_explicit_terminal_state() {
    let (mut core, fixture) = prepared::<96>();
    let launch = launch_task(
        &mut core,
        fixture,
        fixture.worker,
        fixture.resource,
        fixture.authority,
        2,
    );
    assert_eq!(
        core.retire_agent_entry(fixture.supervisor, fixture.authority, fixture.worker),
        Err(KernelError::AgentEntryRetirementNotReady)
    );
    core.cancel_task(fixture.supervisor, fixture.authority, launch.task)
        .expect("accepted task cancels");
    core.retire_agent_entry(fixture.supervisor, fixture.authority, fixture.worker)
        .expect("cancelled task entry retires");

    let open_capability = core
        .derive_capability(
            fixture.supervisor,
            fixture.authority,
            fixture.other,
            OperationSet::only(Operation::Act),
        )
        .expect("open authority derives");
    let image = register_image(
        &mut core,
        fixture.supervisor,
        fixture.authority,
        fixture.resource,
        AgentImageKind::Worker,
        3,
    );
    core.launch_agent(
        fixture.other,
        open_capability,
        fixture.resource,
        image,
        AgentEntryKind::Worker,
        None,
    )
    .expect("open entry launches");
    assert_eq!(
        core.retire_agent_entry(fixture.supervisor, fixture.authority, fixture.other),
        Err(KernelError::AgentEntryRetirementNotReady)
    );
    core.retire_agent(fixture.other)
        .expect("target agent retires");
    assert_eq!(
        core.agents()
            .iter()
            .find(|agent| agent.id == fixture.other)
            .unwrap()
            .status,
        AgentStatus::Retired
    );
    core.retire_agent_entry(fixture.supervisor, fixture.authority, fixture.other)
        .expect("retired open entry retires");
}

#[test]
fn admitted_runtime_reference_blocks_until_semantic_release() {
    let (mut core, fixture) = prepared::<128>();
    let launch = launch_task(
        &mut core,
        fixture,
        fixture.worker,
        fixture.resource,
        fixture.authority,
        2,
    );
    let admission = core
        .request_runtime_admission(
            fixture.supervisor,
            fixture.authority,
            fixture.worker,
            launch.task,
        )
        .expect("admission requests");
    let permit = core.prepare_next_runtime_admission().unwrap();
    core.commit_runtime_admission(permit).unwrap();
    core.dispatch_next(fixture.worker).unwrap();
    core.complete_task(fixture.worker, launch.capability, launch.task)
        .unwrap();
    core.verify_task(fixture.supervisor, fixture.authority, launch.task)
        .unwrap();
    assert_eq!(
        core.retire_agent_entry(fixture.supervisor, fixture.authority, fixture.worker),
        Err(KernelError::AgentEntryRetirementReferenced)
    );

    let release = core
        .prepare_runtime_admission_release_batch([admission])
        .expect("release prepares");
    core.commit_runtime_admission_release_batch(release)
        .expect("release commits");
    assert_eq!(
        core.runtime_admission(admission).unwrap().status,
        RuntimeAdmissionStatus::Released
    );
    core.retire_agent_entry(fixture.supervisor, fixture.authority, fixture.worker)
        .expect("released admission stays historical");
}

#[test]
fn received_message_blocks_until_acknowledgement() {
    let (mut core, fixture) = prepared::<96>();
    let launch = launch_task(
        &mut core,
        fixture,
        fixture.worker,
        fixture.resource,
        fixture.authority,
        2,
    );
    complete_and_verify(&mut core, fixture, launch, fixture.authority);
    let message = core
        .send_message(
            fixture.supervisor,
            fixture.worker,
            MessageKind::Notify,
            MessagePayload::empty(),
        )
        .expect("message sends");
    core.receive_message(fixture.worker)
        .expect("message receives");
    assert_eq!(
        core.retire_agent_entry(fixture.supervisor, fixture.authority, fixture.worker),
        Err(KernelError::AgentEntryRetirementReferenced)
    );
    core.acknowledge_message(fixture.worker, message)
        .expect("message acknowledges");
    core.retire_agent_entry(fixture.supervisor, fixture.authority, fixture.worker)
        .expect("acknowledged message stays historical");
}

#[test]
fn fault_handler_and_driver_bindings_are_persistent_entry_references() {
    let (mut core, fixture) = prepared::<128>();
    let handler_launch = launch_task(
        &mut core,
        fixture,
        fixture.worker,
        fixture.resource,
        fixture.authority,
        2,
    );
    complete_and_verify(&mut core, fixture, handler_launch, fixture.authority);
    core.install_fault_handler(
        fixture.supervisor,
        fixture.authority,
        fixture.resource,
        FaultKind::ExecutionTrap,
        fixture.worker,
    )
    .expect("handler installs");
    assert_eq!(
        core.retire_agent_entry(fixture.supervisor, fixture.authority, fixture.worker),
        Err(KernelError::AgentEntryRetirementReferenced)
    );

    let driver_launch = launch_task(
        &mut core,
        fixture,
        fixture.other,
        fixture.resource,
        fixture.authority,
        3,
    );
    complete_and_verify(&mut core, fixture, driver_launch, fixture.authority);
    let device = core
        .register_resource(ResourceKind::Device, None)
        .expect("device registers");
    let device_authority = core
        .grant_capability(
            fixture.supervisor,
            device,
            OperationSet::only(Operation::Delegate),
        )
        .expect("device authority grants");
    core.bind_driver(fixture.supervisor, device_authority, device, fixture.other)
        .expect("driver binds");
    assert_eq!(
        core.retire_agent_entry(fixture.supervisor, fixture.authority, fixture.other),
        Err(KernelError::AgentEntryRetirementReferenced)
    );
}

#[test]
fn compacted_scope_on_retired_resource_uses_active_ancestor_cleanup_authority() {
    let (mut core, fixture) = prepared::<128>();
    let child = core
        .create_resource(
            fixture.supervisor,
            ResourceKind::Service,
            Some((fixture.resource, fixture.authority)),
            all_operations(),
        )
        .expect("child resource creates");
    let launch = launch_task(
        &mut core,
        fixture,
        fixture.worker,
        child.resource,
        child.capability,
        2,
    );
    complete_and_verify(&mut core, fixture, launch, child.capability);
    core.compact_task_prefix(fixture.supervisor, child.capability, launch.task)
        .expect("terminal task compacts");
    core.compact_intent_prefix(fixture.supervisor, child.capability, launch.intent)
        .expect("terminal intent compacts");
    core.retire_resource(fixture.supervisor, child.capability, child.resource)
        .expect("child resource retires");

    let receipt = core
        .retire_agent_entry(fixture.supervisor, fixture.authority, fixture.worker)
        .expect("ancestor authority retires historical entry");
    assert_eq!(receipt.entry().task, Some(launch.task));
    assert_eq!(receipt.entry().intent, Some(launch.intent));
    assert_eq!(receipt.entry().resource, child.resource);
}

#[test]
fn unknown_unauthorized_worker_and_event_full_failures_are_atomic() {
    let (mut core, fixture) = prepared::<64>();
    let launch = launch_task(
        &mut core,
        fixture,
        fixture.worker,
        fixture.resource,
        fixture.authority,
        2,
    );
    complete_and_verify(&mut core, fixture, launch, fixture.authority);
    let observe_only = core
        .derive_capability(
            fixture.supervisor,
            fixture.authority,
            fixture.supervisor,
            OperationSet::only(Operation::Observe),
        )
        .expect("observe authority derives");
    let entries = core.agent_entries().to_vec();
    let events = core.events().len();

    assert_eq!(
        core.retire_agent_entry(fixture.supervisor, fixture.authority, AgentId::new(99)),
        Err(KernelError::AgentEntryNotFound)
    );
    assert_eq!(
        core.retire_agent_entry(fixture.supervisor, observe_only, fixture.worker),
        Err(KernelError::OperationDenied)
    );
    assert_eq!(
        core.retire_agent_entry(fixture.worker, launch.capability, fixture.supervisor),
        Err(KernelError::AgentEntryKindMismatch)
    );
    assert_eq!(core.agent_entries(), entries.as_slice());
    assert_eq!(core.events().len(), events);

    while core.events().len() < 64 {
        core.observe(fixture.supervisor, fixture.authority, fixture.resource)
            .expect("filler observation fits");
    }
    assert_eq!(
        core.retire_agent_entry(fixture.supervisor, fixture.authority, fixture.worker),
        Err(KernelError::EventLogFull)
    );
    assert_eq!(core.agent_entries(), entries.as_slice());
    assert_eq!(core.events().len(), 64);
}
