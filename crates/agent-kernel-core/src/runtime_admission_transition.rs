//! Two-phase runtime admission commit and rejection transitions.

use crate::runtime_admission_event::{runtime_admission_event, runtime_admission_queue_event};
use crate::{
    EventKind, KernelCore, KernelError, RunQueueEntry, RuntimeAdmissionFailure,
    RuntimeAdmissionPermit, RuntimeAdmissionRecord, RuntimeAdmissionStatus,
};

impl<
        const AGENTS: usize,
        const RESOURCES: usize,
        const CAPS: usize,
        const EVENTS: usize,
        const ACTIONS: usize,
        const OBSERVATIONS: usize,
        const CHECKPOINTS: usize,
        const INTENTS: usize,
        const TASKS: usize,
        const RUN_QUEUE: usize,
        const MESSAGES: usize,
        const MEMORY_CELLS: usize,
        const NAMESPACE_ENTRIES: usize,
        const FAULTS: usize,
        const FAULT_HANDLERS: usize,
        const FAULT_POLICIES: usize,
        const WAITERS: usize,
        const AGENT_IMAGES: usize,
        const DRIVER_BINDINGS: usize,
        const DEVICE_EVENTS: usize,
        const DRIVER_COMMANDS: usize,
        const DRIVER_INVOCATIONS: usize,
        const RUNTIME_ADMISSIONS: usize,
    >
    KernelCore<
        AGENTS,
        RESOURCES,
        CAPS,
        EVENTS,
        ACTIONS,
        OBSERVATIONS,
        CHECKPOINTS,
        INTENTS,
        TASKS,
        RUN_QUEUE,
        MESSAGES,
        MEMORY_CELLS,
        NAMESPACE_ENTRIES,
        FAULTS,
        FAULT_HANDLERS,
        FAULT_POLICIES,
        WAITERS,
        AGENT_IMAGES,
        DRIVER_BINDINGS,
        DEVICE_EVENTS,
        DRIVER_COMMANDS,
        DRIVER_INVOCATIONS,
        RUNTIME_ADMISSIONS,
    >
{
    pub fn prepare_next_runtime_admission(&self) -> Result<RuntimeAdmissionPermit, KernelError> {
        let record = self
            .runtime_admissions()
            .iter()
            .find(|record| record.status == RuntimeAdmissionStatus::Requested)
            .copied()
            .ok_or(KernelError::RuntimeAdmissionNotPending)?;
        self.ensure_runtime_admission_context(
            record.requester,
            record.authority,
            record.target,
            record.task,
        )?;
        self.ensure_run_queue_capacity()?;
        self.ensure_event_slots(2)?;
        Ok(RuntimeAdmissionPermit::new(
            record,
            self.runtime_admission_generation,
        ))
    }

    pub fn commit_runtime_admission(
        &mut self,
        permit: RuntimeAdmissionPermit,
    ) -> Result<RuntimeAdmissionRecord, KernelError> {
        let record = self.ensure_runtime_admission_permit(permit)?;
        let task = self.ensure_runtime_admission_context(
            record.requester,
            record.authority,
            record.target,
            record.task,
        )?;
        self.ensure_run_queue_capacity()?;
        self.ensure_event_slots(2)?;

        let mut admitted = record;
        admitted.status = RuntimeAdmissionStatus::Admitted;
        *self.find_runtime_admission_mut(record.id)? = admitted;
        self.run_queue[self.run_queue_len] = RunQueueEntry {
            task: record.task,
            agent: record.target,
        };
        self.run_queue_len += 1;
        self.runtime_admission_generation += 1;
        self.record(runtime_admission_event(
            admitted,
            EventKind::RuntimeAdmissionAdmitted,
        ))?;
        self.record(runtime_admission_queue_event(admitted, task))?;
        Ok(admitted)
    }

    pub fn reject_runtime_admission(
        &mut self,
        permit: RuntimeAdmissionPermit,
        failure: RuntimeAdmissionFailure,
    ) -> Result<RuntimeAdmissionRecord, KernelError> {
        let record = self.ensure_runtime_admission_permit(permit)?;
        self.ensure_event_slots(1)?;

        let mut rejected = record;
        rejected.status = RuntimeAdmissionStatus::Rejected;
        rejected.failure = Some(failure);
        *self.find_runtime_admission_mut(record.id)? = rejected;
        self.runtime_admission_generation += 1;
        self.record(runtime_admission_event(
            rejected,
            EventKind::RuntimeAdmissionRejected,
        ))?;
        Ok(rejected)
    }

    fn ensure_runtime_admission_permit(
        &self,
        permit: RuntimeAdmissionPermit,
    ) -> Result<RuntimeAdmissionRecord, KernelError> {
        if permit.generation() != self.runtime_admission_generation {
            return Err(KernelError::RuntimeAdmissionPermitStale);
        }
        let current = self.find_runtime_admission(permit.admission())?;
        if current.status != RuntimeAdmissionStatus::Requested || current != permit.record() {
            return Err(KernelError::RuntimeAdmissionPermitStale);
        }
        Ok(current)
    }
}
