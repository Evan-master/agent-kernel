//! Terminal semantic and native proof for the Runtime Admission Supervisor.

use agent_kernel_core::{
    AgentExecutionState, AgentId, AgentImageId, EventKind, IntentStatus, RuntimeAdmissionStatus,
    TaskId, TaskStatus,
};

use super::{PreparedAdmissionSupervisorFlow, ADMISSION_SUPERVISOR};
use crate::{
    boot_agent_images::BootAdmissionSupervisorImage, native_agent_executor::NativeExecutionReport,
    X86BootedKernel,
};

impl PreparedAdmissionSupervisorFlow {
    pub(crate) fn completed_after_runtime(
        &self,
        booted: &X86BootedKernel,
        report: &NativeExecutionReport,
        contract: BootAdmissionSupervisorImage,
        targets: [(AgentId, TaskId, AgentImageId); 2],
    ) -> bool {
        let Some(context) = self.call_context() else {
            return false;
        };
        let Some(completed) = report.completed(ADMISSION_SUPERVISOR) else {
            return false;
        };
        let task = booted
            .kernel()
            .tasks()
            .iter()
            .find(|record| record.id == self.supervisor.task);
        let execution = booted
            .kernel()
            .execution_contexts()
            .iter()
            .find(|record| record.agent == ADMISSION_SUPERVISOR);
        let admissions = booted.kernel().runtime_admissions();
        matches!(task, Some(task) if task.status == TaskStatus::Completed
            && task.assignee == Some(ADMISSION_SUPERVISOR)
            && task.delegated_capability == Some(self.supervisor.task_capability)
            && task.result == Some(contract.result())
            && task.run_ticks == 1)
            && matches!(execution, Some(execution)
                if execution.state == AgentExecutionState::Idle && execution.task.is_none())
            && completed.context() == context
            && completed.nonce() == contract.nonce()
            && completed.call_count() == 5
            && completed.address_space_switch_count() == 10
            && completed.operations() == contract.expected_operations()
            && completed.return_offsets() == contract.expected_return_offsets()
            && completed.physical_quantum_generation() == 1
            && completed.reclamation_log().is_empty()
            && admissions.len() == 2
            && admissions.iter().enumerate().all(|(index, admission)| {
                admission.id.raw() == (index + 1) as u64
                    && admission.requester == ADMISSION_SUPERVISOR
                    && admission.authority == self.supervisor.admission_authority
                    && admission.target == targets[index].0
                    && admission.task == targets[index].1
                    && admission.image == targets[index].2
                    && admission.status == RuntimeAdmissionStatus::Requested
                    && admission.failure.is_none()
            })
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
