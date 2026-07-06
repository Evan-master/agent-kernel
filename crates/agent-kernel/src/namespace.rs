//! Namespace syscall facade.
//!
//! This module belongs to the `agent-kernel` facade. It exposes syscall-style
//! wrappers for native object namespace operations while delegating all
//! authorization, fixed-capacity storage, and event recording to
//! `agent-kernel-core`.

use agent_kernel_core::{
    AgentId, CapabilityId, Event, KernelError, NamespaceEntryId, NamespaceEntryRecord,
    NamespaceKey, NamespaceObject, ResourceId,
};

use crate::AgentKernel;

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
    >
    AgentKernel<
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
    >
{
    pub fn sys_bind_namespace_entry(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        namespace: ResourceId,
        key: NamespaceKey,
        object: NamespaceObject,
    ) -> Result<NamespaceEntryId, KernelError> {
        self.core
            .bind_namespace_entry(agent, capability, namespace, key, object)
    }

    pub fn sys_resolve_namespace_entry(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        namespace: ResourceId,
        key: NamespaceKey,
    ) -> Result<NamespaceObject, KernelError> {
        self.core
            .resolve_namespace_entry(agent, capability, namespace, key)
    }

    pub fn sys_rebind_namespace_entry(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        entry: NamespaceEntryId,
        object: NamespaceObject,
    ) -> Result<Event, KernelError> {
        self.core
            .rebind_namespace_entry(agent, capability, entry, object)
    }

    pub fn namespace_entries(&self) -> &[NamespaceEntryRecord] {
        self.core.namespace_entries()
    }
}
