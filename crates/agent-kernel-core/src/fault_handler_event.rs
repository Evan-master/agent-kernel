//! Fault handler event builders.
//!
//! This module belongs to `agent-kernel-core`. It keeps replayable handler
//! install and route event construction separate from the routing state machine
//! while preserving no_std fixed-field event records.

use crate::{
    AgentId, CapabilityId, Event, EventKind, FaultKind, FaultRecord, KernelCore, KernelError,
    MessageId, OperationSet, ResourceId, VerificationRequirement,
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
    >
{
    pub(crate) fn record_fault_handler_event(
        &mut self,
        kind: EventKind,
        agent: AgentId,
        capability: Option<CapabilityId>,
        resource: ResourceId,
        fault_kind: FaultKind,
        handler: AgentId,
    ) -> Result<Event, KernelError> {
        self.record(Event {
            sequence: 0,
            agent,
            kind,
            resource: Some(resource),
            capability,
            source_capability: None,
            intent: None,
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
            task: None,
            task_ticks: None,
            task_quantum: None,
            fault: None,
            fault_kind: Some(fault_kind),
            fault_detail: None,
            target_agent: Some(handler),
        })
    }

    pub(crate) fn record_fault_route_event(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        fault_record: FaultRecord,
        handler: AgentId,
        message: MessageId,
    ) -> Result<Event, KernelError> {
        self.record(Event {
            sequence: 0,
            agent,
            kind: EventKind::FaultRouted,
            resource: Some(fault_record.resource),
            capability: Some(capability),
            source_capability: None,
            intent: None,
            intent_kind: None,
            action: None,
            observation: None,
            message: Some(message),
            memory_cell: None,
            namespace_entry: None,
            namespace_key: None,
            namespace_object: None,
            operation: None,
            operations: OperationSet::empty(),
            verification: VerificationRequirement::Optional,
            checkpoint: None,
            task: Some(fault_record.task),
            task_ticks: None,
            task_quantum: None,
            fault: Some(fault_record.id),
            fault_kind: Some(fault_record.kind),
            fault_detail: Some(fault_record.detail),
            target_agent: Some(handler),
        })
    }
}
