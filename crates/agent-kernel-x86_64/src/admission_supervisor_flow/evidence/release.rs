//! FIFO semantic release evidence after zeroed physical frame return.

use agent_kernel_core::{EventKind, RuntimeAdmissionStatus, TaskStatus};

use super::{admission_matches, AdmissionTarget};
use crate::{
    admission_supervisor_flow::{PreparedAdmissionSupervisorFlow, ADMISSION_SUPERVISOR},
    X86BootedKernel,
};

impl PreparedAdmissionSupervisorFlow {
    pub(crate) fn released_batch_after_reclamation(
        &self,
        booted: &X86BootedKernel,
        targets: [AdmissionTarget; 2],
        admission_start: usize,
        first_event: usize,
    ) -> bool {
        let kernel = booted.kernel();
        let admissions = kernel.runtime_admissions();
        let Some(admission_end) = admission_start.checked_add(targets.len()) else {
            return false;
        };
        let Some(batch) = admissions.get(admission_start..admission_end) else {
            return false;
        };
        let release_events = kernel.events().get(first_event..);
        let supervisor_ready = admission_start == 0
            || kernel.tasks().iter().any(|task| {
                task.id == self.supervisor.task
                    && task.assignee == Some(ADMISSION_SUPERVISOR)
                    && task.status == TaskStatus::Verified
            });

        admissions.len() == 4
            && matches!(admission_start, 0 | 2)
            && supervisor_ready
            && admissions[..admission_end]
                .iter()
                .all(|admission| admission.status == RuntimeAdmissionStatus::Released)
            && (admission_end == admissions.len()
                || admissions[admission_end..]
                    .iter()
                    .all(|admission| admission.status == RuntimeAdmissionStatus::Requested))
            && batch.iter().enumerate().all(|(index, admission)| {
                admission_matches(
                    admission,
                    admission_start + index,
                    self.supervisor.admission_authority,
                    targets[index],
                    RuntimeAdmissionStatus::Released,
                ) && kernel.tasks().iter().any(|task| {
                    task.id == admission.task
                        && task.assignee == Some(admission.target)
                        && task.status == TaskStatus::Verified
                })
            })
            && matches!(release_events, Some(events) if events.len() == 2
            && events.iter().enumerate().all(|(index, event)| {
                let admission = batch[index];
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
