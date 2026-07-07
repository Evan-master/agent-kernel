use agent_kernel::AgentKernel;
use agent_kernel_core::{
    AgentEntryKind, AgentId, AgentImageDigest, AgentImageKind, EventKind, IntentKind, Operation,
    OperationSet, ResourceKind, RunQueueEntry, TaskStatus, VerificationRequirement,
};

type TestKernel = AgentKernel<2, 1, 2, 20, 0, 0, 0, 1, 1, 1>;

#[test]
fn scheduler_quantum_syscalls_dispatch_tick_and_requeue() {
    let mut kernel = TestKernel::new();
    let owner = AgentId::new(1);
    let assignee = AgentId::new(2);
    kernel
        .sys_register_agent(owner)
        .expect("owner should register");
    kernel
        .sys_register_agent(assignee)
        .expect("assignee should register");
    let resource = kernel
        .sys_register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let owner_capability = kernel
        .sys_grant(
            owner,
            resource,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Delegate)
                .with(Operation::Verify),
        )
        .expect("owner capability should fit");
    let intent = kernel
        .sys_declare_intent(
            owner,
            owner_capability,
            resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .expect("intent should be declared");
    let task = kernel
        .sys_create_task(owner, owner_capability, intent)
        .expect("task should be created");
    kernel
        .sys_delegate_task(owner, owner_capability, task, assignee)
        .expect("task should be delegated");
    let assignee_capability = kernel.tasks()[0]
        .delegated_capability
        .expect("delegation should derive capability");
    let image = kernel
        .sys_register_agent_image(
            owner,
            owner_capability,
            resource,
            AgentImageKind::Worker,
            AgentImageDigest::new([1; 32]),
            1,
            1,
        )
        .expect("worker image should register");
    kernel
        .sys_verify_agent_image(owner, owner_capability, image)
        .expect("image should verify");
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
        .expect("task should be accepted");
    kernel
        .sys_enqueue_task(assignee, task)
        .expect("task should enqueue");

    kernel
        .sys_dispatch_next_with_quantum(assignee, 2)
        .expect("task should dispatch with explicit quantum");
    let tick = kernel
        .sys_tick_task(assignee, task)
        .expect("task should tick once");
    let expired = kernel
        .sys_tick_task(assignee, task)
        .expect("second tick should expire quantum");

    assert_eq!(tick.kind, EventKind::TaskTicked);
    assert_eq!(tick.task_ticks, Some(1));
    assert_eq!(tick.task_quantum, Some(1));
    assert_eq!(expired.kind, EventKind::TaskQuantumExpired);
    assert_eq!(kernel.tasks()[0].status, TaskStatus::Accepted);
    assert_eq!(kernel.tasks()[0].run_ticks, 2);
    assert_eq!(kernel.tasks()[0].quantum_remaining, 0);
    assert_eq!(
        kernel.run_queue(),
        &[RunQueueEntry {
            task,
            agent: assignee,
        }]
    );
}
