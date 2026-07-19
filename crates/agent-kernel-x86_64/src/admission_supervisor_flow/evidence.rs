//! Phase proof for the resident Runtime Admission Supervisor.

mod agent_entry_retirement;
mod capability_compaction;
mod compaction;
mod intent_compaction;
mod release;
mod task_compaction;
mod terminal;
mod waiter_compaction;

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
            && retained_boot_messages(booted)
            && kernel.waiters().len() == 1
            && matches!(waiter, Some(waiter)
                if waiter.id.raw() == 4
                    && waiter.task == self.supervisor.task
                    && waiter.agent == ADMISSION_SUPERVISOR
                    && waiter.kind == WaiterKind::Mailbox
                    && waiter.active)
            && self.first_waiter_prefix_compacted(booted)
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
            && targets[..2].iter().enumerate().all(|(index, target)| {
                retired_notice(booted, index + 4, *target, ADMISSION_SUPERVISOR)
            })
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

pub(super) fn retired_notice(
    booted: &X86BootedKernel,
    id: usize,
    target: AdmissionTarget,
    recipient: AgentId,
) -> bool {
    let mut count = 0;
    let mut matching = None;
    for event in booted.kernel().events() {
        if event.kind == EventKind::MessageRetired
            && event
                .message
                .is_some_and(|message| message.raw() == id as u64)
        {
            count += 1;
            matching = Some(event);
        }
    }
    matches!((count, matching), (1, Some(event))
        if event.agent == recipient
            && event.target_agent == Some(target.0)
            && event.message_kind == Some(MessageKind::Notify)
            && event.task == Some(target.1)
            && event.resource.is_none()
            && event.capability.is_none()
            && event.intent.is_none()
            && event.action.is_none()
            && event.fault.is_none())
}

pub(super) fn retained_boot_messages(booted: &X86BootedKernel) -> bool {
    matches!(booted.kernel().messages(), [first, second]
        if first.id.raw() == 1
            && second.id.raw() == 2
            && first.status == MessageStatus::Acknowledged
            && second.status == MessageStatus::Acknowledged)
}
