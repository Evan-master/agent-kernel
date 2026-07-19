//! Authenticated retirement of a terminal Runtime Admission prefix.
//!
//! This no_std core module preflights Supervisor authority, terminal status,
//! and Event capacity before compacting fixed storage. It preserves FIFO order,
//! monotonic IDs, replay evidence, and generation-bound permit safety.

use crate::runtime_admission_event::runtime_admission_compaction_event;
use crate::{
    AgentEntryKind, AgentId, CapabilityId, KernelCore, KernelError, Operation,
    RuntimeAdmissionCompaction, RuntimeAdmissionId, RuntimeAdmissionRecord, RuntimeAdmissionStatus,
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
    pub fn compact_runtime_admission_prefix(
        &mut self,
        actor: AgentId,
        authority: CapabilityId,
        through: RuntimeAdmissionId,
    ) -> Result<RuntimeAdmissionCompaction, KernelError> {
        let actor_entry = self
            .find_agent_entry(actor)
            .map_err(|_| KernelError::AgentNotLaunched)?;
        if actor_entry.kind != AgentEntryKind::Supervisor {
            return Err(KernelError::AgentEntryKindMismatch);
        }

        let through_index = self
            .runtime_admissions()
            .iter()
            .position(|record| record.id == through)
            .ok_or(KernelError::RuntimeAdmissionNotFound)?;
        let count = through_index + 1;
        for record in self.runtime_admissions()[..count].iter().copied() {
            if !matches!(
                record.status,
                RuntimeAdmissionStatus::Rejected | RuntimeAdmissionStatus::Released
            ) {
                return Err(KernelError::RuntimeAdmissionCompactionNotReady);
            }
            self.ensure_authorized(actor, authority, record.resource, Operation::Delegate)?;
        }
        self.ensure_event_slots(count)?;

        let previous = self.runtime_admissions;
        let remaining = self.runtime_admission_len - count;
        self.runtime_admissions[..remaining]
            .copy_from_slice(&previous[count..self.runtime_admission_len]);
        for index in remaining..self.runtime_admission_len {
            self.runtime_admissions[index] = RuntimeAdmissionRecord::empty();
        }
        self.runtime_admission_len = remaining;
        self.runtime_admission_generation += 1;
        for record in previous[..count].iter().copied() {
            self.record(runtime_admission_compaction_event(record, actor, authority))?;
        }

        Ok(RuntimeAdmissionCompaction::new(
            previous[0].id,
            through,
            count,
        ))
    }
}
