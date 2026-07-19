#![cfg_attr(not(test), no_std)]
//! Syscall-style facade for the Agent Kernel prototype.
//!
//! This crate owns the kernel boundary over `agent-kernel-core`. It exposes
//! deterministic methods that a user-space supervisor can call without
//! reaching into core state directly.

mod agent;
mod agent_entry_retirement;
mod agent_image_record_retirement;
mod agent_management;
mod agent_record_retirement;
mod capability;
mod driver;
mod driver_endpoint;
mod driver_runtime;
mod fault;
mod fault_compaction;
mod intent_compaction;
mod kernel_default;
mod mailbox;
mod memory;
mod namespace;
mod resource;
mod runtime_admission;
mod scheduler;
mod signal;
mod task_compaction;
mod waiter_compaction;

use agent_kernel_core::{
    ActionId, ActionRecord, AgentId, CapabilityId, CheckpointId, CheckpointRecord, Event, Intent,
    IntentId, IntentKind, KernelCore, KernelError, ObservationRecord, ResourceId, Task, TaskId,
    TaskResult, VerificationRequirement,
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
    const DRIVER_BINDINGS: usize = 0,
    const DEVICE_EVENTS: usize = 0,
    const DRIVER_COMMANDS: usize = 0,
    const DRIVER_INVOCATIONS: usize = 0,
    const RUNTIME_ADMISSIONS: usize = TASKS,
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
        DRIVER_BINDINGS,
        DEVICE_EVENTS,
        DRIVER_COMMANDS,
        DRIVER_INVOCATIONS,
        RUNTIME_ADMISSIONS,
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
        const DRIVER_BINDINGS: usize,
        const DEVICE_EVENTS: usize,
        const DRIVER_COMMANDS: usize,
        const DRIVER_INVOCATIONS: usize,
        const RUNTIME_ADMISSIONS: usize,
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
        DRIVER_BINDINGS,
        DEVICE_EVENTS,
        DRIVER_COMMANDS,
        DRIVER_INVOCATIONS,
        RUNTIME_ADMISSIONS,
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

    pub fn can_complete_task(
        &self,
        agent: AgentId,
        capability: CapabilityId,
        task: TaskId,
    ) -> Result<(), KernelError> {
        self.core.can_complete_task(agent, capability, task)
    }

    pub fn sys_submit_task_result(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        task: TaskId,
        result: TaskResult,
    ) -> Result<Event, KernelError> {
        self.core
            .submit_task_result(agent, capability, task, result)
    }

    pub fn sys_inspect_task_result(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        task: TaskId,
    ) -> Result<Event, KernelError> {
        self.core.inspect_task_result(agent, capability, task)
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

    pub fn has_event_capacity(&self, needed: usize) -> bool {
        self.core.has_event_capacity(needed)
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
