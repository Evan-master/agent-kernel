//! Supervisor-owned runtime admission request store.
//!
//! Requests bind a root Delegate capability to one accepted task, its launched
//! target Agent, and its verified image before any platform state is touched.

use crate::runtime_admission_event::runtime_admission_event;
use crate::{
    AgentEntryKind, AgentId, CapabilityId, EventKind, KernelCore, KernelError, Operation,
    RuntimeAdmissionId, RuntimeAdmissionRecord, RuntimeAdmissionStatus, Task, TaskId,
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
    pub fn request_runtime_admission(
        &mut self,
        requester: AgentId,
        authority: CapabilityId,
        target: AgentId,
        task: TaskId,
    ) -> Result<RuntimeAdmissionId, KernelError> {
        let task_record =
            self.ensure_runtime_admission_context(requester, authority, target, task)?;
        if self.runtime_admissions().iter().any(|record| {
            matches!(
                record.status,
                RuntimeAdmissionStatus::Requested | RuntimeAdmissionStatus::Admitted
            ) && (record.target == target || record.task == task)
        }) {
            return Err(KernelError::RuntimeAdmissionDuplicate);
        }
        if self.runtime_admission_len >= RUNTIME_ADMISSIONS {
            return Err(KernelError::RuntimeAdmissionStoreFull);
        }
        self.ensure_event_slots(1)?;

        let entry = self.find_agent_entry(target)?;
        let record = RuntimeAdmissionRecord {
            id: RuntimeAdmissionId::new(self.next_runtime_admission),
            requester,
            authority,
            target,
            task,
            image: entry.image,
            resource: task_record.resource,
            status: RuntimeAdmissionStatus::Requested,
            failure: None,
        };
        self.runtime_admissions[self.runtime_admission_len] = record;
        self.runtime_admission_len += 1;
        self.next_runtime_admission += 1;
        self.runtime_admission_generation += 1;
        self.record(runtime_admission_event(
            record,
            EventKind::RuntimeAdmissionRequested,
        ))?;
        Ok(record.id)
    }

    pub fn runtime_admissions(&self) -> &[RuntimeAdmissionRecord] {
        &self.runtime_admissions[..self.runtime_admission_len]
    }

    pub const fn runtime_admission_capacity(&self) -> usize {
        RUNTIME_ADMISSIONS
    }

    pub fn runtime_admission(
        &self,
        admission: RuntimeAdmissionId,
    ) -> Result<RuntimeAdmissionRecord, KernelError> {
        self.find_runtime_admission(admission)
    }

    pub(crate) fn find_runtime_admission(
        &self,
        admission: RuntimeAdmissionId,
    ) -> Result<RuntimeAdmissionRecord, KernelError> {
        self.runtime_admissions()
            .iter()
            .find(|record| record.id == admission)
            .copied()
            .ok_or(KernelError::RuntimeAdmissionNotFound)
    }

    pub(crate) fn find_runtime_admission_mut(
        &mut self,
        admission: RuntimeAdmissionId,
    ) -> Result<&mut RuntimeAdmissionRecord, KernelError> {
        self.runtime_admissions[..self.runtime_admission_len]
            .iter_mut()
            .find(|record| record.id == admission)
            .ok_or(KernelError::RuntimeAdmissionNotFound)
    }

    pub(crate) fn ensure_runtime_admission_context(
        &self,
        requester: AgentId,
        authority: CapabilityId,
        target: AgentId,
        task: TaskId,
    ) -> Result<Task, KernelError> {
        let requester_entry = self
            .find_agent_entry(requester)
            .map_err(|_| KernelError::AgentNotLaunched)?;
        if requester_entry.kind != AgentEntryKind::Supervisor {
            return Err(KernelError::AgentEntryKindMismatch);
        }

        let task_record = self.find_runnable_task(target, task)?;
        self.ensure_authorized(
            requester,
            authority,
            task_record.resource,
            Operation::Delegate,
        )?;
        let target_entry = self
            .find_agent_entry(target)
            .map_err(|_| KernelError::AgentNotLaunched)?;
        if target_entry.task != Some(task) {
            return Err(KernelError::AgentEntryScopeMismatch);
        }
        self.ensure_agent_admitted_for_task(target, task)?;
        self.ensure_launch_image(target_entry.image, task_record.resource, target_entry.kind)?;
        self.ensure_not_queued(task)?;
        Ok(task_record)
    }
}
