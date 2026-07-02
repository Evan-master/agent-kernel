//! Host-side supervisor simulator for the Agent Kernel prototype.
//!
//! This binary owns user-space experimentation. It drives the kernel facade
//! through syscall-style methods and prints the event sequence without mutating
//! kernel internals directly.

use agent_kernel::AgentKernel;
use agent_kernel_core::{
    ActionId, AgentId, CheckpointId, Event, EventKind, Operation, OperationSet, ResourceKind,
};

fn main() {
    let mut kernel = AgentKernel::<8, 8, 24, 8, 8>::new();
    let agent = AgentId::new(1);
    let target_agent = AgentId::new(2);

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
    let task = kernel
        .sys_create_task(agent, owner_capability, workspace)
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

    println!("Agent Kernel supervisor boot");
    for event in kernel.events() {
        println!("{}", format_event(event));
    }
}

fn format_event(event: &Event) -> String {
    let agent = event.agent.raw();
    let resource = event
        .resource
        .map(|resource| resource.raw())
        .unwrap_or_default();

    match event.kind {
        EventKind::Observation => {
            format!(
                "event[{}] observation agent={} resource={}",
                event.sequence, agent, resource
            )
        }
        EventKind::CheckpointCreated => {
            let checkpoint = event
                .checkpoint
                .map(|checkpoint| checkpoint.raw())
                .unwrap_or_default();
            format!(
                "event[{}] checkpoint agent={} resource={} checkpoint={}",
                event.sequence, agent, resource, checkpoint
            )
        }
        EventKind::RollbackRequested => {
            let checkpoint = event
                .checkpoint
                .map(|checkpoint| checkpoint.raw())
                .unwrap_or_default();
            format!(
                "event[{}] rollback agent={} resource={} checkpoint={}",
                event.sequence, agent, resource, checkpoint
            )
        }
        EventKind::ActionExecuted => {
            let action = event.action.map(|action| action.raw()).unwrap_or_default();
            format!(
                "event[{}] action agent={} resource={} action={}",
                event.sequence, agent, resource, action
            )
        }
        EventKind::VerificationRequested => {
            let action = event.action.map(|action| action.raw()).unwrap_or_default();
            format!(
                "event[{}] verification agent={} resource={} action={}",
                event.sequence, agent, resource, action
            )
        }
        EventKind::DelegationRequested => {
            let task = event.task.map(|task| task.raw()).unwrap_or_default();
            let target_agent = event
                .target_agent
                .map(|agent| agent.raw())
                .unwrap_or_default();
            format!(
                "event[{}] delegation agent={} resource={} task={} target_agent={}",
                event.sequence, agent, resource, task, target_agent
            )
        }
        EventKind::TaskCreated => format_task_event(event, "task_created"),
        EventKind::TaskAccepted => format_task_event(event, "task_accepted"),
        EventKind::TaskCompleted => format_task_event(event, "task_completed"),
        EventKind::TaskVerified => format_task_event(event, "task_verified"),
        EventKind::TaskCancelled => format_task_event(event, "task_cancelled"),
        EventKind::TaskQueued => format_task_event(event, "task_queued"),
        EventKind::TaskDispatched => format_task_event(event, "task_dispatched"),
        EventKind::TaskYielded => format_task_event(event, "task_yielded"),
    }
}

fn format_task_event(event: &Event, label: &str) -> String {
    let agent = event.agent.raw();
    let resource = event
        .resource
        .map(|resource| resource.raw())
        .unwrap_or_default();
    let task = event.task.map(|task| task.raw()).unwrap_or_default();

    format!(
        "event[{}] {} agent={} resource={} task={}",
        event.sequence, label, agent, resource, task
    )
}
