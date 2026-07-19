//! Dense Agent Entry Store and retirement Event proof for the first batch.

use agent_kernel_core::{
    AgentId, AgentImageKind, CapabilityId, Event, EventKind, KernelError, Operation, Task,
    TaskStatus,
};

use super::AdmissionTarget;
use crate::{
    admission_supervisor_flow::{PreparedAdmissionSupervisorFlow, ADMISSION_SUPERVISOR},
    X86BootedKernel,
};

const RETAINED_AGENTS: [AgentId; 11] = [
    AgentId::new(1),
    AgentId::new(2),
    AgentId::new(4),
    AgentId::new(3),
    AgentId::new(5),
    AgentId::new(6),
    AgentId::new(7),
    AgentId::new(8),
    AgentId::new(12),
    AgentId::new(13),
    AgentId::new(14),
];

impl PreparedAdmissionSupervisorFlow {
    pub(super) fn first_batch_entries_retired(
        &self,
        booted: &X86BootedKernel,
        targets: [AdmissionTarget; 4],
    ) -> bool {
        let kernel = booted.kernel();
        let Ok(first_task) = kernel.task(targets[0].1) else {
            return false;
        };
        let Ok(second_task) = kernel.task(targets[1].1) else {
            return false;
        };
        let Some(first_capability) = first_task.delegated_capability else {
            return false;
        };
        let Some(second_capability) = second_task.delegated_capability else {
            return false;
        };
        let Ok(first_capability_record) = kernel.capability(first_capability) else {
            return false;
        };
        let Ok(second_capability_record) = kernel.capability(second_capability) else {
            return false;
        };
        let mut events = kernel
            .events()
            .iter()
            .filter(|event| event.kind == EventKind::AgentEntryRetired);
        let Some(first_event) = events.next() else {
            return false;
        };
        let Some(second_event) = events.next() else {
            return false;
        };

        kernel.agent_entry_capacity() == 14
            && kernel.agent_entry_count() == RETAINED_AGENTS.len()
            && kernel
                .agent_entries()
                .iter()
                .map(|entry| entry.agent)
                .eq(RETAINED_AGENTS)
            && kernel.agent_entry(targets[0].0) == Err(KernelError::AgentEntryNotFound)
            && kernel.agent_entry(targets[1].0) == Err(KernelError::AgentEntryNotFound)
            && task_retained(first_task, targets[0], first_capability)
            && task_retained(second_task, targets[1], second_capability)
            && capability_retained(first_capability_record, targets[0].0, first_task)
            && capability_retained(second_capability_record, targets[1].0, second_task)
            && kernel
                .capability(self.supervisor.admission_authority)
                .is_ok()
            && retirement_event_matches(
                first_event,
                targets[0],
                first_task,
                first_capability,
                self.supervisor.admission_authority,
            )
            && retirement_event_matches(
                second_event,
                targets[1],
                second_task,
                second_capability,
                self.supervisor.admission_authority,
            )
            && second_event.sequence == first_event.sequence + 1
            && events.next().is_none()
    }
}

fn task_retained(task: Task, target: AdmissionTarget, capability: CapabilityId) -> bool {
    task.id == target.1
        && task.assignee == Some(target.0)
        && task.delegated_capability == Some(capability)
        && task.status == TaskStatus::Verified
}

fn capability_retained(
    capability: agent_kernel_core::Capability,
    agent: AgentId,
    task: Task,
) -> bool {
    capability.agent == agent
        && capability.resource == task.resource
        && capability.task == Some(task.id)
        && !capability.revoked
}

fn retirement_event_matches(
    event: &Event,
    target: AdmissionTarget,
    task: Task,
    capability: CapabilityId,
    authority: CapabilityId,
) -> bool {
    event.agent == ADMISSION_SUPERVISOR
        && event.target_agent == Some(target.0)
        && event.resource == Some(task.resource)
        && event.capability == Some(capability)
        && event.source_capability == Some(authority)
        && event.operation == Some(Operation::Rollback)
        && event.intent == Some(task.intent)
        && event.task == Some(task.id)
        && event.agent_image == Some(target.2)
        && event.agent_image_kind == Some(AgentImageKind::Worker)
}
