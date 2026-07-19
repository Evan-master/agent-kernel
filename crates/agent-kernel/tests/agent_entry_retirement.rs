use agent_kernel::AgentKernel;
use agent_kernel_core::{
    AgentEntryKind, AgentId, AgentImageDigest, AgentImageKind, KernelError, Operation,
    OperationSet, ResourceKind, VerificationRequirement,
};

type TestKernel = AgentKernel<3, 1, 6, 64, 0, 0, 0, 4, 4, 2>;

#[test]
fn facade_forwards_agent_entry_retirement_inspection_and_relaunch() {
    let mut kernel = TestKernel::new();
    let supervisor = AgentId::new(1);
    let worker = AgentId::new(2);
    kernel.sys_register_agent(supervisor).unwrap();
    kernel.sys_register_agent(worker).unwrap();
    let resource = kernel
        .sys_register_resource(ResourceKind::Workspace, None)
        .unwrap();
    let authority = kernel
        .sys_grant(
            supervisor,
            resource,
            OperationSet::only(Operation::Act)
                .with(Operation::Verify)
                .with(Operation::Rollback)
                .with(Operation::Delegate),
        )
        .unwrap();
    let supervisor_image = kernel
        .sys_register_agent_image(
            supervisor,
            authority,
            resource,
            AgentImageKind::Supervisor,
            AgentImageDigest::new([1; 32]),
            1,
            1,
        )
        .unwrap();
    kernel
        .sys_verify_agent_image(supervisor, authority, supervisor_image)
        .unwrap();
    kernel
        .sys_launch_agent(
            supervisor,
            authority,
            resource,
            supervisor_image,
            AgentEntryKind::Supervisor,
            None,
        )
        .unwrap();

    let first_task =
        launch_cancelled_worker(&mut kernel, supervisor, worker, resource, authority, 2);
    let first_entry = kernel.agent_entry(worker).unwrap();
    assert_eq!(kernel.agent_entry_capacity(), 3);
    assert_eq!(kernel.agent_entry_count(), 2);

    let receipt = kernel
        .sys_retire_agent_entry(supervisor, authority, worker)
        .unwrap();
    assert_eq!(receipt.entry(), first_entry);
    assert_eq!(kernel.agent_entry_count(), 1);
    assert_eq!(
        kernel.agent_entry(worker),
        Err(KernelError::AgentEntryNotFound)
    );

    let second_task =
        launch_cancelled_worker(&mut kernel, supervisor, worker, resource, authority, 3);
    assert_ne!(second_task, first_task);
    assert_eq!(kernel.agent_entry(worker).unwrap().task, Some(second_task));
    assert_eq!(kernel.agent_entry_count(), 2);
}

fn launch_cancelled_worker(
    kernel: &mut TestKernel,
    supervisor: AgentId,
    worker: AgentId,
    resource: agent_kernel_core::ResourceId,
    authority: agent_kernel_core::CapabilityId,
    digest: u8,
) -> agent_kernel_core::TaskId {
    let intent = kernel
        .sys_declare_intent(
            supervisor,
            authority,
            resource,
            agent_kernel_core::IntentKind::Act,
            VerificationRequirement::Required,
        )
        .unwrap();
    let task = kernel
        .sys_create_task(supervisor, authority, intent)
        .unwrap();
    kernel
        .sys_delegate_task(supervisor, authority, task, worker)
        .unwrap();
    let capability = kernel.task(task).unwrap().delegated_capability.unwrap();
    let image = kernel
        .sys_register_agent_image(
            supervisor,
            authority,
            resource,
            AgentImageKind::Worker,
            AgentImageDigest::new([digest; 32]),
            1,
            1,
        )
        .unwrap();
    kernel
        .sys_verify_agent_image(supervisor, authority, image)
        .unwrap();
    kernel
        .sys_launch_task_agent(worker, capability, task, image, AgentEntryKind::Worker)
        .unwrap();
    kernel.sys_accept_task(worker, task).unwrap();
    kernel.sys_cancel_task(supervisor, authority, task).unwrap();
    task
}
