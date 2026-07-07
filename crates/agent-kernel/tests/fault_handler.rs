use agent_kernel::AgentKernel;
use agent_kernel_core::{
    AgentEntryKind, AgentId, AgentImageDigest, AgentImageKind, EventKind, FaultHandlerId,
    FaultKind, IntentKind, MessageId, MessageKind, MessageStatus, Operation, OperationSet,
    ResourceKind, TaskStatus, VerificationRequirement,
};

type TestKernel = AgentKernel<3, 1, 3, 28, 0, 0, 0, 1, 1, 1, 2, 0, 0, 1, 1>;

#[test]
fn fault_handler_syscalls_route_fault_message_and_allow_recovery() {
    let mut kernel = TestKernel::new();
    let owner = AgentId::new(1);
    let assignee = AgentId::new(2);
    let handler = AgentId::new(3);
    kernel
        .sys_register_agent(owner)
        .expect("owner should register");
    kernel
        .sys_register_agent(assignee)
        .expect("assignee should register");
    kernel
        .sys_register_agent(handler)
        .expect("handler should register");
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
                .with(Operation::Verify)
                .with(Operation::Rollback),
        )
        .expect("owner capability should fit");
    let handler_id = kernel
        .sys_install_fault_handler(
            owner,
            owner_capability,
            resource,
            FaultKind::ExecutionTrap,
            handler,
        )
        .expect("handler should install");
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
        .expect("task should dispatch");
    let fault = kernel
        .sys_fault_task(assignee, task, FaultKind::ExecutionTrap, 7)
        .expect("task should fault");

    let message = kernel
        .sys_route_fault_to_handler(owner, owner_capability, fault)
        .expect("fault should route");
    let received = kernel
        .sys_receive_message(handler)
        .expect("handler should receive routed fault");
    let acknowledged = kernel
        .sys_acknowledge_message(handler, message)
        .expect("handler should acknowledge routed fault");
    kernel
        .sys_recover_faulted_task(owner, owner_capability, task)
        .expect("owner should recover faulted task");
    kernel
        .sys_enqueue_task(assignee, task)
        .expect("recovered task should enqueue");
    kernel
        .sys_dispatch_next_with_quantum(assignee, 1)
        .expect("recovered task should dispatch");
    kernel
        .sys_complete_task(assignee, assignee_capability, task)
        .expect("recovered task should complete");

    assert_eq!(handler_id, FaultHandlerId::new(1));
    assert_eq!(message, MessageId::new(1));
    assert_eq!(received, message);
    assert_eq!(acknowledged.kind, EventKind::MessageAcknowledged);
    assert_eq!(kernel.fault_handlers().len(), 1);
    assert_eq!(kernel.fault_handlers()[0].handler, handler);
    assert_eq!(kernel.messages()[0].kind, MessageKind::Fault);
    assert_eq!(kernel.messages()[0].payload.fault, Some(fault));
    assert_eq!(kernel.messages()[0].status, MessageStatus::Acknowledged);
    assert_eq!(kernel.events()[17].kind, EventKind::MessageSent);
    assert_eq!(kernel.events()[18].kind, EventKind::FaultRouted);
    assert_eq!(kernel.tasks()[0].status, TaskStatus::Completed);
}
