//! Host-side supervisor simulator for the Agent Kernel prototype.
//!
//! This binary owns user-space experimentation. It drives the kernel facade
//! through syscall-style methods and prints the event sequence without mutating
//! kernel internals directly.

mod format;

use agent_kernel::AgentKernel;
use agent_kernel_core::{
    ActionId, AgentId, CheckpointId, IntentKind, MessageKind, MessagePayload, Operation,
    OperationSet, ResourceKind, VerificationRequirement,
};

use crate::format::format_event;

fn main() {
    let mut kernel = AgentKernel::<8, 8, 8, 40, 8, 8, 8, 8, 8, 8, 8>::new();
    let agent = AgentId::new(1);
    let target_agent = AgentId::new(2);

    kernel
        .sys_register_agent(agent)
        .expect("owner agent should fit in simulator kernel");
    kernel
        .sys_register_agent(target_agent)
        .expect("target agent should fit in simulator kernel");
    let workspace = kernel
        .sys_register_resource(ResourceKind::Workspace, None)
        .expect("workspace resource should fit in simulator kernel");
    let owner_capability = kernel
        .sys_grant(
            agent,
            workspace,
            OperationSet::empty()
                .with(Operation::Observe)
                .with(Operation::Act)
                .with(Operation::Verify)
                .with(Operation::Checkpoint)
                .with(Operation::Rollback)
                .with(Operation::Delegate),
        )
        .expect("agent capability should fit in simulator kernel");
    let action = ActionId::new(1);
    let checkpoint = CheckpointId::new(1);
    kernel
        .sys_observe(agent, owner_capability, workspace)
        .expect("agent should observe workspace");
    kernel
        .sys_act(agent, owner_capability, action, workspace)
        .expect("agent should execute action");
    kernel
        .sys_verify(agent, owner_capability, action, workspace)
        .expect("agent should request verification");
    kernel
        .sys_checkpoint(agent, owner_capability, checkpoint, workspace)
        .expect("agent should checkpoint workspace");
    kernel
        .sys_rollback(agent, owner_capability, checkpoint, workspace)
        .expect("agent should request rollback");
    let intent = kernel
        .sys_declare_intent(
            agent,
            owner_capability,
            workspace,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .expect("agent should declare action intent");
    let task = kernel
        .sys_create_task(agent, owner_capability, intent)
        .expect("agent should create task");
    kernel
        .sys_delegate_task(agent, owner_capability, task, target_agent)
        .expect("agent should request task delegation");
    let assignee_capability = kernel.tasks()[0]
        .delegated_capability
        .expect("delegation should derive target agent capability");
    kernel
        .sys_accept_task(target_agent, task)
        .expect("target agent should accept task");
    kernel
        .sys_enqueue_task(target_agent, task)
        .expect("target agent should enqueue accepted task");
    let dispatched = kernel
        .sys_dispatch_next(target_agent)
        .expect("target agent should dispatch next task");
    assert_eq!(dispatched, task);
    kernel
        .sys_complete_task(target_agent, assignee_capability, task)
        .expect("target agent should complete task");
    kernel
        .sys_verify_task(agent, owner_capability, task)
        .expect("agent should verify task");
    let message = kernel
        .sys_send_message(
            agent,
            target_agent,
            MessageKind::Notify,
            MessagePayload {
                resource: Some(workspace),
                capability: None,
                intent: Some(intent),
                task: Some(task),
                action: None,
            },
        )
        .expect("agent should send task notification");
    let received = kernel
        .sys_receive_message(target_agent)
        .expect("target agent should receive task notification");
    assert_eq!(received, message);
    kernel
        .sys_acknowledge_message(target_agent, message)
        .expect("target agent should acknowledge task notification");

    println!("Agent Kernel supervisor boot");
    for event in kernel.events() {
        println!("{}", format_event(event));
    }
}
