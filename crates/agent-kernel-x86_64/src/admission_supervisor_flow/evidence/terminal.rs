//! Terminal transcript and notification proof for the resident Supervisor.

use agent_kernel_core::{AgentExecutionState, EventKind, RuntimeAdmissionStatus, TaskStatus};

use super::{admission_matches, retained_boot_messages, retired_notice, AdmissionTarget};
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
            && completed.call_count() == 44
            && completed.address_space_switch_count() == 88
            && completed.operations() == contract.expected_operations()
            && completed.return_offsets() == contract.expected_return_offsets()
            && completed.physical_quantum_generation() == 1
            && completed.reclamation_log().len() == 1
            && self.event_archive_committed(booted, report)
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
            && self.fault_store_compacted(booted)
            && self.initial_intent_prefix_compacted(booted)
            && self.first_batch_entries_retired(booted, targets)
            && self.capability_store_compacted(booted)
            && self.resource_record_retired_and_reused(booted)
            && self.memory_cell_record_retired_and_reused(booted, report)
            && retained_boot_messages(booted)
            && kernel.waiters().is_empty()
            && self.waiter_store_compacted(booted)
            && targets.iter().enumerate().all(|(index, target)| {
                retired_notice(booted, index + 4, *target, ADMISSION_SUPERVISOR)
            })
            && matches!(kernel.events().last(), Some(event)
                if event.kind == EventKind::TaskCompleted
                    && event.agent == ADMISSION_SUPERVISOR
                    && event.task == Some(self.supervisor.task))
    }
}
