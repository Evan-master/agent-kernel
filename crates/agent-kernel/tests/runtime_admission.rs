use agent_kernel::AgentKernel;
use agent_kernel_core::{
    AgentEntryKind, AgentId, AgentImageDigest, AgentImageKind, IntentKind, Operation, OperationSet,
    ResourceKind, VerificationRequirement,
};

#[test]
fn facade_task_scoped_launch_admits_delegated_worker() {
    let mut kernel = AgentKernel::<2, 2, 8, 32, 0, 0, 0, 2, 2, 2>::new();
    let owner = AgentId::new(1);
    let worker = AgentId::new(2);
    kernel
        .sys_register_agent(owner)
        .expect("owner should register");
    kernel
        .sys_register_agent(worker)
        .expect("worker should register");
    let resource = kernel
        .sys_register_resource(ResourceKind::Workspace, None)
        .expect("resource should fit");
    let owner_capability = kernel
        .sys_grant(
            owner,
            resource,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Delegate),
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
        .expect("intent should declare");
    let task = kernel
        .sys_create_task(owner, owner_capability, intent)
        .expect("task should create");
    kernel
        .sys_delegate_task(owner, owner_capability, task, worker)
        .expect("task should delegate");
    let worker_capability = kernel.tasks()[0]
        .delegated_capability
        .expect("delegation should derive worker capability");
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
        .sys_launch_task_agent(
            worker,
            worker_capability,
            task,
            image,
            AgentEntryKind::Worker,
        )
        .expect("worker should launch for task");
    kernel
        .sys_accept_task(worker, task)
        .expect("worker should accept task");
    kernel
        .sys_enqueue_task(worker, task)
        .expect("worker should enqueue task");
    let dispatched = kernel
        .sys_dispatch_next(worker)
        .expect("worker should dispatch task");

    assert_eq!(dispatched, task);
    assert_eq!(kernel.agent_entry(worker).unwrap().task, Some(task));
}
