use agent_kernel_core::{AgentId, EventKind, KernelCore, Operation, OperationSet, ResourceKind};

#[test]
fn derive_capability_records_event_and_target_can_use_subset_authority() {
    let mut core = KernelCore::<2, 1, 2, 5, 0, 1, 0, 0, 0, 0>::new();
    let owner = AgentId::new(1);
    let target = AgentId::new(2);
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

    let event = core.events()[3];
    assert_eq!(event.kind, EventKind::CapabilityDerived);
    assert_eq!(event.agent, owner);
    assert_eq!(event.target_agent, Some(target));
    assert_eq!(event.resource, Some(resource));
    assert_eq!(event.capability, Some(derived));
    assert_eq!(event.source_capability, Some(source));
    assert_eq!(event.operations, OperationSet::only(Operation::Observe));
    assert_eq!(event.task, None);

    core.observe(target, derived, resource)
        .expect("target should use derived observe authority");
    assert_eq!(core.events()[4].kind, EventKind::Observation);
}
