//! Phase proof for the resident Runtime Admission Supervisor.

mod compaction;
mod release;
mod task_compaction;
mod terminal;

use agent_kernel_core::{
    AgentExecutionState, AgentId, AgentImageId, EventKind, IntentStatus, MessageKind,
    MessageStatus, RuntimeAdmissionStatus, TaskId, TaskStatus, WaiterKind,
};

use super::{PreparedAdmissionSupervisorFlow, ADMISSION_SUPERVISOR};
use crate::X86BootedKernel;

pub(super) type AdmissionTarget = (AgentId, TaskId, AgentImageId);

impl PreparedAdmissionSupervisorFlow {
    pub(crate) fn waiting_after_requests(
        &self,
        booted: &X86BootedKernel,
        targets: [AdmissionTarget; 2],
    ) -> bool {
        let kernel = booted.kernel();
        let task = kernel
            .tasks()
            .iter()
            .find(|record| record.id == self.supervisor.task);
        let execution = kernel
            .execution_contexts()
            .iter()
            .find(|record| record.agent == ADMISSION_SUPERVISOR);
        let admissions = kernel.runtime_admissions();
        let waiter = kernel.waiters().last();
        matches!(task, Some(task) if task.status == TaskStatus::Waiting
            && task.assignee == Some(ADMISSION_SUPERVISOR)
            && task.delegated_capability == Some(self.supervisor.task_capability)
            && task.result.is_none()
            && task.run_ticks == 1)
            && matches!(execution, Some(execution)
                if execution.state == AgentExecutionState::Waiting
                    && execution.task == Some(self.supervisor.task))
            && kernel.run_queue().is_empty()
            && kernel.messages().len() == 2
            && kernel.waiters().len() == 3
            && matches!(waiter, Some(waiter)
                if waiter.id.raw() == 3
                    && waiter.task == self.supervisor.task
                    && waiter.agent == ADMISSION_SUPERVISOR
                    && waiter.kind == WaiterKind::Mailbox
                    && waiter.active)
            && admissions.len() == 2
            && admissions.iter().enumerate().all(|(index, admission)| {
                admission_matches(
                    admission,
                    index,
                    self.supervisor.admission_authority,
                    targets[index],
                    RuntimeAdmissionStatus::Requested,
                )
            })
            && matches!(kernel.events().last(), Some(event)
                if event.kind == EventKind::MessageWaitStarted
                    && event.agent == ADMISSION_SUPERVISOR
                    && event.task == Some(self.supervisor.task)
                    && event.waiter == waiter.map(|waiter| waiter.id))
    }

    pub(crate) fn waiting_between_batches(
        &self,
        booted: &X86BootedKernel,
        targets: [AdmissionTarget; 4],
    ) -> bool {
        let kernel = booted.kernel();
        let task = kernel
            .tasks()
            .iter()
            .find(|record| record.id == self.supervisor.task);
        let execution = kernel
            .execution_contexts()
            .iter()
            .find(|record| record.agent == ADMISSION_SUPERVISOR);
        let admissions = kernel.runtime_admissions();
        let notices = kernel.messages().get(2..);
        let waiter = kernel.waiters().last();

        matches!(task, Some(task) if task.status == TaskStatus::Waiting
            && task.assignee == Some(ADMISSION_SUPERVISOR)
            && task.delegated_capability == Some(self.supervisor.task_capability)
            && task.result.is_none()
            && task.run_ticks == 1)
            && matches!(execution, Some(execution)
                if execution.state == AgentExecutionState::Waiting
                    && execution.task == Some(self.supervisor.task))
            && kernel.run_queue().is_empty()
            && kernel.messages().len() == 4
            && kernel.waiters().len() == 4
            && matches!(kernel.waiters().get(2), Some(first)
                if first.id.raw() == 3
                    && first.agent == ADMISSION_SUPERVISOR
                    && first.kind == WaiterKind::Mailbox
                    && !first.active)
            && matches!(waiter, Some(waiter)
                if waiter.id.raw() == 4
                    && waiter.task == self.supervisor.task
                    && waiter.agent == ADMISSION_SUPERVISOR
                    && waiter.kind == WaiterKind::Mailbox
                    && waiter.active)
            && admissions.len() == 4
            && admissions.iter().enumerate().all(|(index, admission)| {
                let status = if index < 2 {
                    RuntimeAdmissionStatus::Admitted
                } else {
                    RuntimeAdmissionStatus::Requested
                };
                admission_matches(
                    admission,
                    index,
                    self.supervisor.admission_authority,
                    targets[index],
                    status,
                )
            })
            && matches!(notices, Some(notices) if notices.len() == 2
            && notices.iter().enumerate().all(|(index, message)| {
                acknowledged_notice(message, index + 3, targets[index], ADMISSION_SUPERVISOR)
            }))
            && matches!(kernel.events().last(), Some(event)
                if event.kind == EventKind::MessageWaitStarted
                    && event.agent == ADMISSION_SUPERVISOR
                    && event.task == Some(self.supervisor.task)
                    && event.waiter == waiter.map(|waiter| waiter.id))
    }

    pub(crate) fn verify_completed(&self, booted: &mut X86BootedKernel) -> Option<()> {
        let report = *booted.report();
        booted
            .kernel_mut()
            .sys_verify_task(
                report.bootstrap_agent,
                report.bootstrap_capability,
                self.supervisor.task,
            )
            .ok()?;
        let task = booted
            .kernel()
            .tasks()
            .iter()
            .find(|record| record.id == self.supervisor.task)?;
        let intent = booted
            .kernel()
            .intents()
            .iter()
            .find(|record| record.id == self.supervisor.intent)?;
        (task.status == TaskStatus::Verified
            && intent.status == IntentStatus::Fulfilled
            && matches!(booted.kernel().events().last(), Some(event)
                if event.kind == EventKind::IntentFulfilled
                    && event.task == Some(self.supervisor.task)))
        .then_some(())
    }
}

pub(super) fn admission_matches(
    admission: &agent_kernel_core::RuntimeAdmissionRecord,
    index: usize,
    authority: agent_kernel_core::CapabilityId,
    target: AdmissionTarget,
    status: RuntimeAdmissionStatus,
) -> bool {
    admission.id.raw() == (index + 1) as u64
        && admission.requester == ADMISSION_SUPERVISOR
        && admission.authority == authority
        && admission.target == target.0
        && admission.task == target.1
        && admission.image == target.2
        && admission.status == status
        && admission.failure.is_none()
}

pub(super) fn acknowledged_notice(
    message: &agent_kernel_core::MessageRecord,
    id: usize,
    target: AdmissionTarget,
    recipient: AgentId,
) -> bool {
    message.id.raw() == id as u64
        && message.sender == target.0
        && message.recipient == recipient
        && message.kind == MessageKind::Notify
        && message.payload.task == Some(target.1)
        && message.payload.resource.is_none()
        && message.payload.capability.is_none()
        && message.payload.intent.is_none()
        && message.payload.action.is_none()
        && message.payload.fault.is_none()
        && message.status == MessageStatus::Acknowledged
}
