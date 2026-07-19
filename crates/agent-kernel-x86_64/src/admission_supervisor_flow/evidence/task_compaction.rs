//! Active Task Store and audit proof for Supervisor-triggered compaction.

use agent_kernel_core::{EventKind, KernelError, Operation, TaskId};

use crate::{
    admission_supervisor_flow::{PreparedAdmissionSupervisorFlow, ADMISSION_SUPERVISOR},
    X86BootedKernel, X86_TASK_CAPACITY,
};

impl PreparedAdmissionSupervisorFlow {
    pub(super) fn initial_task_prefix_compacted(&self, booted: &X86BootedKernel) -> bool {
        let kernel = booted.kernel();
        let mut events = kernel
            .events()
            .iter()
            .filter(|event| event.kind == EventKind::TaskCompacted);
        let Some(first) = events.next() else {
            return false;
        };
        let mut compacted = [*first; 6];
        for slot in &mut compacted[1..] {
            let Some(event) = events.next() else {
                return false;
            };
            *slot = *event;
        }

        kernel.task_capacity() == X86_TASK_CAPACITY
            && kernel.tasks().len() == 6
            && kernel
                .tasks()
                .iter()
                .enumerate()
                .all(|(index, task)| task.id == TaskId::new(index as u64 + 7))
            && (1..=6).all(|raw| kernel.task(TaskId::new(raw)) == Err(KernelError::TaskNotFound))
            && compacted.iter().enumerate().all(|(index, event)| {
                event.sequence == first.sequence + index as u64
                    && event.agent == ADMISSION_SUPERVISOR
                    && event.capability == Some(self.supervisor.admission_authority)
                    && event.operation == Some(Operation::Rollback)
                    && event.resource.is_some()
                    && event.intent.is_some()
                    && event.task == Some(TaskId::new(index as u64 + 1))
            })
            && events.next().is_none()
    }
}
