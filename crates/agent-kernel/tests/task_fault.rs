use agent_kernel::AgentKernel;
use agent_kernel_core::{
    AgentEntryKind, AgentId, EventKind, FaultId, FaultKind, IntentKind, Operation, OperationSet,
    ResourceKind, TaskStatus, VerificationRequirement,
};

type TestKernel = AgentKernel<2, 1, 2, 24, 0, 0, 0, 1, 1, 1, 0, 0, 0, 1>;

#[test]
fn task_fault_syscalls_fault_recover_and_allow_completion() {
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
                .with(Operation::Rollback),
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
        .expect("delegation should derive task capability");
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
        .sys_dispatch_next_with_quantum(assignee, 2)
        .expect("task should dispatch");

    let fault = kernel
        .sys_fault_task(assignee, task, FaultKind::ExecutionTrap, 7)
        .expect("running task should fault");
    let recovered = kernel
        .sys_recover_faulted_task(owner, owner_capability, task)
        .expect("rollback authority should recover faulted task");
    kernel
        .sys_enqueue_task(assignee, task)
        .expect("recovered task should enqueue");
    kernel
        .sys_dispatch_next_with_quantum(assignee, 1)
        .expect("recovered task should redispatch");
    kernel
        .sys_complete_task(assignee, assignee_capability, task)
        .expect("recovered task should complete");

    assert_eq!(fault, FaultId::new(1));
    assert_eq!(recovered.kind, EventKind::TaskFaultRecovered);
    assert_eq!(kernel.faults()[0].kind, FaultKind::ExecutionTrap);
    assert_eq!(kernel.faults()[0].detail, 7);
    assert_eq!(kernel.tasks()[0].status, TaskStatus::Completed);
    assert_eq!(kernel.tasks()[0].last_fault, Some(fault));
}
