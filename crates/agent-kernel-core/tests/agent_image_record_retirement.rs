use agent_kernel_core::{
    AgentEntryKind, AgentId, AgentImageDigest, AgentImageId, AgentImageKind, EventKind, IntentKind,
    KernelCore, KernelError, Operation, OperationSet, ResourceKind, RuntimeAdmissionFailure,
    VerificationRequirement,
};

type TestCore<const EVENTS: usize> =
    KernelCore<4, 3, 12, EVENTS, 0, EVENTS, 0, 4, 4, 4, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 4>;

#[derive(Copy, Clone)]
struct Fixture {
    actor: AgentId,
    target: AgentId,
    resource: agent_kernel_core::ResourceId,
    authority: agent_kernel_core::CapabilityId,
}

fn all_operations() -> OperationSet {
    OperationSet::only(Operation::Observe)
        .with(Operation::Act)
        .with(Operation::Verify)
        .with(Operation::Rollback)
        .with(Operation::Delegate)
}

fn setup<const EVENTS: usize>() -> (TestCore<EVENTS>, Fixture) {
    let mut core = TestCore::new();
    let actor = AgentId::new(1);
    let target = AgentId::new(2);
    core.register_agent(actor).expect("actor registers");
    core.register_agent(target).expect("target registers");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource registers");
    let authority = core
        .grant_capability(actor, resource, all_operations())
        .expect("authority grants");
    (
        core,
        Fixture {
            actor,
            target,
            resource,
            authority,
        },
    )
}

fn register_image<const EVENTS: usize>(
    core: &mut TestCore<EVENTS>,
    fixture: Fixture,
    byte: u8,
) -> AgentImageId {
    core.register_agent_image(
        fixture.actor,
        fixture.authority,
        fixture.resource,
        AgentImageKind::Worker,
        AgentImageDigest::new([byte; 32]),
        1,
        u16::from(byte),
    )
    .expect("image registers")
}

#[test]
fn retirement_removes_dense_record_preserves_order_and_reuses_capacity_with_fresh_id() {
    let (mut core, fixture) = setup::<64>();
    let first = register_image(&mut core, fixture, 1);
    let target = register_image(&mut core, fixture, 2);
    let trailing = register_image(&mut core, fixture, 3);
    core.retire_agent_image(fixture.actor, fixture.authority, target)
        .expect("target image retires");
    let record = core.agent_image(target).unwrap();
    let event_start = core.events().len();

    let receipt = core
        .retire_agent_image_record(fixture.actor, fixture.authority, target)
        .expect("terminal image record retires");

    assert_eq!(receipt.record(), record);
    assert_eq!(receipt.image(), target);
    assert_eq!(receipt.actor(), fixture.actor);
    assert_eq!(receipt.authority(), fixture.authority);
    assert_eq!(
        core.agent_images()
            .iter()
            .map(|image| image.id)
            .collect::<Vec<_>>(),
        vec![first, trailing]
    );
    assert_eq!(
        core.agent_image(target),
        Err(KernelError::AgentImageNotFound)
    );

    let event = core.events()[event_start];
    assert_eq!(event.kind, EventKind::AgentImageRecordRetired);
    assert_eq!(event.agent, fixture.actor);
    assert_eq!(event.target_agent, Some(record.owner));
    assert_eq!(event.resource, Some(record.resource));
    assert_eq!(event.capability, Some(fixture.authority));
    assert_eq!(event.operation, Some(Operation::Rollback));
    assert_eq!(event.agent_image, Some(record.id));
    assert_eq!(event.agent_image_kind, Some(record.kind));
    assert_eq!(event.agent_image_digest, Some(record.digest));
    assert_eq!(event.agent_image_abi_version, Some(record.abi_version));
    assert_eq!(event.agent_image_entry_version, Some(record.entry_version));

    let fresh = register_image(&mut core, fixture, 4);
    assert_eq!(fresh.raw(), 4);
    assert_eq!(
        core.agent_images()
            .iter()
            .map(|image| image.id)
            .collect::<Vec<_>>(),
        vec![first, trailing, fresh]
    );
}

#[test]
fn pending_and_verified_images_do_not_meet_record_retirement_gate() {
    let (mut core, fixture) = setup::<32>();
    let image = register_image(&mut core, fixture, 5);
    let events = core.events().len();

    assert_eq!(
        core.retire_agent_image_record(fixture.actor, fixture.authority, image),
        Err(KernelError::AgentImageRecordRetirementNotReady)
    );
    assert_eq!(core.events().len(), events);
    core.verify_agent_image(fixture.actor, fixture.authority, image)
        .unwrap();
    let verified_events = core.events().len();
    assert_eq!(
        core.retire_agent_image_record(fixture.actor, fixture.authority, image),
        Err(KernelError::AgentImageRecordRetirementNotReady)
    );
    assert_eq!(core.events().len(), verified_events);
}

