//! Fixed-capacity fault handler store and routing transitions.
//!
//! This module belongs to `agent-kernel-core`. It installs deterministic
//! resource-scoped fault handlers and routes still-faulted task traps to
//! handler agents through native mailbox IPC. It performs no allocation,
//! host callbacks, or supervisor-side mutation.

use crate::{
    AgentId, CapabilityId, EventKind, FaultHandlerId, FaultHandlerRecord, FaultId, FaultKind,
    FaultRecord, KernelCore, KernelError, MessageId, MessageKind, MessagePayload, Operation,
    ResourceId, TaskStatus,
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
    pub fn install_fault_handler(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        resource: ResourceId,
        kind: FaultKind,
        handler: AgentId,
    ) -> Result<FaultHandlerId, KernelError> {
        self.ensure_agent_active(agent)?;
        self.ensure_agent_active(handler)?;
        self.ensure_authorized(agent, capability, resource, Operation::Rollback)?;
        if self.find_fault_handler(resource, kind).is_ok() {
            return Err(KernelError::FaultHandlerAlreadyExists);
        }
        if self.fault_handler_len >= FAULT_HANDLERS {
            return Err(KernelError::FaultHandlerStoreFull);
        }
        self.ensure_event_slots(1)?;

        let id = FaultHandlerId::new(self.next_fault_handler);
        self.next_fault_handler += 1;
        self.fault_handlers[self.fault_handler_len] = FaultHandlerRecord {
            id,
            resource,
            kind,
            installer: agent,
            handler,
        };
        self.fault_handler_len += 1;
        self.record_fault_handler_event(
            EventKind::FaultHandlerInstalled,
            agent,
            Some(capability),
            resource,
            kind,
            handler,
        )?;
        Ok(id)
    }

    pub fn route_fault_to_handler(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        fault: FaultId,
    ) -> Result<MessageId, KernelError> {
        self.ensure_agent_active(agent)?;
        let fault_record = self.find_fault_record(fault)?;
        let task_record = self.find_task(fault_record.task)?;
        if task_record.status != TaskStatus::Faulted || task_record.last_fault != Some(fault) {
            return Err(KernelError::TaskStatusMismatch);
        }
        self.ensure_authorized(
            agent,
            capability,
            fault_record.resource,
            Operation::Rollback,
        )?;
        let handler = self.find_fault_handler(fault_record.resource, fault_record.kind)?;
        self.ensure_agent_active(handler.handler)?;
        self.ensure_message_capacity()?;
        let waiter_index = self.find_mailbox_waiter_index(handler.handler);
        if let Some(waiter_index) = waiter_index {
            let waiter = self.waiters[waiter_index];
            self.ensure_agent_admitted_for_task(waiter.agent, waiter.task)?;
            self.ensure_not_queued(waiter.task)?;
            self.ensure_run_queue_capacity()?;
        }
        self.ensure_event_slots(2 + usize::from(waiter_index.is_some()))?;

        let message = self.append_message(
            agent,
            handler.handler,
            MessageKind::Fault,
            MessagePayload {
                resource: Some(fault_record.resource),
                capability: None,
                intent: Some(task_record.intent),
                task: Some(fault_record.task),
                action: None,
                fault: Some(fault),
            },
        );
        self.record_message_event(EventKind::MessageSent, agent, handler.handler, message)?;
        if let Some(waiter_index) = waiter_index {
            self.wake_mailbox_waiter(waiter_index, agent, message)?;
        }
        self.record_fault_route_event(agent, capability, fault_record, handler.handler, message)?;
        Ok(message)
    }

    pub fn fault_handlers(&self) -> &[FaultHandlerRecord] {
        &self.fault_handlers[..self.fault_handler_len]
    }

    fn find_fault_record(&self, fault: FaultId) -> Result<FaultRecord, KernelError> {
        self.faults()
            .iter()
            .find(|record| record.id == fault)
            .copied()
            .ok_or(KernelError::TaskStatusMismatch)
    }

    pub(crate) fn find_fault_handler(
        &self,
        resource: ResourceId,
        kind: FaultKind,
    ) -> Result<FaultHandlerRecord, KernelError> {
        self.fault_handlers()
            .iter()
            .find(|record| record.resource == resource && record.kind == kind)
            .copied()
            .ok_or(KernelError::FaultHandlerNotFound)
    }
}
