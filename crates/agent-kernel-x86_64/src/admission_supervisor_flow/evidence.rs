//! Terminal semantic and native proof for the Runtime Admission Supervisor.

use agent_kernel_core::{
    AgentExecutionState, AgentId, AgentImageId, EventKind, IntentStatus, MessageKind,
    MessageStatus, RuntimeAdmissionStatus, TaskId, TaskStatus, WaiterKind,
};

use super::{PreparedAdmissionSupervisorFlow, ADMISSION_SUPERVISOR};
use crate::{
    boot_agent_images::BootAdmissionSupervisorImage, native_agent_executor::NativeExecutionReport,
    X86BootedKernel,
};

impl PreparedAdmissionSupervisorFlow {
    pub(crate) fn waiting_after_requests(
        &self,
        booted: &X86BootedKernel,
        targets: [(AgentId, TaskId, AgentImageId); 2],
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
                admission.id.raw() == (index + 1) as u64
                    && admission.requester == ADMISSION_SUPERVISOR
                    && admission.authority == self.supervisor.admission_authority
                    && admission.target == targets[index].0
                    && admission.task == targets[index].1
                    && admission.image == targets[index].2
                    && admission.status == RuntimeAdmissionStatus::Requested
                    && admission.failure.is_none()
            })
            && matches!(kernel.events().last(), Some(event)
                if event.kind == EventKind::MessageWaitStarted
                    && event.agent == ADMISSION_SUPERVISOR
                    && event.task == Some(self.supervisor.task)
                    && event.waiter == waiter.map(|waiter| waiter.id))
    }

    pub(crate) fn completed_after_notifications(
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
            && completed.context() == context
            && completed.nonce() == contract.nonce()
            && completed.call_count() == 9
            && completed.address_space_switch_count() == 18
            && completed.operations() == contract.expected_operations()
            && completed.return_offsets() == contract.expected_return_offsets()
            && completed.physical_quantum_generation() == 1
            && completed.reclamation_log().is_empty()
            && admissions.len() == 2
            && admissions.iter().all(|admission| {
                admission.status == RuntimeAdmissionStatus::Admitted && admission.failure.is_none()
            })
            && matches!(kernel.waiters().last(), Some(waiter)
                if waiter.id.raw() == 3
                    && waiter.agent == ADMISSION_SUPERVISOR
                    && waiter.kind == WaiterKind::Mailbox
                    && !waiter.active)
            && matches!(notices, Some(notices) if notices.len() == 2
            && notices.iter().enumerate().all(|(index, message)| {
                message.id.raw() == (index + 3) as u64
                    && message.sender == targets[index].0
                    && message.recipient == ADMISSION_SUPERVISOR
                    && message.kind == MessageKind::Notify
                    && message.payload.task == Some(targets[index].1)
                    && message.payload.resource.is_none()
                    && message.payload.capability.is_none()
                    && message.payload.intent.is_none()
                    && message.payload.action.is_none()
                    && message.payload.fault.is_none()
                    && message.status == MessageStatus::Acknowledged
            }))
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

    pub(crate) fn released_after_reclamation(
        &self,
        booted: &X86BootedKernel,
        targets: [(AgentId, TaskId, AgentImageId); 2],
        first_event: usize,
    ) -> bool {
        let kernel = booted.kernel();
        let admissions = kernel.runtime_admissions();
        let release_events = kernel.events().get(first_event..);
        let supervisor_verified = kernel.tasks().iter().any(|task| {
            task.id == self.supervisor.task
                && task.assignee == Some(ADMISSION_SUPERVISOR)
                && task.status == TaskStatus::Verified
        });

        supervisor_verified
            && admissions.len() == 2
            && admissions.iter().enumerate().all(|(index, admission)| {
                admission.id.raw() == (index + 1) as u64
                    && admission.requester == ADMISSION_SUPERVISOR
                    && admission.authority == self.supervisor.admission_authority
                    && admission.target == targets[index].0
                    && admission.task == targets[index].1
                    && admission.image == targets[index].2
                    && admission.status == RuntimeAdmissionStatus::Released
                    && admission.failure.is_none()
                    && kernel.tasks().iter().any(|task| {
                        task.id == admission.task
                            && task.assignee == Some(admission.target)
                            && task.status == TaskStatus::Verified
                    })
            })
            && matches!(release_events, Some(events) if events.len() == 2
            && events.iter().enumerate().all(|(index, event)| {
                let admission = admissions[index];
                event.sequence == (first_event + index + 1) as u64
                    && event.kind == EventKind::RuntimeAdmissionReleased
                    && event.agent == admission.requester
                    && event.capability == Some(admission.authority)
                    && event.resource == Some(admission.resource)
                    && event.task == Some(admission.task)
                    && event.target_agent == Some(admission.target)
                    && event.agent_image == Some(admission.image)
                    && event.runtime_admission == Some(admission.id)
            }))
    }
}