#[test]
fn cleanup_requires_rollback_and_accepts_ancestor_authority_for_retired_resource() {
    let (mut core, fixture) = setup::<64>();
    let child = core
        .create_resource(
            fixture.actor,
            ResourceKind::Service,
            Some((fixture.resource, fixture.authority)),
            all_operations(),
        )
        .unwrap();
    let image = core
        .register_agent_image(
            fixture.actor,
            child.capability,
            child.resource,
            AgentImageKind::Worker,
            AgentImageDigest::new([6; 32]),
            1,
            1,
        )
        .unwrap();
    core.retire_agent_image(fixture.actor, child.capability, image)
        .unwrap();
    let observe_only = core
        .grant_capability(
            fixture.actor,
            child.resource,
            OperationSet::only(Operation::Observe),
        )
        .unwrap();

    assert_eq!(
        core.retire_agent_image_record(fixture.actor, observe_only, image),
        Err(KernelError::OperationDenied)
    );
    assert_eq!(
        core.retire_agent_image_record(fixture.actor, fixture.authority, image),
        Err(KernelError::ResourceMismatch)
    );
    core.retire_resource(fixture.actor, child.capability, child.resource)
        .unwrap();
    assert_eq!(
        core.retire_agent_image_record(fixture.actor, fixture.authority, image)
            .unwrap()
            .image(),
        image
    );
}

#[test]
fn agent_entry_reference_blocks_retirement_without_mutation() {
    let (mut core, fixture) = setup::<64>();
    let target_authority = core
        .derive_capability(
            fixture.actor,
            fixture.authority,
            fixture.target,
            OperationSet::only(Operation::Act),
        )
        .unwrap();
    let image = register_image(&mut core, fixture, 7);
    core.verify_agent_image(fixture.actor, fixture.authority, image)
        .unwrap();
    core.launch_agent(
        fixture.target,
        target_authority,
        fixture.resource,
        image,
        AgentEntryKind::Worker,
        None,
    )
    .unwrap();
    core.retire_agent_image(fixture.actor, fixture.authority, image)
        .unwrap();
    let images = core.agent_images().to_vec();
    let events = core.events().len();

    assert_eq!(
        core.retire_agent_image_record(fixture.actor, fixture.authority, image),
        Err(KernelError::AgentImageRecordRetirementReferenced)
    );
    assert_eq!(core.agent_images(), images.as_slice());
    assert_eq!(core.events().len(), events);
}

#[test]
fn retained_rejected_runtime_admission_reference_blocks_until_compacted() {
    let (mut core, fixture) = setup::<128>();
    let supervisor_image = core
        .register_agent_image(
            fixture.actor,
            fixture.authority,
            fixture.resource,
            AgentImageKind::Supervisor,
            AgentImageDigest::new([8; 32]),
            1,
            1,
        )
        .unwrap();
    core.verify_agent_image(fixture.actor, fixture.authority, supervisor_image)
        .unwrap();
    core.launch_agent(
        fixture.actor,
        fixture.authority,
        fixture.resource,
        supervisor_image,
        AgentEntryKind::Supervisor,
        None,
    )
    .unwrap();
    let intent = core
        .declare_intent(
            fixture.actor,
            fixture.authority,
            fixture.resource,
            IntentKind::Act,
            VerificationRequirement::Optional,
        )
        .unwrap();
    let task = core
        .create_task(fixture.actor, fixture.authority, intent)
        .unwrap();
    core.delegate_task(fixture.actor, fixture.authority, task, fixture.target)
        .unwrap();
    let task_authority = core.task(task).unwrap().delegated_capability.unwrap();
    let image = register_image(&mut core, fixture, 9);
    core.verify_agent_image(fixture.actor, fixture.authority, image)
        .unwrap();
    core.launch_task_agent(
        fixture.target,
        task_authority,
        task,
        image,
        AgentEntryKind::Worker,
    )
    .unwrap();
    core.accept_task(fixture.target, task).unwrap();
    let admission = core
        .request_runtime_admission(fixture.actor, fixture.authority, fixture.target, task)
        .unwrap();
    let permit = core.prepare_next_runtime_admission().unwrap();
    core.reject_runtime_admission(permit, RuntimeAdmissionFailure::AllocationUnavailable)
        .unwrap();
    core.cancel_task(fixture.actor, fixture.authority, task)
        .unwrap();
    core.retire_agent_entry(fixture.actor, fixture.authority, fixture.target)
        .unwrap();
    core.retire_agent_image(fixture.actor, fixture.authority, image)
        .unwrap();

    assert_eq!(
        core.retire_agent_image_record(fixture.actor, fixture.authority, image),
        Err(KernelError::AgentImageRecordRetirementReferenced)
    );
    core.compact_runtime_admission_prefix(fixture.actor, fixture.authority, admission)
        .unwrap();
    assert_eq!(
        core.retire_agent_image_record(fixture.actor, fixture.authority, image)
            .unwrap()
            .image(),
        image
    );
}

#[test]
fn missing_image_and_full_event_log_leave_store_unchanged() {
    let (mut core, fixture) = setup::<8>();
    assert_eq!(
        core.retire_agent_image_record(fixture.actor, fixture.authority, AgentImageId::new(99)),
        Err(KernelError::AgentImageNotFound)
    );
    let image = register_image(&mut core, fixture, 10);
    core.retire_agent_image(fixture.actor, fixture.authority, image)
        .unwrap();
    while core.events().len() < 8 {
        core.observe(fixture.actor, fixture.authority, fixture.resource)
            .expect("filler observation fits");
    }
    let images = core.agent_images().to_vec();

    assert_eq!(
        core.retire_agent_image_record(fixture.actor, fixture.authority, image),
        Err(KernelError::EventLogFull)
    );
    assert_eq!(core.agent_images(), images.as_slice());
    assert_eq!(core.events().len(), 8);
}
