use agent_kernel::AgentKernel;
use agent_kernel_core::{
    AgentEntryKind, AgentExecutionState, AgentId, IntentKind, Operation, OperationSet,
    ResourceKind, TaskStatus, VerificationRequirement,
};

type TestKernel = AgentKernel<2, 1, 4, 24, 0, 0, 0, 2, 2, 2>;

#[test]
fn execution_contexts_expose_dispatch_and_completion_state() {
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
    let capability = kernel
        .sys_grant(
            owner,
            resource,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Delegate),
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
        .expect("intent should fit");
    let task = kernel
        .sys_create_task(owner, capability, intent)
        .expect("task should fit");
    kernel
        .sys_delegate_task(owner, capability, task, assignee)
        .expect("task should delegate");
    let assignee_capability = kernel.tasks()[0]
        .delegated_capability
        .expect("delegation should derive capability");
    kernel
        .sys_launch_task_agent(assignee, assignee_capability, task, AgentEntryKind::Worker)
        .expect("assignee should launch for delegated task");
    kernel
        .sys_accept_task(assignee, task)
        .expect("task should be accepted");

    kernel
        .sys_enqueue_task(assignee, task)
        .expect("task should enqueue");
    kernel
        .sys_dispatch_next_with_quantum(assignee, 3)
        .expect("task should dispatch");
    let context = kernel.execution_contexts()[1];
    assert_eq!(context.agent, assignee);
    assert_eq!(context.state, AgentExecutionState::Running);
    assert_eq!(context.task, Some(task));
    assert_eq!(context.quantum_remaining, 3);

    kernel
        .sys_complete_task(assignee, assignee_capability, task)
        .expect("running task should complete");
    assert_eq!(
        kernel.execution_contexts()[1].state,
        AgentExecutionState::Idle
    );
    assert_eq!(kernel.execution_contexts()[1].task, None);
    assert_eq!(kernel.tasks()[0].status, TaskStatus::Completed);
}
