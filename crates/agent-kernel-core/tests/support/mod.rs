use agent_kernel_core::{
    AgentEntryKind, AgentId, AgentImageDigest, AgentImageKind, CapabilityId, IntentKind,
    KernelCore, Operation, OperationSet, ResourceId, ResourceKind, TaskId, VerificationRequirement,
};

pub type MailboxWaitCore<
    const EVENTS: usize,
    const RUN_QUEUE: usize,
    const MESSAGES: usize,
    const WAITERS: usize,
> = KernelCore<3, 1, 3, EVENTS, 0, EVENTS, 0, 2, 2, RUN_QUEUE, MESSAGES, 0, 0, 0, 0, 0, WAITERS, 2>;

#[derive(Copy, Clone)]
#[allow(dead_code)]
pub struct MailboxWaitFlow {
    pub owner: AgentId,
    pub sender: AgentId,
    pub recipient: AgentId,
    pub owner_capability: CapabilityId,
    pub sender_capability: CapabilityId,
    pub recipient_capability: CapabilityId,
    pub resource: ResourceId,
    pub sender_task: TaskId,
    pub recipient_task: TaskId,
}

pub fn running_recipient<
    const EVENTS: usize,
    const RUN_QUEUE: usize,
    const MESSAGES: usize,
    const WAITERS: usize,
>(
    core: &mut MailboxWaitCore<EVENTS, RUN_QUEUE, MESSAGES, WAITERS>,
) -> MailboxWaitFlow {
    let owner = AgentId::new(1);
    let sender = AgentId::new(2);
    let recipient = AgentId::new(3);
    core.register_agent(owner).expect("owner should register");
    core.register_agent(sender).expect("sender should register");
    core.register_agent(recipient)
        .expect("recipient should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let owner_capability = core
        .grant_capability(
            owner,
            resource,
            OperationSet::empty()
                .with(Operation::Observe)
                .with(Operation::Act)
                .with(Operation::Delegate)
                .with(Operation::Verify),
        )
        .expect("owner capability should fit");
    let (recipient_task, recipient_capability) =
        delegated_task(core, owner, owner_capability, resource, recipient, 3);
    let (sender_task, sender_capability) =
        delegated_task(core, owner, owner_capability, resource, sender, 2);
    core.enqueue_task(recipient, recipient_task)
        .expect("recipient should enqueue");
    core.dispatch_next_with_quantum(recipient, 1)
        .expect("recipient should dispatch");
    core.enqueue_task(sender, sender_task)
        .expect("sender should remain queued");
    MailboxWaitFlow {
        owner,
        sender,
        recipient,
        owner_capability,
        sender_capability,
        recipient_capability,
        resource,
        sender_task,
        recipient_task,
    }
}

fn delegated_task<
    const EVENTS: usize,
    const RUN_QUEUE: usize,
    const MESSAGES: usize,
    const WAITERS: usize,
>(
    core: &mut MailboxWaitCore<EVENTS, RUN_QUEUE, MESSAGES, WAITERS>,
    owner: AgentId,
    owner_capability: CapabilityId,
    resource: ResourceId,
    assignee: AgentId,
    digest_byte: u8,
) -> (TaskId, CapabilityId) {
    let intent = core
        .declare_intent(
            owner,
            owner_capability,
            resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .expect("intent should declare");
    let task = core
        .create_task(owner, owner_capability, intent)
        .expect("task should create");
    core.delegate_task(owner, owner_capability, task, assignee)
        .expect("task should delegate");
    let capability = core
        .tasks()
        .iter()
        .find(|record| record.id == task)
        .and_then(|record| record.delegated_capability)
        .expect("delegation should derive capability");
    let image = core
        .register_agent_image(
            owner,
            owner_capability,
            resource,
            AgentImageKind::Worker,
            AgentImageDigest::new([digest_byte; 32]),
            1,
            1,
        )
        .expect("image should register");
    core.verify_agent_image(owner, owner_capability, image)
        .expect("image should verify");
    core.launch_task_agent(assignee, capability, task, image, AgentEntryKind::Worker)
        .expect("task agent should launch");
    core.accept_task(assignee, task)
        .expect("task should accept");
    (task, capability)
}
