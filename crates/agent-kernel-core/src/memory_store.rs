//! Fixed-capacity native memory cell store.
//!
//! This module owns deterministic remember/recall behavior for
//! `agent-kernel-core`. It requires explicit capabilities, scopes cells to
//! `ResourceKind::Memory`, records every successful recall and write, and keeps
//! all failure paths atomic with respect to both memory cells and the event log.

use crate::{
    AgentId, CapabilityId, Event, EventKind, KernelCore, KernelError, MemoryCellId,
    MemoryCellRecord, MemoryValue, Operation, OperationSet, ResourceId, ResourceKind,
    VerificationRequirement,
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
    pub fn create_memory_cell(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        resource: ResourceId,
        value: MemoryValue,
    ) -> Result<MemoryCellId, KernelError> {
        self.ensure_authorized(agent, capability, resource, Operation::Act)?;
        self.ensure_memory_resource(resource)?;
        if self.memory_cell_len >= MEMORY_CELLS {
            return Err(KernelError::MemoryCellStoreFull);
        }
        self.ensure_event_slots(1)?;

        let cell = MemoryCellId::new(self.next_memory_cell);
        self.next_memory_cell += 1;
        self.memory_cells[self.memory_cell_len] = MemoryCellRecord {
            id: cell,
            resource,
            creator: agent,
            last_writer: agent,
            value,
            revision: 1,
        };
        self.memory_cell_len += 1;
        self.record_memory_event(
            EventKind::MemoryCellCreated,
            agent,
            capability,
            resource,
            cell,
            Operation::Act,
        )?;
        Ok(cell)
    }

    pub fn recall_memory_cell(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        cell: MemoryCellId,
    ) -> Result<MemoryValue, KernelError> {
        self.ensure_agent_active(agent)?;
        let record = self.find_memory_cell(cell)?;
        self.ensure_authorized(agent, capability, record.resource, Operation::Observe)?;
        self.ensure_event_slots(1)?;

        self.record_memory_event(
            EventKind::MemoryCellRecalled,
            agent,
            capability,
            record.resource,
            cell,
            Operation::Observe,
        )?;
        Ok(record.value)
    }

    pub fn remember_memory_cell(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        cell: MemoryCellId,
        value: MemoryValue,
    ) -> Result<Event, KernelError> {
        self.ensure_agent_active(agent)?;
        let record = self.find_memory_cell(cell)?;
        self.ensure_authorized(agent, capability, record.resource, Operation::Act)?;
        self.ensure_event_slots(1)?;

        let memory_cell = self.find_memory_cell_mut(cell)?;
        memory_cell.value = value;
        memory_cell.revision += 1;
        memory_cell.last_writer = agent;
        self.record_memory_event(
            EventKind::MemoryCellRemembered,
            agent,
            capability,
            record.resource,
            cell,
            Operation::Act,
        )
    }

    pub fn memory_cells(&self) -> &[MemoryCellRecord] {
        &self.memory_cells[..self.memory_cell_len]
    }

    fn ensure_memory_resource(&self, resource: ResourceId) -> Result<(), KernelError> {
        if self.find_resource(resource)?.kind == ResourceKind::Memory {
            Ok(())
        } else {
            Err(KernelError::ResourceKindMismatch)
        }
    }

    pub(crate) fn find_memory_cell(
        &self,
        id: MemoryCellId,
    ) -> Result<MemoryCellRecord, KernelError> {
        for cell in self.memory_cells() {
            if cell.id == id {
                return Ok(*cell);
            }
        }

        Err(KernelError::MemoryCellNotFound)
    }

    fn find_memory_cell_mut(
        &mut self,
        id: MemoryCellId,
    ) -> Result<&mut MemoryCellRecord, KernelError> {
        for cell in &mut self.memory_cells[..self.memory_cell_len] {
            if cell.id == id {
                return Ok(cell);
            }
        }

        Err(KernelError::MemoryCellNotFound)
    }

    fn record_memory_event(
        &mut self,
        kind: EventKind,
        agent: AgentId,
        capability: CapabilityId,
        resource: ResourceId,
        cell: MemoryCellId,
        operation: Operation,
    ) -> Result<Event, KernelError> {
        self.record(Event {
            sequence: 0,
            agent,
            kind,
            resource: Some(resource),
            capability: Some(capability),
            source_capability: None,
            intent: None,
            intent_kind: None,
            action: None,
            observation: None,
            message: None,
            message_kind: None,
            memory_cell: Some(cell),
            namespace_entry: None,
            namespace_key: None,
            namespace_object: None,
            operation: Some(operation),
            operations: OperationSet::empty(),
            verification: VerificationRequirement::Optional,
            checkpoint: None,
            task: None,
            runtime_admission: None,
            task_result: None,
            task_ticks: None,
            task_quantum: None,
            fault: None,
            fault_kind: None,
            fault_detail: None,
            fault_policy: None,
            fault_policy_action: None,
            waiter: None,
            waiter_kind: None,
            signal: None,
            target_agent: None,
            driver_binding: None,
            device_event: None,
            device_event_kind: None,
            device_event_payload: None,
            driver_command: None,
            driver_command_kind: None,
            driver_command_payload: None,
            driver_command_result: None,
            driver_invocation: None,
            driver_invocation_ticks: None,
            driver_invocation_quantum: None,
            agent_image: None,
            agent_image_kind: None,
            agent_image_digest: None,
            agent_image_abi_version: None,
            agent_image_entry_version: None,
        })
    }
}
