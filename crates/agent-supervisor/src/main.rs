//! Host-side supervisor simulator for the Agent Kernel prototype.
//!
//! This binary owns user-space experimentation. It drives the kernel facade
//! through syscall-style methods and prints the event sequence without mutating
//! kernel internals directly.

use agent_kernel::AgentKernel;
use agent_kernel_core::{
    AgentId, CheckpointId, Event, EventKind, Operation, OperationSet, ResourceKind,
};

fn main() {
    let mut kernel = AgentKernel::<8, 8, 16>::new();
    let agent = AgentId::new(1);

    let workspace = kernel
        .sys_register_resource(ResourceKind::Workspace, None)
        .expect("workspace resource should fit in simulator kernel");
    let capability = kernel
        .sys_grant(
            agent,
            workspace,
            OperationSet::empty()
                .with(Operation::Observe)
                .with(Operation::Checkpoint)
                .with(Operation::Rollback),
        )
        .expect("agent capability should fit in simulator kernel");

    let checkpoint = CheckpointId::new(1);
    kernel
        .sys_observe(agent, capability, workspace)
        .expect("agent should observe workspace");
    kernel
        .sys_checkpoint(agent, capability, checkpoint, workspace)
        .expect("agent should checkpoint workspace");
    kernel
        .sys_rollback(agent, capability, checkpoint, workspace)
        .expect("agent should request rollback");

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
            format!(
                "event[{}] action agent={} resource={}",
                event.sequence, agent, resource
            )
        }
        EventKind::VerificationRequested => {
            format!(
                "event[{}] verification agent={} resource={}",
                event.sequence, agent, resource
            )
        }
        EventKind::DelegationRequested => {
            format!(
                "event[{}] delegation agent={} resource={}",
                event.sequence, agent, resource
            )
        }
    }
}
