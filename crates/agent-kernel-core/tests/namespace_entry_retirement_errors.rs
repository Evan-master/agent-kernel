mod namespace_entry_retirement_support;

use agent_kernel_core::{
    AgentId, KernelError, NamespaceEntryId, Operation, OperationSet, ResourceKind,
};

use namespace_entry_retirement_support::setup;

#[test]
fn rollback_authority_is_required_without_mutation() {
    let (mut core, fixture) = setup::<32>();
    let observe = core
        .grant_capability(
            fixture.actor,
            fixture.workspace,
            OperationSet::only(Operation::Observe),
        )
        .unwrap();
    let entries = core.namespace_entries().to_vec();
    let events = core.events().len();

    assert_eq!(
        core.retire_namespace_entry(fixture.actor, observe, fixture.target),
        Err(KernelError::OperationDenied)
    );
    assert_eq!(core.namespace_entries(), entries.as_slice());
    assert_eq!(core.events().len(), events);
}

#[test]
fn authority_must_target_the_entry_workspace() {
    let (mut core, fixture) = setup::<40>();
    let other = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("second workspace registers");
    let foreign = core
        .grant_capability(
            fixture.actor,
            other,
            OperationSet::only(Operation::Rollback),
        )
        .unwrap();

    assert_eq!(
        core.retire_namespace_entry(fixture.actor, foreign, fixture.target),
        Err(KernelError::ResourceMismatch)
    );
    assert_eq!(core.namespace_entries().len(), 2);
    assert_eq!(core.namespace_entries()[1].id, fixture.retained);
}

#[test]
fn inactive_and_missing_callers_fail_before_entry_lookup() {
    let (mut core, fixture) = setup::<32>();
    core.suspend_agent(fixture.actor).unwrap();

    assert_eq!(
        core.retire_namespace_entry(fixture.actor, fixture.authority, NamespaceEntryId::new(99),),
        Err(KernelError::AgentSuspended)
    );
    assert_eq!(
        core.retire_namespace_entry(
            AgentId::new(99),
            fixture.authority,
            NamespaceEntryId::new(99),
        ),
        Err(KernelError::AgentNotFound)
    );
    assert_eq!(core.namespace_entries().len(), 2);
}

#[test]
fn missing_entry_and_event_exhaustion_are_atomic() {
    let (mut core, fixture) = setup::<5>();
    core.resolve_namespace_entry(
        fixture.actor,
        fixture.authority,
        fixture.workspace,
        agent_kernel_core::NamespaceKey::new(11),
    )
    .unwrap();
    assert_eq!(core.events().len(), 5);
    let entries = core.namespace_entries().to_vec();

    assert_eq!(
        core.retire_namespace_entry(fixture.actor, fixture.authority, NamespaceEntryId::new(99),),
        Err(KernelError::NamespaceEntryNotFound)
    );
    assert_eq!(
        core.retire_namespace_entry(fixture.actor, fixture.authority, fixture.target),
        Err(KernelError::EventLogFull)
    );
    assert_eq!(core.namespace_entries(), entries.as_slice());
    assert_eq!(core.events().len(), 5);
}
