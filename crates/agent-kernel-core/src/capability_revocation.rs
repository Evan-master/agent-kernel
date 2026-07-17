//! Trusted and source-authorized capability revocation.
//!
//! This core module owns atomic revocation transitions. Trusted kernel callers
//! may revoke one capability directly, while Agent-facing callers must prove
//! ownership of an active, root-scoped `Delegate` source and a direct child.

use crate::{AgentId, CapabilityId, Event, EventKind, KernelCore, KernelError, Operation};

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
    pub fn revoke_capability(&mut self, capability: CapabilityId) -> Result<(), KernelError> {
        let cap = self.find_capability(capability)?;
        self.ensure_event_slots(1)?;

        self.find_capability_mut(capability)?.revoked = true;
        self.record_capability_event(
            EventKind::CapabilityRevoked,
            cap.agent,
            cap.resource,
            capability,
            None,
            cap.operations,
            cap.task,
            None,
            None,
        )?;
        Ok(())
    }

    pub fn revoke_derived_capability(
        &mut self,
        actor: AgentId,
        source_capability: CapabilityId,
        target_capability: CapabilityId,
    ) -> Result<Event, KernelError> {
        self.ensure_agent_active(actor)?;
        let source = self.find_capability(source_capability)?;
        self.find_resource(source.resource)?;
        self.ensure_capability_chain_active(source)?;
        if source.agent != actor {
            return Err(KernelError::AgentMismatch);
        }
        if source.task.is_some() {
            return Err(KernelError::CapabilityScopeMismatch);
        }
        if !source.operations.allows(Operation::Delegate) {
            return Err(KernelError::OperationDenied);
        }

        let target = self.find_capability(target_capability)?;
        if target.revoked {
            return Err(KernelError::CapabilityRevoked);
        }
        if target.parent != Some(source_capability) {
            return Err(KernelError::CapabilityScopeMismatch);
        }
        if target.resource != source.resource {
            return Err(KernelError::ResourceMismatch);
        }
        self.ensure_event_slots(1)?;

        self.find_capability_mut(target_capability)?.revoked = true;
        self.record_capability_event(
            EventKind::CapabilityRevoked,
            actor,
            target.resource,
            target_capability,
            Some(source_capability),
            target.operations,
            target.task,
            None,
            Some(target.agent),
        )
    }
}
