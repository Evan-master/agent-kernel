use agent_kernel::AgentKernel;
use agent_kernel_core::{
    AgentEntryKind, AgentId, AgentImageDigest, AgentImageKind, EventKind, IntentKind, Operation,
    OperationSet, ResourceKind, RuntimeAdmissionStatus, VerificationRequirement,
};

type TestKernel = AgentKernel<3, 2, 8, 40, 0, 0, 0, 2, 2, 2>;

#[test]
fn facade_exposes_request_prepare_and_atomic_commit() {
    let mut kernel = TestKernel::new();
    let supervisor = AgentId::new(1);
    let target = AgentId::new(2);
    kernel
        .sys_register_agent(supervisor)
        .expect("supervisor registers");
    kernel.sys_register_agent(target).expect("target registers");
    let resource = kernel
        .sys_register_resource(ResourceKind::Workspace, None)
        .expect("resource registers");
    let authority = kernel
        .sys_grant(
            supervisor,
            resource,
            OperationSet::only(Operation::Act)
                .with(Operation::Delegate)
                .with(Operation::Verify),
        )
        .expect("authority grants");
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
        .expect("supervisor image registers");
    kernel
        .sys_verify_agent_image(supervisor, authority, supervisor_image)
        .expect("supervisor image verifies");
    kernel
        .sys_launch_agent(
            supervisor,
            authority,
            resource,
            supervisor_image,
            AgentEntryKind::Supervisor,
            None,
        )
        .expect("supervisor launches");
    let intent = kernel
        .sys_declare_intent(
            supervisor,
            authority,
            resource,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .expect("intent declares");
    let task = kernel
        .sys_create_task(supervisor, authority, intent)
        .expect("task creates");
    kernel
        .sys_delegate_task(supervisor, authority, task, target)
        .expect("task delegates");
    let task_capability = kernel.tasks()[0]
        .delegated_capability
        .expect("task capability exists");
    let image = kernel
        .sys_register_agent_image(
            supervisor,
            authority,
            resource,
            AgentImageKind::Worker,
            AgentImageDigest::new([2; 32]),
            1,
            1,
        )
        .expect("worker image registers");
    kernel
        .sys_verify_agent_image(supervisor, authority, image)
        .expect("worker image verifies");
    kernel
        .sys_launch_task_agent(target, task_capability, task, image, AgentEntryKind::Worker)
        .expect("worker launches");
    kernel.sys_accept_task(target, task).expect("task accepts");

    let admission = kernel
        .sys_request_runtime_admission(supervisor, authority, target, task)
        .expect("request crosses facade");
    let permit = kernel
        .sys_prepare_next_runtime_admission()
        .expect("permit crosses facade");
    let record = kernel
        .sys_commit_runtime_admission(permit)
        .expect("commit crosses facade");

    assert_eq!(record.id, admission);
    assert_eq!(record.status, RuntimeAdmissionStatus::Admitted);
    assert_eq!(kernel.runtime_admissions(), [record]);
    assert_eq!(
        kernel.events()[kernel.events().len() - 2].kind,
        EventKind::RuntimeAdmissionAdmitted
    );
    assert_eq!(kernel.events().last().unwrap().kind, EventKind::TaskQueued);

    kernel
        .sys_dispatch_next(target)
        .expect("target dispatches through facade");
    kernel
        .sys_complete_task(target, task_capability, task)
        .expect("target completes through facade");
    kernel
        .sys_verify_task(supervisor, authority, task)
        .expect("target verifies through facade");
    let release = kernel
        .sys_prepare_runtime_admission_release_batch([admission])
        .expect("release permit crosses facade");
    let [released] = kernel
        .sys_commit_runtime_admission_release_batch(release)
        .expect("release commit crosses facade");

    assert_eq!(released.status, RuntimeAdmissionStatus::Released);
    assert_eq!(kernel.runtime_admissions(), [released]);
    assert_eq!(
        kernel.events().last().unwrap().kind,
        EventKind::RuntimeAdmissionReleased
    );
}
