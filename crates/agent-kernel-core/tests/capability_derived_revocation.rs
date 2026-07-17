use agent_kernel_core::{
    AgentId, CapabilityId, EventKind, IntentKind, KernelCore, KernelError, Operation, OperationSet,
    ResourceKind, VerificationRequirement,
};

type TestCore = KernelCore<3, 1, 5, 12, 0, 1, 0, 0, 0, 0>;

#[test]
fn source_owner_revokes_directly_derived_capability_with_auditable_lineage() {
    let (mut core, owner, target, resource, source, derived) = prepared_capabilities();

    let event = core
        .revoke_derived_capability(owner, source, derived)
        .expect("source owner should revoke its direct child");

    assert_eq!(event.kind, EventKind::CapabilityRevoked);
    assert_eq!(event.agent, owner);
    assert_eq!(event.resource, Some(resource));
    assert_eq!(event.capability, Some(derived));
    assert_eq!(event.source_capability, Some(source));
    assert_eq!(event.operations, OperationSet::only(Operation::Observe));
    assert_eq!(event.target_agent, Some(target));
    assert!(!core.capability(source).unwrap().revoked);
    assert!(core.capability(derived).unwrap().revoked);

    let events_before = core.events().len();
    assert_eq!(
        core.observe(target, derived, resource),
        Err(KernelError::CapabilityRevoked)
    );
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn derived_revocation_rejects_wrong_actor_missing_delegate_and_wrong_parent() {
    let (mut core, owner, target, _resource, source, derived) = prepared_capabilities();
    let other = AgentId::new(3);
    let events_before = core.events().len();

    assert_eq!(
        core.revoke_derived_capability(other, source, derived),
        Err(KernelError::AgentMismatch)
    );

    let non_delegating = core
        .grant_capability(
            owner,
            core.capability(source).unwrap().resource,
            OperationSet::only(Operation::Observe),
        )
        .expect("second source should fit");
    assert_eq!(
        core.revoke_derived_capability(owner, non_delegating, derived),
        Err(KernelError::OperationDenied)
    );

    let unrelated_source = core
        .grant_capability(
            owner,
            core.capability(source).unwrap().resource,
            OperationSet::only(Operation::Delegate),
        )
        .expect("unrelated source should fit");
    assert_eq!(
        core.revoke_derived_capability(owner, unrelated_source, derived),
        Err(KernelError::CapabilityScopeMismatch)
    );
    assert!(!core.capability(derived).unwrap().revoked);
    assert_eq!(core.events().len(), events_before + 2);
    assert_eq!(core.capability(derived).unwrap().agent, target);
}

#[test]
fn derived_revocation_rejects_already_revoked_target_without_second_event() {
    let (mut core, owner, _target, _resource, source, derived) = prepared_capabilities();
    core.revoke_capability(derived)
        .expect("trusted kernel revocation should succeed");
    let events_before = core.events().len();

    assert_eq!(
        core.revoke_derived_capability(owner, source, derived),
        Err(KernelError::CapabilityRevoked)
    );
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn source_owner_can_revoke_authority_from_a_suspended_target() {
    let (mut core, owner, target, _resource, source, derived) = prepared_capabilities();
    core.suspend_agent(target)
        .expect("target should suspend before authority is withdrawn");

    core.revoke_derived_capability(owner, source, derived)
        .expect("inactive recipient must not block source revocation");

    assert!(core.capability(derived).unwrap().revoked);
    assert_eq!(
        core.events().last().unwrap().kind,
        EventKind::CapabilityRevoked
    );
}

#[test]
fn task_scoped_capability_cannot_revoke_generic_authority() {
    let mut core = KernelCore::<2, 1, 3, 12, 0, 0, 0, 1, 1, 0>::new();
    let owner = AgentId::new(1);
    let worker = AgentId::new(2);
    core.register_agent(owner).unwrap();
    core.register_agent(worker).unwrap();
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .unwrap();
    let source = core
        .grant_capability(
            owner,
            resource,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Delegate),
        )
        .unwrap();
    let intent = core
        .declare_intent(
            owner,
            source,
            resource,
            IntentKind::Act,
            VerificationRequirement::Optional,
        )
        .unwrap();
    let task = core.create_task(owner, source, intent).unwrap();
    let task_capability = core
        .delegate_task(owner, source, task, worker)
        .unwrap()
        .capability
        .unwrap();
    let events_before = core.events().len();

    assert_eq!(
        core.revoke_derived_capability(worker, task_capability, source),
        Err(KernelError::CapabilityScopeMismatch)
    );
    assert_eq!(core.events().len(), events_before);
    assert!(!core.capability(source).unwrap().revoked);
}

#[test]
fn derived_revocation_event_capacity_failure_is_atomic() {
    let mut core = KernelCore::<2, 1, 2, 4, 0, 1, 0, 0, 0, 0>::new();
    let owner = AgentId::new(1);
    let target = AgentId::new(2);
    core.register_agent(owner).unwrap();
    core.register_agent(target).unwrap();
    let resource = core.register_resource(ResourceKind::Service, None).unwrap();
    let source = core
        .grant_capability(
            owner,
            resource,
            OperationSet::empty()
                .with(Operation::Observe)
                .with(Operation::Delegate),
        )
        .unwrap();
    let derived = core
        .derive_capability(
            owner,
            source,
            target,
            OperationSet::only(Operation::Observe),
        )
        .unwrap();

    assert_eq!(
        core.revoke_derived_capability(owner, source, derived),
        Err(KernelError::EventLogFull)
    );
    assert!(!core.capability(derived).unwrap().revoked);
    assert_eq!(core.events().len(), 4);
}

fn prepared_capabilities() -> (
    TestCore,
    AgentId,
    AgentId,
    agent_kernel_core::ResourceId,
    CapabilityId,
    CapabilityId,
) {
    let mut core = TestCore::new();
    let owner = AgentId::new(1);
    let target = AgentId::new(2);
    core.register_agent(owner).unwrap();
    core.register_agent(target).unwrap();
    core.register_agent(AgentId::new(3)).unwrap();
    let resource = core.register_resource(ResourceKind::Service, None).unwrap();
    let source = core
        .grant_capability(
            owner,
            resource,
            OperationSet::empty()
                .with(Operation::Observe)
                .with(Operation::Delegate),
        )
        .unwrap();
    let derived = core
        .derive_capability(
            owner,
            source,
            target,
            OperationSet::only(Operation::Observe),
        )
        .unwrap();
    (core, owner, target, resource, source, derived)
}
