//! Terminal native and semantic evidence for one Reuse Worker.

use agent_kernel_core::{AgentExecutionState, EventKind, IntentStatus, TaskStatus};

use super::PreparedReuseWorkerFlow;
use crate::{
    boot_agent_images::BootReuseWorkerImage, native_agent_executor::NativeExecutionReport,
    X86BootedKernel,
};

impl PreparedReuseWorkerFlow {
    pub(crate) fn completed_after_runtime(
        &self,
        booted: &X86BootedKernel,
        report: &NativeExecutionReport,
        contract: BootReuseWorkerImage,
    ) -> bool {
        let Some(context) = self.call_context() else {
            return false;
        };
        let Some(completed) = report.completed(self.agent) else {
            return false;
        };
        let task = booted
            .kernel()
            .tasks()
            .iter()
            .find(|record| record.id == self.task);
        let execution = booted
            .kernel()
            .execution_contexts()
            .iter()
            .find(|record| record.agent == self.agent);
        matches!(task, Some(task) if task.status == TaskStatus::Completed
            && task.assignee == Some(self.agent)
            && task.delegated_capability == Some(self.capability)
            && task.result == Some(contract.result())
            && task.run_ticks == 1)
            && matches!(execution, Some(execution)
                if execution.state == AgentExecutionState::Idle && execution.task.is_none())
            && completed.context() == context
            && completed.nonce() == contract.nonce()
            && completed.call_count() == 3
            && completed.address_space_switch_count() == 6
            && completed.operations() == contract.expected_operations()
            && completed.return_offsets() == contract.expected_return_offsets()
            && completed.physical_quantum_generation() == 1
            && completed.restart_generation() == 0
            && completed.lazy_data_byte() == 0
            && completed.runtime_page_generation() == 0
            && !completed.runtime_page_released()
            && completed.runtime_page_observation().is_none()
            && completed.runtime_region_generation() == 0
            && !completed.runtime_regions_released()
            && completed.runtime_region_observations().is_empty()
            && completed.reclamation_log().is_empty()
            && booted.kernel().events().iter().any(|event| {
                event.kind == EventKind::TaskCompleted
                    && event.agent == self.agent
                    && event.task == Some(self.task)
            })
    }

    pub(crate) fn verify_completed(&self, booted: &mut X86BootedKernel) -> Option<()> {
        let report = *booted.report();
        booted
            .kernel_mut()
            .sys_verify_task(
                report.bootstrap_agent,
                report.bootstrap_capability,
                self.task,
            )
            .ok()?;
        let task = booted
            .kernel()
            .tasks()
            .iter()
            .find(|record| record.id == self.task)?;
        let intent = booted
            .kernel()
            .intents()
            .iter()
            .find(|record| record.id == self.intent)?;
        (task.status == TaskStatus::Verified
            && intent.status == IntentStatus::Fulfilled
            && matches!(booted.kernel().events().last(), Some(event)
                if event.kind == EventKind::IntentFulfilled
                    && event.task == Some(self.task)))
        .then_some(())
    }
}
