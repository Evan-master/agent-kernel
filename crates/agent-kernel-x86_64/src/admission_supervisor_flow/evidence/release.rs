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
        admission_id_start: usize,
        first_event: usize,
    ) -> bool {
        let kernel = booted.kernel();
        let admissions = kernel.runtime_admissions();
        let Some(batch) = admissions.get(..targets.len()) else {
            return false;
        };
        let release_events = kernel.events().get(first_event..);
        let supervisor_ready = admission_id_start == 0
            || kernel.tasks().iter().any(|task| {
                task.id == self.supervisor.task
                    && task.assignee == Some(ADMISSION_SUPERVISOR)
                    && task.status == TaskStatus::Verified
            });

        admissions.len() == if admission_id_start == 0 { 4 } else { 2 }
            && matches!(admission_id_start, 0 | 2)
            && supervisor_ready
            && batch
                .iter()
                .all(|admission| admission.status == RuntimeAdmissionStatus::Released)
            && admissions[targets.len()..]
                .iter()
                .all(|admission| admission.status == RuntimeAdmissionStatus::Requested)
            && batch.iter().enumerate().all(|(index, admission)| {
                admission_matches(
                    admission,
                    admission_id_start + index,
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
                event.sequence == events[0].sequence + index as u64
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
