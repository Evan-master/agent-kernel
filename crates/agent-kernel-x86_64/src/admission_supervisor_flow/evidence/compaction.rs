//! Active-store and audit proof for Supervisor-triggered prefix compaction.

use agent_kernel_core::{
    EventKind, KernelError, Operation, RuntimeAdmissionId, RuntimeAdmissionStatus,
};

use super::{admission_matches, AdmissionTarget};
use crate::{
    admission_supervisor_flow::{PreparedAdmissionSupervisorFlow, ADMISSION_SUPERVISOR},
    X86BootedKernel,
};

impl PreparedAdmissionSupervisorFlow {
    pub(super) fn first_batch_compacted(
        &self,
        booted: &X86BootedKernel,
        targets: [AdmissionTarget; 4],
    ) -> bool {
        let kernel = booted.kernel();
        let admissions = kernel.runtime_admissions();
        let mut events = kernel
            .events()
            .iter()
            .filter(|event| event.kind == EventKind::RuntimeAdmissionCompacted);
        let Some(first) = events.next() else {
            return false;
        };
        let Some(second) = events.next() else {
            return false;
        };

        admissions.len() == 2
            && admissions.iter().enumerate().all(|(index, admission)| {
                admission_matches(
                    admission,
                    index + 2,
                    self.supervisor.admission_authority,
                    targets[index + 2],
                    RuntimeAdmissionStatus::Admitted,
                )
            })
            && kernel.runtime_admission(RuntimeAdmissionId::new(1))
                == Err(KernelError::RuntimeAdmissionNotFound)
            && kernel.runtime_admission(RuntimeAdmissionId::new(2))
                == Err(KernelError::RuntimeAdmissionNotFound)
            && events.next().is_none()
            && [first, second]
                .iter()
                .copied()
                .enumerate()
                .all(|(index, event)| {
                    event.sequence == first.sequence + index as u64
                        && event.agent == ADMISSION_SUPERVISOR
                        && event.capability == Some(self.supervisor.admission_authority)
                        && event.operation == Some(Operation::Delegate)
                        && event.resource.is_some()
                        && event.runtime_admission
                            == Some(RuntimeAdmissionId::new(index as u64 + 1))
                        && event.target_agent == Some(targets[index].0)
                        && event.task == Some(targets[index].1)
                        && event.agent_image == Some(targets[index].2)
                })
    }
}
