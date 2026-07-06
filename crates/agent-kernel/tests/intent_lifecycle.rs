use agent_kernel::AgentKernel;
use agent_kernel_core::{
    AgentEntryKind, AgentId, AgentImageDigest, AgentImageKind, EventKind, IntentId, IntentKind,
    IntentStatus, Operation, OperationSet, ResourceKind, TaskId, VerificationRequirement,
};

type TestKernel = AgentKernel<2, 4, 4, 16, 4, 4, 4, 4, 4, 4>;

#[test]
fn sys_declare_intent_records_and_exposes_intent() {
    let mut kernel = TestKernel::new();
    let agent = AgentId::new(1);
    kernel
        .sys_register_agent(agent)
        .expect("agent should register");
    let resource = kernel
        .sys_register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = kernel
        .sys_grant(agent, resource, OperationSet::only(Operation::Act))
        .expect("capability should fit");

    let intent = kernel
        .sys_declare_intent(
            agent,
            capability,
            resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .expect("intent should be declared");

    assert_eq!(intent, IntentId::new(1));
    assert_eq!(kernel.intents().len(), 1);
    assert_eq!(kernel.intents()[0].id, intent);
    assert_eq!(kernel.intents()[0].owner, agent);
    assert_eq!(kernel.intents()[0].resource, resource);
    assert_eq!(kernel.intents()[0].kind, IntentKind::Act);
    assert_eq!(kernel.intents()[0].status, IntentStatus::Declared);
    assert_eq!(
        kernel.intents()[0].verification,
        VerificationRequirement::Required
    );
    assert_eq!(kernel.events()[2].kind, EventKind::IntentDeclared);
    assert_eq!(kernel.events()[2].intent, Some(intent));
}

#[test]
fn sys_create_task_accepts_intent_id() {
    let mut kernel = TestKernel::new();
    let agent = AgentId::new(2);
    kernel
        .sys_register_agent(agent)
        .expect("agent should register");
    let resource = kernel
        .sys_register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = kernel
        .sys_grant(agent, resource, OperationSet::only(Operation::Act))
        .expect("capability should fit");
    let intent = kernel
        .sys_declare_intent(
            agent,
            capability,
            resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .expect("intent should be declared");

    let task = kernel
        .sys_create_task(agent, capability, intent)
        .expect("task should be created");

    assert_eq!(task, TaskId::new(1));
    assert_eq!(kernel.tasks()[0].intent, intent);
    assert_eq!(kernel.intents()[0].status, IntentStatus::Bound);
    assert_eq!(kernel.events()[3].kind, EventKind::TaskCreated);
    assert_eq!(kernel.events()[3].intent, Some(intent));
    assert_eq!(kernel.events()[4].kind, EventKind::IntentBound);
    assert_eq!(kernel.events()[4].intent, Some(intent));
    assert_eq!(kernel.events()[4].task, Some(task));
}

#[test]
fn sys_verify_task_exposes_fulfilled_intent_status() {
    let mut kernel = TestKernel::new();
    let owner = AgentId::new(3);
    let assignee = AgentId::new(4);
    kernel
        .sys_register_agent(owner)
        .expect("owner should register");
    kernel
        .sys_register_agent(assignee)
        .expect("assignee should register");
    let resource = kernel
        .sys_register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = kernel
        .sys_grant(
            owner,
            resource,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Delegate)
                .with(Operation::Verify),
        )
        .expect("capability should fit");
    let intent = kernel
        .sys_declare_intent(
            owner,
            capability,
            resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .expect("intent should be declared");
    let task = kernel
        .sys_create_task(owner, capability, intent)
        .expect("task should be created");
    kernel
        .sys_delegate_task(owner, capability, task, assignee)
        .expect("task should delegate");
    let assignee_capability = kernel.tasks()[0]
        .delegated_capability
        .expect("delegation should derive capability");
    let image = kernel
        .sys_register_agent_image(
            owner,
            capability,
            resource,
            AgentImageKind::Worker,
            AgentImageDigest::new([1; 32]),
            1,
            1,
        )
        .expect("worker image should register");
    kernel
        .sys_launch_task_agent(
            assignee,
            assignee_capability,
            task,
            image,
            AgentEntryKind::Worker,
        )
        .expect("assignee should launch for delegated task");
    kernel
        .sys_accept_task(assignee, task)
        .expect("task should accept");
    kernel
        .sys_enqueue_task(assignee, task)
        .expect("task should enqueue");
    kernel
        .sys_dispatch_next(assignee)
        .expect("task should dispatch");
    kernel
        .sys_complete_task(assignee, assignee_capability, task)
        .expect("task should complete");

    kernel
        .sys_verify_task(owner, capability, task)
        .expect("task should verify");

    assert_eq!(kernel.intents()[0].status, IntentStatus::Fulfilled);
    assert_eq!(
        kernel
            .events()
            .last()
            .expect("intent event should exist")
            .kind,
        EventKind::IntentFulfilled
    );
}

#[test]
fn sys_cancel_task_exposes_cancelled_intent_status() {
    let mut kernel = TestKernel::new();
    let owner = AgentId::new(5);
    kernel
        .sys_register_agent(owner)
        .expect("owner should register");
    let resource = kernel
        .sys_register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let capability = kernel
        .sys_grant(
            owner,
            resource,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Rollback),
        )
        .expect("capability should fit");
    let intent = kernel
        .sys_declare_intent(
            owner,
            capability,
            resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .expect("intent should be declared");
    let task = kernel
        .sys_create_task(owner, capability, intent)
        .expect("task should be created");

    kernel
        .sys_cancel_task(owner, capability, task)
        .expect("task should cancel");

    assert_eq!(kernel.intents()[0].status, IntentStatus::Cancelled);
    assert_eq!(
        kernel
            .events()
            .last()
            .expect("intent event should exist")
            .kind,
        EventKind::IntentCancelled
    );
}
