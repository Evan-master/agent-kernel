//! Atomic semantic release for physically reclaimed Runtime Admissions.
//!
//! This core module prepares an opaque, generation-bound record batch and
//! commits its terminal events only after the architecture owner finishes the
//! external reclamation transaction. Every check is fixed-capacity and no_std.

use crate::runtime_admission_event::runtime_admission_event;
use crate::{
    AgentExecutionState, EventKind, KernelCore, KernelError, RuntimeAdmissionId,
    RuntimeAdmissionRecord, RuntimeAdmissionReleaseBatch, RuntimeAdmissionStatus, TaskStatus,
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
    >
{
    pub fn prepare_runtime_admission_release_batch<const COUNT: usize>(
        &self,
        admissions: [RuntimeAdmissionId; COUNT],
    ) -> Result<RuntimeAdmissionReleaseBatch<COUNT>, KernelError> {
        if COUNT == 0 {
            return Err(KernelError::RuntimeAdmissionReleaseBatchEmpty);
        }
        if COUNT > TASKS {
            return Err(KernelError::RuntimeAdmissionReleaseBatchTooLarge);
        }

        let mut records = [RuntimeAdmissionRecord::empty(); COUNT];
        for (index, admission) in admissions.iter().copied().enumerate() {
            if admission.raw() == 0 || admissions[..index].contains(&admission) {
                return Err(KernelError::RuntimeAdmissionReleaseDuplicate);
            }
            let record = self.find_runtime_admission(admission)?;
            self.ensure_runtime_admission_release_ready(record)?;
            records[index] = record;
        }
        self.ensure_event_slots(COUNT)?;

        Ok(RuntimeAdmissionReleaseBatch::new(
            records,
            self.runtime_admission_generation,
        ))
    }

    pub fn commit_runtime_admission_release_batch<const COUNT: usize>(
        &mut self,
        permit: RuntimeAdmissionReleaseBatch<COUNT>,
    ) -> Result<[RuntimeAdmissionRecord; COUNT], KernelError> {
        if permit.generation() != self.runtime_admission_generation {
            return Err(KernelError::RuntimeAdmissionReleasePermitStale);
        }
        if permit.is_empty() {
            return Err(KernelError::RuntimeAdmissionReleaseBatchEmpty);
        }

        let records = *permit.records();
        let mut indices = [0; COUNT];
        for (index, record) in records.iter().copied().enumerate() {
            if records[..index]
                .iter()
                .any(|existing| existing.id == record.id)
            {
                return Err(KernelError::RuntimeAdmissionReleaseDuplicate);
            }
            let current = self
                .find_runtime_admission(record.id)
                .map_err(|_| KernelError::RuntimeAdmissionReleasePermitStale)?;
            if current != record {
                return Err(KernelError::RuntimeAdmissionReleasePermitStale);
            }
            self.ensure_runtime_admission_release_ready(current)?;
            indices[index] = self
                .runtime_admissions()
                .iter()
                .position(|candidate| candidate.id == record.id)
                .ok_or(KernelError::RuntimeAdmissionReleasePermitStale)?;
        }
        self.ensure_event_slots(COUNT)?;

        let mut released = records;
        for (index, record) in released.iter_mut().enumerate() {
            record.status = RuntimeAdmissionStatus::Released;
            self.runtime_admissions[indices[index]] = *record;
        }
        self.runtime_admission_generation += 1;
        for record in released.iter().copied() {
            self.record(runtime_admission_event(
                record,
                EventKind::RuntimeAdmissionReleased,
            ))?;
        }
        Ok(released)
    }

    fn ensure_runtime_admission_release_ready(
        &self,
        record: RuntimeAdmissionRecord,
    ) -> Result<(), KernelError> {
        if record.status != RuntimeAdmissionStatus::Admitted || record.failure.is_some() {
            return Err(KernelError::RuntimeAdmissionReleaseNotReady);
        }
        let task = self
            .find_task(record.task)
            .map_err(|_| KernelError::RuntimeAdmissionReleaseNotReady)?;
        if task.status != TaskStatus::Verified
            || task.assignee != Some(record.target)
            || task.resource != record.resource
        {
            return Err(KernelError::RuntimeAdmissionReleaseNotReady);
        }
        let entry = self
            .find_agent_entry(record.target)
            .map_err(|_| KernelError::RuntimeAdmissionReleaseNotReady)?;
        if entry.task != Some(record.task)
            || entry.image != record.image
            || entry.resource != record.resource
        {
            return Err(KernelError::RuntimeAdmissionReleaseNotReady);
        }
        let execution = self
            .execution_context(record.target)
            .map_err(|_| KernelError::RuntimeAdmissionReleaseNotReady)?;
        if execution.state != AgentExecutionState::Idle
            || execution.task.is_some()
            || execution.driver_invocation.is_some()
        {
            return Err(KernelError::RuntimeAdmissionReleaseNotReady);
        }
        Ok(())
    }
}
