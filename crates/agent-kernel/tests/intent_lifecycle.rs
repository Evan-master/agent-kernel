use agent_kernel::AgentKernel;
use agent_kernel_core::{
    AgentId, EventKind, IntentId, IntentKind, Operation, OperationSet, ResourceKind, TaskId,
    VerificationRequirement,
};

type TestKernel = AgentKernel<4, 4, 16, 4, 4, 4>;

#[test]
fn sys_declare_intent_records_and_exposes_intent() {
    let mut kernel = TestKernel::new();
    let agent = AgentId::new(1);
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
    assert_eq!(
        kernel.intents()[0].verification,
        VerificationRequirement::Required
    );
    assert_eq!(kernel.events()[1].kind, EventKind::IntentDeclared);
    assert_eq!(kernel.events()[1].intent, Some(intent));
}

#[test]
fn sys_create_task_accepts_intent_id() {
    let mut kernel = TestKernel::new();
    let agent = AgentId::new(2);
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
    assert_eq!(kernel.events()[2].kind, EventKind::TaskCreated);
    assert_eq!(kernel.events()[2].intent, Some(intent));
}
