#![cfg_attr(not(test), no_std)]
//! Syscall-style facade for the Agent Kernel prototype.
//!
//! This crate owns the kernel boundary over `agent-kernel-core`. It exposes
//! deterministic methods that a user-space supervisor can call without
//! reaching into core state directly.

mod agent;
mod capability;
mod fault;
mod mailbox;
mod memory;
mod namespace;
mod resource;
mod scheduler;
mod signal;

use agent_kernel_core::{
    ActionId, ActionRecord, AgentId, CapabilityId, CheckpointId, CheckpointRecord, Event, Intent,
    IntentId, IntentKind, KernelCore, KernelError, ObservationRecord, ResourceId, Task, TaskId,
    VerificationRequirement,
};

#[derive(Debug)]
pub struct AgentKernel<
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
    const MESSAGES: usize = 0,
    const MEMORY_CELLS: usize = 0,
    const NAMESPACE_ENTRIES: usize = 0,
    const FAULTS: usize = 0,
    const FAULT_HANDLERS: usize = 0,
    const FAULT_POLICIES: usize = 0,
    const WAITERS: usize = 0,
    const AGENT_IMAGES: usize = AGENTS,
> {
    pub(crate) core: KernelCore<
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
    >,
}

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
    pub const fn new() -> Self {
        Self {
            core: KernelCore::new(),
        }
    }

    pub fn sys_observe(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        resource: ResourceId,
    ) -> Result<Event, KernelError> {
        self.core.observe(agent, capability, resource)
    }

    pub fn sys_act(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        action: ActionId,
        resource: ResourceId,
    ) -> Result<Event, KernelError> {
        self.core.act(agent, capability, action, resource)
    }

    pub fn sys_verify(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        action: ActionId,
        resource: ResourceId,
    ) -> Result<Event, KernelError> {
        self.core.verify(agent, capability, action, resource)
    }

    pub fn sys_checkpoint(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        checkpoint: CheckpointId,
        resource: ResourceId,
    ) -> Result<Event, KernelError> {
        self.core
            .checkpoint(agent, capability, checkpoint, resource)
    }

    pub fn sys_rollback(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        checkpoint: CheckpointId,
        resource: ResourceId,
    ) -> Result<Event, KernelError> {
        self.core.rollback(agent, capability, checkpoint, resource)
    }

    pub fn sys_create_task(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        intent: IntentId,
    ) -> Result<TaskId, KernelError> {
        self.core.create_task(agent, capability, intent)
    }

    pub fn sys_declare_intent(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        resource: ResourceId,
        kind: IntentKind,
        verification: VerificationRequirement,
    ) -> Result<IntentId, KernelError> {
        self.core
            .declare_intent(agent, capability, resource, kind, verification)
    }

    pub fn sys_delegate_task(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        task: TaskId,
        target_agent: AgentId,
    ) -> Result<Event, KernelError> {
        self.core
            .delegate_task(agent, capability, task, target_agent)
    }

    pub fn sys_accept_task(&mut self, agent: AgentId, task: TaskId) -> Result<Event, KernelError> {
        self.core.accept_task(agent, task)
    }

    pub fn sys_complete_task(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        task: TaskId,
    ) -> Result<Event, KernelError> {
        self.core.complete_task(agent, capability, task)
    }

    pub fn sys_verify_task(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        task: TaskId,
    ) -> Result<Event, KernelError> {
        self.core.verify_task(agent, capability, task)
    }

    pub fn sys_cancel_task(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        task: TaskId,
    ) -> Result<Event, KernelError> {
        self.core.cancel_task(agent, capability, task)
    }

    pub fn events(&self) -> &[Event] {
        self.core.events()
    }

    pub fn actions(&self) -> &[ActionRecord] {
        self.core.actions()
    }

    pub fn observations(&self) -> &[ObservationRecord] {
        self.core.observations()
    }

    pub fn checkpoints(&self) -> &[CheckpointRecord] {
        self.core.checkpoints()
    }

    pub fn intents(&self) -> &[Intent] {
        self.core.intents()
    }

    pub fn tasks(&self) -> &[Task] {
        self.core.tasks()
    }
}

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
    > Default
    for AgentKernel<
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
    fn default() -> Self {
        Self::new()
    }
}
