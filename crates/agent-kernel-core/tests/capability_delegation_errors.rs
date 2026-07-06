use agent_kernel_core::{
    AgentId, CapabilityId, EventKind, IntentKind, KernelCore, KernelError, Operation, OperationSet,
    ResourceKind, TaskStatus, VerificationRequirement,
};

#[test]
fn derive_capability_requires_delegate_authority_without_allocation() {
    let mut core = KernelCore::<2, 1, 2, 4, 0, 1, 0, 0, 0, 0>::new();
    let owner = AgentId::new(1);
    let target = AgentId::new(2);
    core.register_agent(owner).expect("owner should register");
    core.register_agent(target).expect("target should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let source = core
        .grant_capability(owner, resource, OperationSet::only(Operation::Observe))
        .expect("source capability should fit");
    let events_before = core.events().len();

    assert_eq!(
        core.derive_capability(
            owner,
            source,
            target,
            OperationSet::only(Operation::Observe)
        ),
        Err(KernelError::OperationDenied)
    );
    assert_eq!(core.events().len(), events_before);
    assert_eq!(
        core.observe(target, CapabilityId::new(2), resource),
        Err(KernelError::CapabilityNotFound)
    );
}

#[test]
fn derive_capability_cannot_expand_source_operations() {
    let mut core = KernelCore::<2, 1, 2, 4, 1, 0, 0, 0, 0, 0>::new();
    let owner = AgentId::new(3);
    let target = AgentId::new(4);
    core.register_agent(owner).expect("owner should register");
    core.register_agent(target).expect("target should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let source = core
        .grant_capability(
            owner,
            resource,
            OperationSet::empty()
                .with(Operation::Observe)
                .with(Operation::Delegate),
        )
        .expect("source capability should fit");
    let events_before = core.events().len();

    assert_eq!(
        core.derive_capability(owner, source, target, OperationSet::only(Operation::Act)),
        Err(KernelError::OperationDenied)
    );
    assert_eq!(core.events().len(), events_before);
    assert_eq!(
        core.act(
            target,
            CapabilityId::new(2),
            agent_kernel_core::ActionId::new(1),
            resource
        ),
        Err(KernelError::CapabilityNotFound)
    );
}

#[test]
fn derive_capability_rejects_task_scoped_source_without_root_authority() {
    let mut core = KernelCore::<2, 1, 2, 8, 0, 0, 0, 1, 1, 0>::new();
    let owner = AgentId::new(5);
    let target = AgentId::new(6);
    core.register_agent(owner).expect("owner should register");
    core.register_agent(target).expect("target should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let source = core
        .grant_capability(
            owner,
            resource,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Delegate),
        )
        .expect("source capability should fit");
    let intent = core
        .declare_intent(
            owner,
            source,
            resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .expect("intent should declare");
    let task = core
        .create_task(owner, source, intent)
        .expect("task should fit");
    let delegation = core
        .delegate_task(owner, source, task, target)
        .expect("task should delegate");
    let task_capability = delegation
        .capability
        .expect("task delegation should derive a capability");
    let events_before = core.events().len();

    assert_eq!(
        core.derive_capability(
            target,
            task_capability,
            owner,
            OperationSet::only(Operation::Act)
        ),
        Err(KernelError::CapabilityScopeMismatch)
    );
    assert_eq!(core.tasks()[0].status, TaskStatus::Delegated);
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn derive_capability_rejects_inactive_target_without_allocation() {
    let mut core = KernelCore::<1, 1, 2, 3, 0, 0, 0, 0, 0, 0>::new();
    let owner = AgentId::new(7);
    let missing_target = AgentId::new(8);
    core.register_agent(owner).expect("owner should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let source = core
        .grant_capability(
            owner,
            resource,
            OperationSet::empty()
                .with(Operation::Observe)
                .with(Operation::Delegate),
        )
        .expect("source capability should fit");
    let events_before = core.events().len();

    assert_eq!(
        core.derive_capability(
            owner,
            source,
            missing_target,
            OperationSet::only(Operation::Observe)
        ),
        Err(KernelError::AgentNotFound)
    );
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn derive_capability_event_log_full_leaves_no_derived_capability() {
    let mut core = KernelCore::<2, 1, 2, 3, 0, 1, 0, 0, 0, 0>::new();
    let owner = AgentId::new(9);
    let target = AgentId::new(10);
    core.register_agent(owner).expect("owner should register");
    core.register_agent(target).expect("target should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let source = core
        .grant_capability(
            owner,
            resource,
            OperationSet::empty()
                .with(Operation::Observe)
                .with(Operation::Delegate),
        )
        .expect("source capability should fit");

    assert_eq!(
        core.derive_capability(
            owner,
            source,
            target,
            OperationSet::only(Operation::Observe)
        ),
        Err(KernelError::EventLogFull)
    );
    assert_eq!(core.events().len(), 3);
    assert_eq!(
        core.events().last().unwrap().kind,
        EventKind::CapabilityGranted
    );
    assert_eq!(
        core.observe(target, CapabilityId::new(2), resource),
        Err(KernelError::CapabilityNotFound)
    );
}

#[test]
fn source_revocation_invalidates_derived_root_capability() {
    let mut core = KernelCore::<2, 1, 2, 5, 0, 1, 0, 0, 0, 0>::new();
    let owner = AgentId::new(11);
    let target = AgentId::new(12);
    core.register_agent(owner).expect("owner should register");
    core.register_agent(target).expect("target should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let source = core
        .grant_capability(
            owner,
            resource,
            OperationSet::empty()
                .with(Operation::Observe)
                .with(Operation::Delegate),
        )
        .expect("source capability should fit");
    let derived = core
        .derive_capability(
            owner,
            source,
            target,
            OperationSet::only(Operation::Observe),
        )
        .expect("owner should derive observe authority");

    core.revoke_capability(source)
        .expect("source capability should revoke");
    let events_before = core.events().len();

    assert_eq!(
        core.observe(target, derived, resource),
        Err(KernelError::CapabilityRevoked)
    );
    assert!(core.observations().is_empty());
    assert_eq!(core.events().len(), events_before);
}
