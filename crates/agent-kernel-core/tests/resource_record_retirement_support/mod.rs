use agent_kernel_core::{
    AgentEntryKind, AgentId, AgentImageDigest, AgentImageKind, KernelCore, Operation, OperationSet,
    ResourceCreateOutcome, ResourceId, ResourceKind,
};

pub type TestCore<const EVENTS: usize> =
    KernelCore<4, 4, 12, EVENTS, 2, 2, 2, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4>;

#[derive(Copy, Clone)]
pub struct Fixture {
    pub actor: AgentId,
    pub root: ResourceId,
    pub authority: agent_kernel_core::CapabilityId,
    pub target: ResourceCreateOutcome,
}

pub fn all_operations() -> OperationSet {
    OperationSet::only(Operation::Observe)
        .with(Operation::Act)
        .with(Operation::Verify)
        .with(Operation::Rollback)
        .with(Operation::Delegate)
}

pub fn setup<const EVENTS: usize>(entry_kind: AgentEntryKind) -> (TestCore<EVENTS>, Fixture) {
    let mut core = TestCore::new();
    let actor = AgentId::new(1);
    core.register_agent(actor).expect("actor registers");
    let root = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("root registers");
    let authority = core
        .grant_capability(actor, root, all_operations())
        .expect("root authority grants");
    let image_kind = match entry_kind {
        AgentEntryKind::Supervisor => AgentImageKind::Supervisor,
        _ => AgentImageKind::Worker,
    };
    let image = core
        .register_agent_image(
            actor,
            authority,
            root,
            image_kind,
            AgentImageDigest::new([0x41; 32]),
            1,
            1,
        )
        .expect("image registers");
    core.verify_agent_image(actor, authority, image)
        .expect("image verifies");
    core.launch_agent(actor, authority, root, image, entry_kind, None)
        .expect("actor launches");
    let target = core
        .create_resource(
            actor,
            ResourceKind::Service,
            Some((root, authority)),
            all_operations(),
        )
        .expect("target creates");
    (
        core,
        Fixture {
            actor,
            root,
            authority,
            target,
        },
    )
}

pub fn prepare_target<const EVENTS: usize>(core: &mut TestCore<EVENTS>, fixture: Fixture) {
    core.retire_resource(
        fixture.actor,
        fixture.target.capability,
        fixture.target.resource,
    )
    .expect("target lifecycle retires");
    core.revoke_capability_for_cleanup(fixture.actor, fixture.authority, fixture.target.capability)
        .expect("target capability revokes for cleanup");
    core.compact_capability(fixture.actor, fixture.authority, fixture.target.capability)
        .expect("target capability compacts");
}
