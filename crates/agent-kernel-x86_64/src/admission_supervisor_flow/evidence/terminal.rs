//! Terminal transcript and notification proof for the resident Supervisor.

use agent_kernel_core::{
    AgentExecutionState, EventKind, RuntimeAdmissionStatus, TaskStatus, WaiterKind,
};

use super::{acknowledged_notice, admission_matches, AdmissionTarget};
use crate::{
    admission_supervisor_flow::{PreparedAdmissionSupervisorFlow, ADMISSION_SUPERVISOR},
    boot_agent_images::BootAdmissionSupervisorImage,
    native_agent_executor::NativeExecutionReport,
    X86BootedKernel,
};

impl PreparedAdmissionSupervisorFlow {
    pub(crate) fn completed_after_notifications(
        &self,
        booted: &X86BootedKernel,
        report: &NativeExecutionReport,
        contract: BootAdmissionSupervisorImage,
        targets: [AdmissionTarget; 4],
    ) -> bool {
        let Some(context) = self.call_context() else {
            return false;
        };
        let Some(completed) = report.completed(ADMISSION_SUPERVISOR) else {
            return false;
        };
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

        matches!(task, Some(task) if task.status == TaskStatus::Completed
            && task.assignee == Some(ADMISSION_SUPERVISOR)
            && task.delegated_capability == Some(self.supervisor.task_capability)
            && task.result == Some(contract.result())
            && task.run_ticks == 1)
            && matches!(execution, Some(execution)
                if execution.state == AgentExecutionState::Idle && execution.task.is_none())
            && kernel.run_queue().is_empty()
            && completed.context() == context
            && completed.nonce() == contract.nonce()
            && completed.call_count() == 17
            && completed.address_space_switch_count() == 34
            && completed.operations() == contract.expected_operations()
            && completed.return_offsets() == contract.expected_return_offsets()
            && completed.physical_quantum_generation() == 1
            && completed.reclamation_log().is_empty()
            && admissions.len() == 2
            && admissions.iter().enumerate().all(|(index, admission)| {
                admission_matches(
                    admission,
                    index + 2,
                    self.supervisor.admission_authority,
                    targets[index + 2],
                    RuntimeAdmissionStatus::Admitted,
                )
            })
            && self.first_batch_compacted(booted, targets)
            && self.initial_task_prefix_compacted(booted)
            && kernel.messages().len() == 6
            && matches!(kernel.waiters().last(), Some(waiter)
                if waiter.id.raw() == 4
                    && waiter.agent == ADMISSION_SUPERVISOR
                    && waiter.kind == WaiterKind::Mailbox
                    && !waiter.active)
            && matches!(notices, Some(notices) if notices.len() == 4
            && notices.iter().enumerate().all(|(index, message)| {
                acknowledged_notice(message, index + 3, targets[index], ADMISSION_SUPERVISOR)
            }))
            && matches!(kernel.events().last(), Some(event)
                if event.kind == EventKind::TaskCompleted
                    && event.agent == ADMISSION_SUPERVISOR
                    && event.task == Some(self.supervisor.task))
    }
}
