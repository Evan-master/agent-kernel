//! Host-side supervisor simulator for the Agent Kernel prototype.
//!
//! This binary owns user-space experimentation. It drives the kernel facade
//! through syscall-style methods and prints the event sequence without mutating
//! kernel internals directly.

mod durable_archive_flow;
mod flow_resources;
mod format;
mod format_agent;
mod format_driver;
mod format_event_archive;
mod format_fault;
mod format_signal;
mod virtual_device;

use agent_kernel_core::{
    ActionId, AgentEntryKind, AgentId, AgentImageDigest, AgentImageKind, CheckpointId, FaultKind,
    FaultPolicyAction, IntentKind, MessageKind, MessagePayload, Operation, OperationSet,
    ResourceKind, SignalKey, VerificationRequirement,
};

use crate::durable_archive_flow::commit_signed_archive;
use crate::flow_resources::{
    drive_driver_flow, drive_resource_flow, ResourceFlowContext, SupervisorKernel,
};
use crate::format::format_event;
use crate::format_event_archive::{
    format_durable_archive_receipt, format_event_archive_checkpoint,
};

fn main() {
    let mut kernel = SupervisorKernel::new();
    let agent = AgentId::new(1);
    let target_agent = AgentId::new(2);
    let handler_agent = AgentId::new(3);

    kernel
        .sys_register_agent(agent)
        .expect("owner agent should fit in simulator kernel");
    kernel
        .sys_register_agent(target_agent)
        .expect("target agent should fit in simulator kernel");
    kernel
        .sys_register_agent(handler_agent)
        .expect("handler agent should fit in simulator kernel");
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
    let supervisor_image = kernel
        .sys_register_agent_image(
            agent,
            owner_capability,
            workspace,
            AgentImageKind::Supervisor,
            AgentImageDigest::new([1; 32]),
            1,
            1,
        )
        .expect("supervisor image should register");
    kernel
        .sys_verify_agent_image(agent, owner_capability, supervisor_image)
        .expect("supervisor image should verify");
    kernel
        .sys_launch_agent(
            agent,
            owner_capability,
            workspace,
            supervisor_image,
            AgentEntryKind::Supervisor,
            None,
        )
        .expect("owner agent should launch into workspace entry");
    kernel
        .sys_install_fault_handler(
            agent,
            owner_capability,
            workspace,
            FaultKind::ExecutionTrap,
            handler_agent,
        )
        .expect("agent should install workspace fault handler");
    kernel
        .sys_install_fault_policy(
            agent,
            owner_capability,
            workspace,
            FaultKind::ExecutionTrap,
            FaultPolicyAction::RouteToHandler,
        )
        .expect("agent should install workspace fault policy");
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
    let worker_image = kernel
        .sys_register_agent_image(
            agent,
            owner_capability,
            workspace,
            AgentImageKind::Worker,
            AgentImageDigest::new([2; 32]),
            1,
            1,
        )
        .expect("worker image should register");
    kernel
        .sys_verify_agent_image(agent, owner_capability, worker_image)
        .expect("worker image should verify");
    kernel
        .sys_launch_task_agent(
            target_agent,
            assignee_capability,
            task,
            worker_image,
            AgentEntryKind::Worker,
        )
        .expect("target agent should launch into delegated task entry");
    kernel
        .sys_accept_task(target_agent, task)
        .expect("target agent should accept task");
    kernel
        .sys_enqueue_task(target_agent, task)
        .expect("target agent should enqueue accepted task");
    let dispatched = kernel
        .sys_dispatch_next_with_quantum(target_agent, 2)
        .expect("target agent should dispatch next task with quantum");
    assert_eq!(dispatched, task);
    kernel
        .sys_tick_task(target_agent, task)
        .expect("target agent should advance task by one tick");
    kernel
        .sys_tick_task(target_agent, task)
        .expect("target agent should expire task quantum");
    let dispatched = kernel
        .sys_dispatch_next_with_quantum(target_agent, 2)
        .expect("target agent should redispatch expired task");
    assert_eq!(dispatched, task);
    let fault = kernel
        .sys_fault_task(target_agent, task, FaultKind::ExecutionTrap, 7)
        .expect("target agent should fault running task");
    let fault_policy_outcome = kernel
        .sys_apply_fault_policy(agent, owner_capability, fault)
        .expect("agent should apply fault policy");
    let fault_message = fault_policy_outcome
        .message
        .expect("route policy should produce fault message");
    let received_fault = kernel
        .sys_receive_message(handler_agent)
        .expect("handler should receive routed fault");
    assert_eq!(received_fault, fault_message);
    kernel
        .sys_acknowledge_message(handler_agent, fault_message)
        .expect("handler should acknowledge routed fault");
    kernel
        .sys_recover_faulted_task(agent, owner_capability, task)
        .expect("owner rollback capability should recover faulted task");
    kernel
        .sys_enqueue_task(target_agent, task)
        .expect("target agent should requeue recovered task");
    let dispatched = kernel
        .sys_dispatch_next_with_quantum(target_agent, 2)
        .expect("target agent should redispatch recovered task");
    assert_eq!(dispatched, task);
    let wait_signal = SignalKey::new(1);
    kernel
        .sys_wait_task(
            target_agent,
            assignee_capability,
            task,
            workspace,
            wait_signal,
        )
        .expect("target agent should wait on workspace signal");
    let signal_outcome = kernel
        .sys_emit_signal(agent, owner_capability, workspace, wait_signal)
        .expect("owner agent should emit workspace signal");
    assert_eq!(signal_outcome.woken_task, Some(task));
    let dispatched = kernel
        .sys_dispatch_next_with_quantum(target_agent, 2)
        .expect("target agent should redispatch woken task");
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
                fault: None,
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

    drive_resource_flow(
        &mut kernel,
        ResourceFlowContext {
            agent,
            target_agent,
            owner_capability,
            workspace,
            task,
        },
    );
    drive_driver_flow(
        &mut kernel,
        ResourceFlowContext {
            agent,
            target_agent,
            owner_capability,
            workspace,
            task,
        },
    );

    println!("Agent Kernel supervisor boot");
    for event in kernel.events() {
        println!("{}", format_event(event));
    }
    let proposal = kernel
        .sys_prepare_event_archive(64)
        .expect("event prefix should produce an archive proposal");
    let outcome = commit_signed_archive(
        &mut kernel,
        agent,
        owner_capability,
        owner_capability,
        workspace,
        workspace,
        proposal,
    )
    .expect("signed durable archive flow should commit");
    println!("{}", format_event_archive_checkpoint(&outcome.checkpoint));
    println!("{}", format_durable_archive_receipt(&outcome.receipt));
}
