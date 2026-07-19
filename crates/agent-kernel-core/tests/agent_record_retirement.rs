use agent_kernel_core::{
    AgentEntryKind, AgentExecutionState, AgentId, AgentImageDigest, AgentImageKind, AgentStatus,
    EventKind, IntentKind, KernelCore, KernelError, MessageKind, MessagePayload, Operation,
    OperationSet, ResourceKind, VerificationRequirement,
};

type TestCore<const EVENTS: usize> =
    KernelCore<8, 4, 16, EVENTS, 2, 2, 2, 2, 2, 2, 4, 2, 2, 2, 2, 2, 2, 4, 2, 2, 2, 2, 2>;

#[derive(Copy, Clone)]
struct Fixture {
    manager: AgentId,
    target: AgentId,
    resource: agent_kernel_core::ResourceId,
    authority: agent_kernel_core::CapabilityId,
}

fn setup<const EVENTS: usize>(operations: OperationSet) -> (TestCore<EVENTS>, Fixture) {
    let mut core = TestCore::new();
    let manager = AgentId::new(1);
    let target = AgentId::new(9);
    core.register_agent(manager).expect("manager registers");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("management resource registers");
    let authority = core
        .grant_capability(manager, resource, operations)
        .expect("management authority grants");
    core.register_managed_agent(manager, authority, resource, target)
        .expect("managed target registers");
    (
        core,
        Fixture {
            manager,
            target,
            resource,
            authority,
        },
    )
}

fn retire_target<const EVENTS: usize>(core: &mut TestCore<EVENTS>, fixture: Fixture) {
    core.retire_managed_agent(fixture.manager, fixture.authority, fixture.target)
        .expect("managed target retires");
}

#[test]
fn retirement_removes_paired_dense_records_and_reuses_capacity_without_identity_aliasing() {
    let (mut core, fixture) = setup::<64>(OperationSet::only(Operation::Delegate));
    let trailing = AgentId::new(2);
    core.register_agent(trailing)
        .expect("trailing agent registers");
    retire_target(&mut core, fixture);
    let target_record = core
        .agents()
        .iter()
        .find(|record| record.id == fixture.target)
        .copied()
        .unwrap();
    let target_context = core.execution_context(fixture.target).unwrap();
    let event_start = core.events().len();

    let receipt = core
        .retire_agent_record(fixture.manager, fixture.authority, fixture.target)
        .expect("retired unreferenced record is reclaimable");

    assert_eq!(receipt.record(), target_record);
    assert_eq!(receipt.context(), target_context);
    assert_eq!(receipt.actor(), fixture.manager);
    assert_eq!(receipt.authority(), fixture.authority);
    assert_eq!(receipt.management_resource(), fixture.resource);
    assert_eq!(receipt.retired_floor(), fixture.target);
    assert_eq!(core.retired_agent_floor(), fixture.target);
    assert_eq!(
        core.agents()
            .iter()
            .map(|record| record.id)
            .collect::<Vec<_>>(),
        vec![fixture.manager, trailing]
    );
    assert_eq!(
        core.execution_contexts()
            .iter()
            .map(|context| context.agent)
            .collect::<Vec<_>>(),
        vec![fixture.manager, trailing]
    );
    assert_eq!(
        core.execution_context(fixture.target),
        Err(KernelError::AgentNotFound)
    );

    let event = core.events()[event_start];
    assert_eq!(event.kind, EventKind::AgentRecordRetired);
    assert_eq!(event.agent, fixture.manager);
    assert_eq!(event.target_agent, Some(fixture.target));
    assert_eq!(event.resource, Some(fixture.resource));
    assert_eq!(event.capability, Some(fixture.authority));
    assert_eq!(event.operation, Some(Operation::Delegate));

    assert_eq!(
        core.register_managed_agent(
            fixture.manager,
            fixture.authority,
            fixture.resource,
            fixture.target,
        ),
        Err(KernelError::AgentIdStale)
    );
    assert_eq!(
        core.register_agent(AgentId::new(8)),
        Err(KernelError::AgentIdStale)
    );
    assert_eq!(
        core.register_agent(AgentId::new(0)),
        Err(KernelError::AgentIdStale)
    );
    assert_eq!(
        core.register_agent(fixture.manager),
        Err(KernelError::AgentAlreadyExists)
    );

    let fresh = AgentId::new(15);
    core.register_managed_agent(fixture.manager, fixture.authority, fixture.resource, fresh)
        .expect("fresh identity reuses returned capacity");
    assert_eq!(core.agents().last().unwrap().id, fresh);
    assert_eq!(core.execution_contexts().last().unwrap().agent, fresh);
    assert_eq!(core.retired_agent_floor(), fixture.target);
}

