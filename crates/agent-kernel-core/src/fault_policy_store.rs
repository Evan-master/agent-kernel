//! Fixed-capacity fault policy store and application transitions.
//!
//! This module belongs to `agent-kernel-core`. It installs deterministic
//! resource-scoped fault policies and applies their route or recover actions
//! after a task fault. It performs no allocation, host callbacks, or model
//! calls.

use crate::{
    AgentId, CapabilityId, EventKind, FaultId, FaultKind, FaultPolicyAction, FaultPolicyId,
    FaultPolicyOutcome, FaultPolicyRecord, FaultRecord, KernelCore, KernelError, MessageKind,
    MessagePayload, Operation, ResourceId, TaskStatus,
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
    >
{
    pub fn install_fault_policy(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        resource: ResourceId,
        kind: FaultKind,
        action: FaultPolicyAction,
    ) -> Result<FaultPolicyId, KernelError> {
        self.ensure_agent_active(agent)?;
        self.ensure_authorized(agent, capability, resource, Operation::Rollback)?;
        if self.find_fault_policy(resource, kind).is_ok() {
            return Err(KernelError::FaultPolicyAlreadyExists);
        }
        if self.fault_policy_len >= FAULT_POLICIES {
            return Err(KernelError::FaultPolicyStoreFull);
        }
        self.ensure_event_slots(1)?;

        let id = FaultPolicyId::new(self.next_fault_policy);
        self.next_fault_policy += 1;
        self.fault_policies[self.fault_policy_len] = FaultPolicyRecord {
            id,
            resource,
            kind,
            installer: agent,
            action,
        };
        self.fault_policy_len += 1;
        self.record_fault_policy_install_event(agent, capability, resource, id, action)?;
        Ok(id)
    }

    pub fn apply_fault_policy(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        fault: FaultId,
    ) -> Result<FaultPolicyOutcome, KernelError> {
        self.ensure_agent_active(agent)?;
        let fault_record = self.find_policy_fault_record(fault)?;
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
        let policy = self.find_fault_policy(fault_record.resource, fault_record.kind)?;

        match policy.action {
            FaultPolicyAction::RouteToHandler => {
                let handler = self.find_fault_handler(fault_record.resource, fault_record.kind)?;
                self.ensure_agent_active(handler.handler)?;
                self.ensure_message_capacity()?;
                self.ensure_event_slots(3)?;

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
                self.record_fault_route_event(
                    agent,
                    capability,
                    fault_record,
                    handler.handler,
                    message,
                )?;
                let event = self.record_fault_policy_apply_event(
                    agent,
                    capability,
                    policy.id,
                    policy.action,
                    fault_record,
                    Some(message),
                    Some(handler.handler),
                )?;
                Ok(FaultPolicyOutcome {
                    action: policy.action,
                    message: Some(message),
                    event,
                })
            }
            FaultPolicyAction::RecoverTask => {
                self.ensure_event_slots(2)?;
                self.find_task_mut(fault_record.task)?.status = TaskStatus::Accepted;
                self.clear_execution_context_for_task(fault_record.task);
                self.record_fault_event(
                    EventKind::TaskFaultRecovered,
                    agent,
                    Some(capability),
                    fault_record.task,
                    fault,
                    fault_record.kind,
                    fault_record.detail,
                )?;
                let event = self.record_fault_policy_apply_event(
                    agent,
                    capability,
                    policy.id,
                    policy.action,
                    fault_record,
                    None,
                    None,
                )?;
                Ok(FaultPolicyOutcome {
                    action: policy.action,
                    message: None,
                    event,
                })
            }
        }
    }

    pub fn fault_policies(&self) -> &[FaultPolicyRecord] {
        &self.fault_policies[..self.fault_policy_len]
    }

    fn find_policy_fault_record(&self, fault: FaultId) -> Result<FaultRecord, KernelError> {
        self.faults()
            .iter()
            .find(|record| record.id == fault)
            .copied()
            .ok_or(KernelError::TaskStatusMismatch)
    }

    fn find_fault_policy(
        &self,
        resource: ResourceId,
        kind: FaultKind,
    ) -> Result<FaultPolicyRecord, KernelError> {
        self.fault_policies()
            .iter()
            .find(|record| record.resource == resource && record.kind == kind)
            .copied()
            .ok_or(KernelError::FaultPolicyNotFound)
    }
}
