use agent_kernel_core::{
    AgentId, AgentImageDigest, AgentImageKind, AgentImageStatus, EventKind, KernelCore,
    KernelError, Operation, OperationSet, ResourceKind,
};

type ImageCore = KernelCore<2, 2, 4, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2>;

fn digest(byte: u8) -> AgentImageDigest {
    AgentImageDigest::new([byte; 32])
}

fn prepare_owner(
    core: &mut ImageCore,
    operations: OperationSet,
) -> (
    AgentId,
    agent_kernel_core::CapabilityId,
    agent_kernel_core::ResourceId,
) {
    let owner = AgentId::new(1);
    core.register_agent(owner).expect("owner should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(owner, resource, operations)
        .expect("capability should fit");
    (owner, capability, resource)
}

#[test]
fn register_agent_image_stores_metadata_and_replayable_event() {
    let mut core = ImageCore::new();
    let (owner, capability, resource) =
        prepare_owner(&mut core, OperationSet::only(Operation::Act));
    let image_digest = digest(7);

    let image = core
        .register_agent_image(
            owner,
            capability,
            resource,
            AgentImageKind::Supervisor,
            image_digest,
            1,
            2,
        )
        .expect("image should register");

    assert_eq!(image.raw(), 1);
    let image_record = core.agent_image(image).expect("image should be queryable");
    assert_eq!(image_record.id, image);
    assert_eq!(image_record.owner, owner);
    assert_eq!(image_record.resource, resource);
    assert_eq!(image_record.kind, AgentImageKind::Supervisor);
    assert_eq!(image_record.digest, image_digest);
    assert_eq!(image_record.abi_version, 1);
    assert_eq!(image_record.entry_version, 2);
    assert_eq!(image_record.status, AgentImageStatus::Pending);

    let event = core
        .events()
        .last()
        .expect("registration should record event");
    assert_eq!(event.kind, EventKind::AgentImageRegistered);
    assert_eq!(event.agent, owner);
    assert_eq!(event.resource, Some(resource));
    assert_eq!(event.capability, Some(capability));
    assert_eq!(event.agent_image, Some(image));
    assert_eq!(event.agent_image_kind, Some(AgentImageKind::Supervisor));
    assert_eq!(event.agent_image_digest, Some(image_digest));
    assert_eq!(event.agent_image_abi_version, Some(1));
    assert_eq!(event.agent_image_entry_version, Some(2));
}

#[test]
fn register_agent_image_requires_act_authority_without_mutation() {
    let mut core = ImageCore::new();
    let (owner, capability, resource) =
        prepare_owner(&mut core, OperationSet::only(Operation::Observe));
    let events_before = core.events().len();

    let result = core.register_agent_image(
        owner,
        capability,
        resource,
        AgentImageKind::Worker,
        digest(3),
        1,
        1,
    );

    assert_eq!(result, Err(KernelError::OperationDenied));
    assert!(core.agent_images().is_empty());
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn register_agent_image_rejects_zero_versions_without_mutation() {
    let mut core = ImageCore::new();
    let (owner, capability, resource) =
        prepare_owner(&mut core, OperationSet::only(Operation::Act));
    let events_before = core.events().len();

    let abi_result = core.register_agent_image(
        owner,
        capability,
        resource,
        AgentImageKind::Worker,
        digest(4),
        0,
        1,
    );
    let entry_result = core.register_agent_image(
        owner,
        capability,
        resource,
        AgentImageKind::Worker,
        digest(5),
        1,
        0,
    );

    assert_eq!(abi_result, Err(KernelError::AgentImageVersionInvalid));
    assert_eq!(entry_result, Err(KernelError::AgentImageVersionInvalid));
    assert!(core.agent_images().is_empty());
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn register_agent_image_store_full_leaves_event_log_unchanged() {
    let mut core = KernelCore::<2, 2, 4, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0>::new();
    let owner = AgentId::new(1);
    core.register_agent(owner).expect("owner should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(owner, resource, OperationSet::only(Operation::Act))
        .expect("capability should fit");
    let events_before = core.events().len();

    let result = core.register_agent_image(
        owner,
        capability,
        resource,
        AgentImageKind::Worker,
        digest(6),
        1,
        1,
    );

    assert_eq!(result, Err(KernelError::AgentImageStoreFull));
    assert!(core.agent_images().is_empty());
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn retire_agent_image_marks_retired_and_records_event() {
    let mut core = ImageCore::new();
    let (owner, capability, resource) = prepare_owner(
        &mut core,
        OperationSet::empty()
            .with(Operation::Act)
            .with(Operation::Rollback),
    );
    let image = core
        .register_agent_image(
            owner,
            capability,
            resource,
            AgentImageKind::Worker,
            digest(8),
            1,
            1,
        )
        .expect("image should register");

    let event = core
        .retire_agent_image(owner, capability, image)
        .expect("image should retire");

    assert_eq!(event.kind, EventKind::AgentImageRetired);
    assert_eq!(event.agent_image, Some(image));
    assert_eq!(event.agent_image_kind, Some(AgentImageKind::Worker));
    assert_eq!(event.agent_image_digest, None);
    assert_eq!(event.agent_image_abi_version, None);
    assert_eq!(event.agent_image_entry_version, None);
    assert_eq!(
        core.agent_image(image)
            .expect("image should remain queryable")
            .status,
        AgentImageStatus::Retired
    );
}

#[test]
fn retire_agent_image_requires_rollback_without_mutation() {
    let mut core = ImageCore::new();
    let (owner, capability, resource) =
        prepare_owner(&mut core, OperationSet::only(Operation::Act));
    let image = core
        .register_agent_image(
            owner,
            capability,
            resource,
            AgentImageKind::Worker,
            digest(9),
            1,
            1,
        )
        .expect("image should register");
    let events_before = core.events().len();

    let result = core.retire_agent_image(owner, capability, image);

    assert_eq!(result, Err(KernelError::OperationDenied));
    assert_eq!(
        core.agent_image(image)
            .expect("image should remain queryable")
            .status,
        AgentImageStatus::Pending
    );
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn verify_agent_image_marks_verified_and_records_event() {
    let mut core = ImageCore::new();
    let (owner, capability, resource) = prepare_owner(
        &mut core,
        OperationSet::empty()
            .with(Operation::Act)
            .with(Operation::Verify),
    );
    let image = core
        .register_agent_image(
            owner,
            capability,
            resource,
            AgentImageKind::Worker,
            digest(10),
            1,
            1,
        )
        .expect("image should register");

    let event = core
        .verify_agent_image(owner, capability, image)
        .expect("image should verify");

    assert_eq!(event.kind, EventKind::AgentImageVerified);
    assert_eq!(event.agent, owner);
    assert_eq!(event.resource, Some(resource));
    assert_eq!(event.capability, Some(capability));
    assert_eq!(event.agent_image, Some(image));
    assert_eq!(event.agent_image_kind, Some(AgentImageKind::Worker));
    assert_eq!(event.agent_image_digest, None);
    assert_eq!(event.agent_image_abi_version, None);
    assert_eq!(event.agent_image_entry_version, None);
    assert_eq!(
        core.agent_image(image)
            .expect("image should remain queryable")
            .status,
        AgentImageStatus::Verified
    );
}

#[test]
fn verify_agent_image_requires_verify_authority_without_mutation() {
    let mut core = ImageCore::new();
    let (owner, capability, resource) =
        prepare_owner(&mut core, OperationSet::only(Operation::Act));
    let image = core
        .register_agent_image(
            owner,
            capability,
            resource,
            AgentImageKind::Worker,
            digest(11),
            1,
            1,
        )
        .expect("image should register");
    let events_before = core.events().len();

    let result = core.verify_agent_image(owner, capability, image);

    assert_eq!(result, Err(KernelError::OperationDenied));
    assert_eq!(
        core.agent_image(image)
            .expect("image should remain queryable")
            .status,
        AgentImageStatus::Pending
    );
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn verify_agent_image_rejects_non_owner_without_mutation() {
    let mut core = ImageCore::new();
    let owner = AgentId::new(1);
    let other = AgentId::new(2);
    let (prepared_owner, owner_capability, resource) = prepare_owner(
        &mut core,
        OperationSet::empty()
            .with(Operation::Act)
            .with(Operation::Verify),
    );
    core.register_agent(other)
        .expect("other agent should register");
    let other_capability = core
        .grant_capability(other, resource, OperationSet::only(Operation::Verify))
        .expect("other capability should fit");
    assert_eq!(owner, prepared_owner);
    assert_eq!(owner, AgentId::new(1));
    let image = core
        .register_agent_image(
            owner,
            owner_capability,
            resource,
            AgentImageKind::Worker,
            digest(12),
            1,
            1,
        )
        .expect("image should register");
    let events_before = core.events().len();

    let result = core.verify_agent_image(other, other_capability, image);

    assert_eq!(result, Err(KernelError::AgentMismatch));
    assert_eq!(
        core.agent_image(image)
            .expect("image should remain queryable")
            .status,
        AgentImageStatus::Pending
    );
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn verify_agent_image_rejects_repeated_or_retired_status_without_mutation() {
    let mut core = ImageCore::new();
    let (owner, capability, resource) = prepare_owner(
        &mut core,
        OperationSet::empty()
            .with(Operation::Act)
            .with(Operation::Verify)
            .with(Operation::Rollback),
    );
    let verified_image = core
        .register_agent_image(
            owner,
            capability,
            resource,
            AgentImageKind::Worker,
            digest(13),
            1,
            1,
        )
        .expect("verified image should register");
    core.verify_agent_image(owner, capability, verified_image)
        .expect("image should verify once");
    let events_after_verify = core.events().len();

    let repeated = core.verify_agent_image(owner, capability, verified_image);

    assert_eq!(repeated, Err(KernelError::AgentImageStatusMismatch));
    assert_eq!(core.events().len(), events_after_verify);
    assert_eq!(
        core.agent_image(verified_image)
            .expect("image should remain queryable")
            .status,
        AgentImageStatus::Verified
    );

    let retired_image = core
        .register_agent_image(
            owner,
            capability,
            resource,
            AgentImageKind::Worker,
            digest(14),
            1,
            1,
        )
        .expect("retired image should register");
    core.retire_agent_image(owner, capability, retired_image)
        .expect("pending image should retire");
    let events_after_retire = core.events().len();

    let retired = core.verify_agent_image(owner, capability, retired_image);

    assert_eq!(retired, Err(KernelError::AgentImageRetired));
    assert_eq!(core.events().len(), events_after_retire);
}

#[test]
fn verify_agent_image_event_log_full_leaves_pending() {
    let mut core = KernelCore::<2, 2, 4, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2>::new();
    let owner = AgentId::new(1);
    core.register_agent(owner).expect("owner should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = core
        .grant_capability(
            owner,
            resource,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Verify),
        )
        .expect("capability should fit");
    let image = core
        .register_agent_image(
            owner,
            capability,
            resource,
            AgentImageKind::Worker,
            digest(15),
            1,
            1,
        )
        .expect("image should register");
    core.grant_capability(owner, resource, OperationSet::only(Operation::Observe))
        .expect("filler capability should fit");

    let result = core.verify_agent_image(owner, capability, image);

    assert_eq!(result, Err(KernelError::EventLogFull));
    assert_eq!(
        core.agent_image(image)
            .expect("image should remain queryable")
            .status,
        AgentImageStatus::Pending
    );
    assert_eq!(core.events().len(), 4);
}