#[test]
fn retirement_high_water_never_moves_backward() {
    let (mut core, fixture) = setup::<64>(OperationSet::only(Operation::Delegate));
    let higher = AgentId::new(15);
    core.register_managed_agent(fixture.manager, fixture.authority, fixture.resource, higher)
        .unwrap();
    core.retire_managed_agent(fixture.manager, fixture.authority, fixture.target)
        .unwrap();
    core.retire_managed_agent(fixture.manager, fixture.authority, higher)
        .unwrap();

    let higher_retirement = core
        .retire_agent_record(fixture.manager, fixture.authority, higher)
        .unwrap();
    let lower_retirement = core
        .retire_agent_record(fixture.manager, fixture.authority, fixture.target)
        .unwrap();

    assert_eq!(higher_retirement.retired_floor(), higher);
    assert_eq!(lower_retirement.retired_floor(), higher);
    assert_eq!(core.retired_agent_floor(), higher);
    assert_eq!(
        core.register_agent(AgentId::new(12)),
        Err(KernelError::AgentIdStale)
    );
}

#[test]
fn lifecycle_and_execution_state_must_be_terminal_and_idle() {
    let operations = OperationSet::only(Operation::Act)
        .with(Operation::Delegate)
        .with(Operation::Verify);
    let (mut core, fixture) = setup::<64>(operations);
    assert_eq!(
        core.retire_agent_record(fixture.manager, fixture.authority, fixture.target),
        Err(KernelError::AgentRecordRetirementNotReady)
    );
    core.suspend_managed_agent(fixture.manager, fixture.authority, fixture.target)
        .unwrap();
    assert_eq!(
        core.retire_agent_record(fixture.manager, fixture.authority, fixture.target),
        Err(KernelError::AgentRecordRetirementNotReady)
    );
    core.resume_managed_agent(fixture.manager, fixture.authority, fixture.target)
        .unwrap();

    let intent = core
        .declare_intent(
            fixture.manager,
            fixture.authority,
            fixture.resource,
            IntentKind::Act,
            VerificationRequirement::Optional,
        )
        .unwrap();
    let task = core
        .create_task(fixture.manager, fixture.authority, intent)
        .unwrap();
    core.delegate_task(fixture.manager, fixture.authority, task, fixture.target)
        .unwrap();
    let delegated = core.task(task).unwrap().delegated_capability.unwrap();
    let image = core
        .register_agent_image(
            fixture.manager,
            fixture.authority,
            fixture.resource,
            AgentImageKind::Worker,
            AgentImageDigest::new([7; 32]),
            1,
            1,
        )
        .unwrap();
    core.verify_agent_image(fixture.manager, fixture.authority, image)
        .unwrap();
    core.launch_task_agent(
        fixture.target,
        delegated,
        task,
        image,
        AgentEntryKind::Worker,
    )
    .unwrap();
    core.accept_task(fixture.target, task).unwrap();
    core.enqueue_task(fixture.target, task).unwrap();
    core.dispatch_next(fixture.target).unwrap();
    core.retire_agent(fixture.target).unwrap();
    assert_eq!(
        core.execution_context(fixture.target).unwrap().state,
        AgentExecutionState::Running
    );
    assert_eq!(
        core.retire_agent_record(fixture.manager, fixture.authority, fixture.target),
        Err(KernelError::AgentRecordRetirementNotReady)
    );
}

