use agent_kernel::AgentKernel;
use agent_kernel_core::{
    AgentEntryKind, AgentId, AgentImageDigest, AgentImageKind, EventKind, IntentKind, Operation,
    OperationSet, ResourceKind, TaskResult, VerificationRequirement,
};

type TestKernel = AgentKernel<3, 1, 4, 40, 0, 0, 0, 2, 2, 2>;

#[test]
fn verifier_inspection_syscall_returns_audited_task_result() {
    let mut kernel = TestKernel::new();
    let owner = AgentId::new(1);
    let worker = AgentId::new(2);
    let verifier = AgentId::new(3);
    for agent in [owner, worker, verifier] {
        kernel.sys_register_agent(agent).unwrap();
    }
    let resource = kernel
        .sys_register_resource(ResourceKind::Workspace, None)
        .unwrap();
    let owner_capability = kernel
        .sys_grant(
            owner,
            resource,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Delegate)
                .with(Operation::Verify),
        )
        .unwrap();
    let target_intent = kernel
        .sys_declare_intent(
            owner,
            owner_capability,
            resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .unwrap();
    let target = kernel
        .sys_create_task(owner, owner_capability, target_intent)
        .unwrap();
    let target_capability = kernel
        .sys_delegate_task(owner, owner_capability, target, worker)
        .unwrap()
        .capability
        .unwrap();
    launch_task(
        &mut kernel,
        owner,
        owner_capability,
        resource,
        worker,
        target,
        target_capability,
        AgentImageKind::Worker,
        AgentEntryKind::Worker,
        0x11,
    );
    kernel.sys_accept_task(worker, target).unwrap();
    kernel.sys_enqueue_task(worker, target).unwrap();
    kernel.sys_dispatch_next(worker).unwrap();
    let result = TaskResult {
        code: 0x0a01,
        value: 0xa11c_0001,
    };
    kernel
        .sys_submit_task_result(worker, target_capability, target, result)
        .unwrap();
    kernel
        .sys_complete_task(worker, target_capability, target)
        .unwrap();

    let verifier_intent = kernel
        .sys_declare_intent(
            owner,
            owner_capability,
            resource,
            IntentKind::Verify,
            VerificationRequirement::Optional,
        )
        .unwrap();
    let verifier_task = kernel
        .sys_create_task(owner, owner_capability, verifier_intent)
        .unwrap();
    let verifier_task_capability = kernel
        .sys_delegate_task(owner, owner_capability, verifier_task, verifier)
        .unwrap()
        .capability
        .unwrap();
    let verify_capability = kernel
        .sys_derive_capability(
            owner,
            owner_capability,
            verifier,
            OperationSet::only(Operation::Verify),
        )
        .unwrap();
    launch_task(
        &mut kernel,
        owner,
        owner_capability,
        resource,
        verifier,
        verifier_task,
        verifier_task_capability,
        AgentImageKind::Verifier,
        AgentEntryKind::Verifier,
        0x22,
    );
    kernel.sys_accept_task(verifier, verifier_task).unwrap();
    kernel.sys_enqueue_task(verifier, verifier_task).unwrap();
    kernel.sys_dispatch_next(verifier).unwrap();

    let event = kernel
        .sys_inspect_task_result(verifier, verify_capability, target)
        .unwrap();

    assert_eq!(event.kind, EventKind::TaskResultInspected);
    assert_eq!(event.task_result, Some(result));
    assert_eq!(kernel.tasks()[0].result, Some(result));
}

#[allow(clippy::too_many_arguments)]
fn launch_task(
    kernel: &mut TestKernel,
    owner: AgentId,
    owner_capability: agent_kernel_core::CapabilityId,
    resource: agent_kernel_core::ResourceId,
    agent: AgentId,
    task: agent_kernel_core::TaskId,
    task_capability: agent_kernel_core::CapabilityId,
    image_kind: AgentImageKind,
    entry_kind: AgentEntryKind,
    digest: u8,
) {
    let image = kernel
        .sys_register_agent_image(
            owner,
            owner_capability,
            resource,
            image_kind,
            AgentImageDigest::new([digest; 32]),
            1,
            1,
        )
        .unwrap();
    kernel
        .sys_verify_agent_image(owner, owner_capability, image)
        .unwrap();
    kernel
        .sys_launch_task_agent(agent, task_capability, task, image, entry_kind)
        .unwrap();
}
