use agent_kernel_core::{
    AgentId, CapabilityId, KernelCore, NamespaceEntryId, NamespaceKey, NamespaceObject, Operation,
    OperationSet, ResourceId, ResourceKind,
};

pub type TestCore<const EVENTS: usize> = KernelCore<2, 3, 5, EVENTS, 0, 0, 0, 0, 0, 0, 0, 0, 2>;

#[derive(Copy, Clone)]
pub struct Fixture {
    pub actor: AgentId,
    pub workspace: ResourceId,
    pub authority: CapabilityId,
    pub target: NamespaceEntryId,
    pub retained: NamespaceEntryId,
}

pub fn setup<const EVENTS: usize>() -> (TestCore<EVENTS>, Fixture) {
    let mut core = TestCore::new();
    let actor = AgentId::new(1);
    core.register_agent(actor).expect("actor registers");
    let workspace = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("workspace registers");
    let authority = core
        .grant_capability(
            actor,
            workspace,
            OperationSet::only(Operation::Observe)
                .with(Operation::Act)
                .with(Operation::Rollback),
        )
        .expect("namespace authority grants");
    let target = core
        .bind_namespace_entry(
            actor,
            authority,
            workspace,
            NamespaceKey::new(11),
            NamespaceObject::Resource(workspace),
        )
        .expect("target entry binds");
    let retained = core
        .bind_namespace_entry(
            actor,
            authority,
            workspace,
            NamespaceKey::new(12),
            NamespaceObject::Agent(actor),
        )
        .expect("retained entry binds");

    (
        core,
        Fixture {
            actor,
            workspace,
            authority,
            target,
            retained,
        },
    )
}
