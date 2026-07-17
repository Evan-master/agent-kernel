use agent_kernel::AgentKernel;
use agent_kernel_core::{AgentId, EventKind, KernelError, Operation, OperationSet, ResourceKind};

type TestKernel = AgentKernel<2, 1, 2, 5, 0, 1, 0, 0, 0, 0>;

#[test]
fn sys_derive_capability_allows_target_to_use_subset_authority() {
    let mut kernel = TestKernel::new();
    let owner = AgentId::new(1);
    let target = AgentId::new(2);
    kernel
        .sys_register_agent(owner)
        .expect("owner should register");
    kernel
        .sys_register_agent(target)
        .expect("target should register");
    let resource = kernel
        .sys_register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let source = kernel
        .sys_grant(
            owner,
            resource,
            OperationSet::empty()
                .with(Operation::Observe)
                .with(Operation::Delegate),
        )
        .expect("source capability should fit");

    let derived = kernel
        .sys_derive_capability(
            owner,
            source,
            target,
            OperationSet::only(Operation::Observe),
        )
        .expect("owner should derive observe authority");
    let event = kernel
        .sys_observe(target, derived, resource)
        .expect("target should observe through derived authority");

    assert_eq!(kernel.events()[3].kind, EventKind::CapabilityDerived);
    assert_eq!(kernel.events()[3].source_capability, Some(source));
    assert_eq!(kernel.events()[3].target_agent, Some(target));
    assert_eq!(event.kind, EventKind::Observation);
}

#[test]
fn sys_revoke_derived_capability_blocks_target_through_public_facade() {
    let mut kernel = TestKernel::new();
    let owner = AgentId::new(1);
    let target = AgentId::new(2);
    kernel.sys_register_agent(owner).unwrap();
    kernel.sys_register_agent(target).unwrap();
    let resource = kernel
        .sys_register_resource(ResourceKind::Service, None)
        .unwrap();
    let source = kernel
        .sys_grant(
            owner,
            resource,
            OperationSet::empty()
                .with(Operation::Observe)
                .with(Operation::Delegate),
        )
        .unwrap();
    let derived = kernel
        .sys_derive_capability(
            owner,
            source,
            target,
            OperationSet::only(Operation::Observe),
        )
        .unwrap();

    let event = kernel
        .sys_revoke_derived_capability(owner, source, derived)
        .expect("facade should expose authenticated child revocation");

    assert_eq!(event.kind, EventKind::CapabilityRevoked);
    assert_eq!(event.source_capability, Some(source));
    assert_eq!(event.target_agent, Some(target));
    assert_eq!(
        kernel.sys_observe(target, derived, resource),
        Err(KernelError::CapabilityRevoked)
    );
}
