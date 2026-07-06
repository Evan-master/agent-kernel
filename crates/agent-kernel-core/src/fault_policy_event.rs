//! Fault policy event builders.
//!
//! This module belongs to `agent-kernel-core`. It keeps policy install and
//! application event construction separate from policy state transitions while
//! preserving fixed-field no_std event records.

use crate::{
    AgentId, CapabilityId, Event, EventKind, FaultPolicyAction, FaultPolicyId, FaultRecord,
    KernelCore, KernelError, MessageId, OperationSet, ResourceId, TaskId, VerificationRequirement,
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
    pub(crate) fn record_fault_policy_install_event(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        resource: ResourceId,
        policy: FaultPolicyId,
        action: FaultPolicyAction,
    ) -> Result<Event, KernelError> {
        self.record_fault_policy_event(
            EventKind::FaultPolicyInstalled,
            agent,
            Some(capability),
            Some(resource),
            policy,
            action,
            None,
            None,
            None,
            None,
        )
    }

    pub(crate) fn record_fault_policy_apply_event(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        policy: FaultPolicyId,
        action: FaultPolicyAction,
        fault_record: FaultRecord,
        message: Option<MessageId>,
        target_agent: Option<AgentId>,
    ) -> Result<Event, KernelError> {
        self.record_fault_policy_event(
            EventKind::FaultPolicyApplied,
            agent,
            Some(capability),
            Some(fault_record.resource),
            policy,
            action,
            Some(fault_record),
            Some(fault_record.task),
            message,
            target_agent,
        )
    }

    fn record_fault_policy_event(
        &mut self,
        kind: EventKind,
        agent: AgentId,
        capability: Option<CapabilityId>,
        resource: Option<ResourceId>,
        policy: FaultPolicyId,
        action: FaultPolicyAction,
        fault_record: Option<FaultRecord>,
        task: Option<TaskId>,
        message: Option<MessageId>,
        target_agent: Option<AgentId>,
    ) -> Result<Event, KernelError> {
        self.record(Event {
            sequence: 0,
            agent,
            kind,
            resource,
            capability,
            source_capability: None,
            intent: None,
            intent_kind: None,
            action: None,
            observation: None,
            message,
            memory_cell: None,
            namespace_entry: None,
            namespace_key: None,
            namespace_object: None,
            operation: None,
            operations: OperationSet::empty(),
            verification: VerificationRequirement::Optional,
            checkpoint: None,
            task,
            task_ticks: None,
            task_quantum: None,
            fault: fault_record.map(|record| record.id),
            fault_kind: fault_record.map(|record| record.kind),
            fault_detail: fault_record.map(|record| record.detail),
            fault_policy: Some(policy),
            fault_policy_action: Some(action),
            waiter: None,
            signal: None,
            target_agent,
        })
    }
}
