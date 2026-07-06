//! Fixed-capacity task fault store and recovery transitions.
//!
//! This module belongs to `agent-kernel-core`. It records deterministic task
//! traps, moves tasks into and out of `Faulted`, and emits replayable fault
//! events. It performs no allocation, host I/O, or panic-style fault handling.

use crate::{
    AgentId, CapabilityId, Event, EventKind, FaultId, FaultKind, FaultRecord, KernelCore,
    KernelError, Operation, OperationSet, TaskId, TaskStatus, VerificationRequirement,
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
    >
{
    pub fn fault_task(
        &mut self,
        agent: AgentId,
        task: TaskId,
        kind: FaultKind,
        detail: u64,
    ) -> Result<FaultId, KernelError> {
        self.ensure_agent_active(agent)?;
        let task_record = self.find_task(task)?;
        if task_record.status != TaskStatus::Running || task_record.assignee != Some(agent) {
            return Err(KernelError::TaskNotRunnable);
        }
        self.ensure_agent_admitted_for_task(agent, task)?;
        if self.fault_len >= FAULTS {
            return Err(KernelError::FaultStoreFull);
        }
        self.ensure_event_slots(1)?;

        let fault = FaultId::new(self.next_fault);
        self.next_fault += 1;
        self.faults[self.fault_len] = FaultRecord {
            id: fault,
            task,
            agent,
            resource: task_record.resource,
            kind,
            detail,
        };
        self.fault_len += 1;

        let task_ref = self.find_task_mut(task)?;
        task_ref.status = TaskStatus::Faulted;
        task_ref.quantum_remaining = 0;
        task_ref.last_fault = Some(fault);
        self.set_execution_context_faulted(agent, task)?;
        self.record_fault_event(
            EventKind::TaskFaulted,
            agent,
            None,
            task,
            fault,
            kind,
            detail,
        )?;
        Ok(fault)
    }

    pub fn recover_faulted_task(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        task: TaskId,
    ) -> Result<Event, KernelError> {
        self.ensure_agent_active(agent)?;
        let task_record = self.find_task(task)?;
        if task_record.status != TaskStatus::Faulted {
            return Err(KernelError::TaskStatusMismatch);
        }
        self.ensure_authorized(agent, capability, task_record.resource, Operation::Rollback)?;
        let fault = task_record
            .last_fault
            .ok_or(KernelError::TaskStatusMismatch)?;
        let (kind, detail) = self.fault_metadata(fault)?;
        self.ensure_event_slots(1)?;

        self.find_task_mut(task)?.status = TaskStatus::Accepted;
        self.clear_execution_context_for_task(task);
        self.record_fault_event(
            EventKind::TaskFaultRecovered,
            agent,
            Some(capability),
            task,
            fault,
            kind,
            detail,
        )
    }

    pub fn faults(&self) -> &[FaultRecord] {
        &self.faults[..self.fault_len]
    }

    fn fault_metadata(&self, fault: FaultId) -> Result<(FaultKind, u64), KernelError> {
        self.faults()
            .iter()
            .find(|record| record.id == fault)
            .map(|record| (record.kind, record.detail))
            .ok_or(KernelError::TaskStatusMismatch)
    }

    pub(crate) fn record_fault_event(
        &mut self,
        kind: EventKind,
        agent: AgentId,
        capability: Option<CapabilityId>,
        task: TaskId,
        fault: FaultId,
        fault_kind: FaultKind,
        fault_detail: u64,
    ) -> Result<Event, KernelError> {
        let task_record = self.find_task(task)?;
        self.record(Event {
            sequence: self.next_sequence,
            agent,
            kind,
            resource: Some(task_record.resource),
            capability,
            source_capability: None,
            intent: Some(task_record.intent),
            intent_kind: None,
            action: None,
            observation: None,
            message: None,
            memory_cell: None,
            namespace_entry: None,
            namespace_key: None,
            namespace_object: None,
            operation: None,
            operations: OperationSet::empty(),
            verification: VerificationRequirement::Optional,
            checkpoint: None,
            task: Some(task),
            task_ticks: None,
            task_quantum: None,
            fault: Some(fault),
            fault_kind: Some(fault_kind),
            fault_detail: Some(fault_detail),
            fault_policy: None,
            fault_policy_action: None,
            waiter: None,
            signal: None,
            target_agent: None,
        })
    }
}