#[test]
fn unmanaged_retired_identity_has_no_administrative_retirement_path() {
    let (mut core, fixture) = setup::<32>(OperationSet::only(Operation::Delegate));
    let trusted = AgentId::new(8);
    core.register_agent(trusted).unwrap();
    core.retire_agent(trusted).unwrap();

    assert_eq!(
        core.retire_agent_record(fixture.manager, fixture.authority, trusted),
        Err(KernelError::AgentManagementDenied)
    );
}

#[test]
fn exact_delegate_authority_and_active_ancestry_are_required() {
    let (mut core, fixture) = setup::<64>(OperationSet::only(Operation::Delegate));
    let delegate = AgentId::new(2);
    let delegated = AgentId::new(3);
    core.register_agent(delegate).unwrap();
    core.register_agent(delegated).unwrap();
    let first = core
        .derive_capability(
            fixture.manager,
            fixture.authority,
            delegate,
            OperationSet::only(Operation::Delegate),
        )
        .unwrap();
    let second = core
        .derive_capability(
            delegate,
            first,
            delegated,
            OperationSet::only(Operation::Delegate),
        )
        .unwrap();
    retire_target(&mut core, fixture);
    core.revoke_derived_capability(fixture.manager, fixture.authority, first)
        .unwrap();

    assert_eq!(
        core.retire_agent_record(delegated, second, fixture.target),
        Err(KernelError::CapabilityRevoked)
    );

    let wrong = core
        .grant_capability(
            fixture.manager,
            fixture.resource,
            OperationSet::only(Operation::Act),
        )
        .unwrap();
    assert_eq!(
        core.retire_agent_record(fixture.manager, wrong, fixture.target),
        Err(KernelError::OperationDenied)
    );
    assert!(core
        .agents()
        .iter()
        .any(|record| record.id == fixture.target));
    assert_eq!(core.retired_agent_floor(), AgentId::new(0));
}

#[test]
fn capability_and_message_references_block_retirement() {
    let (mut capability_core, fixture) = setup::<48>(OperationSet::only(Operation::Delegate));
    capability_core
        .derive_capability(
            fixture.manager,
            fixture.authority,
            fixture.target,
            OperationSet::only(Operation::Delegate),
        )
        .unwrap();
    retire_target(&mut capability_core, fixture);
    assert_eq!(
        capability_core.retire_agent_record(fixture.manager, fixture.authority, fixture.target),
        Err(KernelError::AgentRecordRetirementReferenced)
    );

    let (mut message_core, fixture) = setup::<48>(OperationSet::only(Operation::Delegate));
    message_core
        .send_message(
            fixture.target,
            fixture.manager,
            MessageKind::Notify,
            MessagePayload::empty(),
        )
        .unwrap();
    retire_target(&mut message_core, fixture);
    assert_eq!(
        message_core.retire_agent_record(fixture.manager, fixture.authority, fixture.target),
        Err(KernelError::AgentRecordRetirementReferenced)
    );
}

#[test]
fn missing_target_and_event_exhaustion_preserve_paired_state_and_floor() {
    let (mut core, fixture) = setup::<4>(OperationSet::only(Operation::Delegate));
    assert_eq!(
        core.retire_agent_record(fixture.manager, fixture.authority, AgentId::new(77)),
        Err(KernelError::AgentNotFound)
    );
    retire_target(&mut core, fixture);
    assert_eq!(core.events().len(), 4);
    let agents = core.agents().to_vec();
    let contexts = core.execution_contexts().to_vec();

    assert_eq!(
        core.retire_agent_record(fixture.manager, fixture.authority, fixture.target),
        Err(KernelError::EventLogFull)
    );
    assert_eq!(core.agents(), agents.as_slice());
    assert_eq!(core.execution_contexts(), contexts.as_slice());
    assert_eq!(core.retired_agent_floor(), AgentId::new(0));
    assert_eq!(core.events().len(), 4);
    assert_eq!(
        core.agents()
            .iter()
            .find(|record| record.id == fixture.target)
            .unwrap()
            .status,
        AgentStatus::Retired
    );
}
