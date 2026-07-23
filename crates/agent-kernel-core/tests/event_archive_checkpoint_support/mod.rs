mod complete_event;

use agent_kernel_core::{
    AgentEntryKind, AgentId, AgentImageDigest, AgentImageKind, CapabilityId, DurableArchiveAnchor,
    DurableArchiveReceipt, DurableArchiveVerificationError, DurableArchiveVerificationRequest,
    DurableArchiveVerifier, DurableSlot, DurableStateDigest, Event, EventArchiveCheckpoint,
    EventArchiveProposal, IntentKind, KernelCore, KernelError, Operation, OperationSet, ResourceId,
    ResourceKind, SignalKey, VerificationRequirement,
};

pub use complete_event::complete_event;

pub type TestCore<const EVENTS: usize> =
    KernelCore<1, 2, 8, EVENTS, 0, 0, 0, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 1>;

#[derive(Copy, Clone)]
pub struct Fixture {
    pub actor: AgentId,
    pub root: ResourceId,
    pub authority: CapabilityId,
}

pub const fn all_operations() -> OperationSet {
    OperationSet::only(Operation::Observe)
        .with(Operation::Act)
        .with(Operation::Verify)
        .with(Operation::Checkpoint)
        .with(Operation::Rollback)
        .with(Operation::Delegate)
}

pub fn fixture<const EVENTS: usize>(kind: AgentEntryKind) -> (TestCore<EVENTS>, Fixture) {
    let mut core = TestCore::new();
    let actor = AgentId::new(1);
    core.register_agent(actor).unwrap();
    let root = core
        .register_resource(ResourceKind::Workspace, None)
        .unwrap();
    let authority = core
        .grant_capability(actor, root, all_operations())
        .unwrap();
    let intent = core
        .declare_intent(
            actor,
            authority,
            root,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .unwrap();
    let task = core.create_task(actor, authority, intent).unwrap();
    core.delegate_task(actor, authority, task, actor).unwrap();
    let task_authority = core.tasks()[0].delegated_capability.unwrap();
    let image_kind = match kind {
        AgentEntryKind::Bootstrap => AgentImageKind::Bootstrap,
        AgentEntryKind::Supervisor => AgentImageKind::Supervisor,
        AgentEntryKind::Worker => AgentImageKind::Worker,
        AgentEntryKind::Verifier => AgentImageKind::Verifier,
        AgentEntryKind::FaultHandler => AgentImageKind::FaultHandler,
        AgentEntryKind::Driver => AgentImageKind::Driver,
    };
    let image = core
        .register_agent_image(
            actor,
            authority,
            root,
            image_kind,
            AgentImageDigest::new([0x40; 32]),
            1,
            1,
        )
        .unwrap();
    core.verify_agent_image(actor, authority, image).unwrap();
    core.launch_task_agent(actor, task_authority, task, image, kind)
        .unwrap();
    core.accept_task(actor, task).unwrap();
    (
        core,
        Fixture {
            actor,
            root,
            authority,
        },
    )
}

pub fn emit<const EVENTS: usize>(core: &mut TestCore<EVENTS>, fixture: Fixture, raw: u64) -> Event {
    core.emit_signal(
        fixture.actor,
        fixture.authority,
        fixture.root,
        SignalKey::new(raw),
    )
    .unwrap()
    .signal_event
}

pub fn commit<const EVENTS: usize>(
    core: &mut TestCore<EVENTS>,
    fixture: Fixture,
    proposal: EventArchiveProposal,
) -> Result<EventArchiveCheckpoint, KernelError> {
    commit_with_archive_authority(core, fixture, fixture.authority, proposal)
}

pub fn commit_with_archive_authority<const EVENTS: usize>(
    core: &mut TestCore<EVENTS>,
    fixture: Fixture,
    archive_authority: CapabilityId,
    proposal: EventArchiveProposal,
) -> Result<EventArchiveCheckpoint, KernelError> {
    let seed = proposal.generation() as u8;
    let receipt = DurableArchiveReceipt::new(
        DurableSlot::for_generation(proposal.generation()).unwrap(),
        fixture.root,
        proposal.generation(),
        proposal.digest(),
        DurableStateDigest::new([seed; 32]),
        DurableStateDigest::new([seed.wrapping_add(1); 32]),
        proposal.generation(),
        DurableArchiveAnchor::unanchored(),
    )
    .unwrap();
    core.commit_durable_event_archive(
        fixture.actor,
        archive_authority,
        fixture.authority,
        proposal,
        receipt,
        &mut AcceptDurableArchiveVerifier,
    )
}

struct AcceptDurableArchiveVerifier;

impl DurableArchiveVerifier for AcceptDurableArchiveVerifier {
    fn verify(
        &mut self,
        _request: DurableArchiveVerificationRequest,
    ) -> Result<(), DurableArchiveVerificationError> {
        Ok(())
    }
}
