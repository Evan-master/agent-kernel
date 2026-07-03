#![cfg_attr(not(test), no_std)]
//! Syscall-style facade for the Agent Kernel prototype.
//!
//! This crate owns the kernel boundary over `agent-kernel-core`. It exposes
//! deterministic methods that a user-space supervisor can call without
//! reaching into core state directly.

mod scheduler;

use agent_kernel_core::{
    ActionId, ActionRecord, AgentId, CapabilityId, CheckpointId, CheckpointRecord, Event, Intent,
    IntentId, IntentKind, KernelCore, KernelError, ObservationRecord, OperationSet, ResourceId,
    ResourceKind, Task, TaskId, VerificationRequirement,
};

#[derive(Debug)]
pub struct AgentKernel<
    const RESOURCES: usize,
    const CAPS: usize,
    const EVENTS: usize,
    const ACTIONS: usize,
    const OBSERVATIONS: usize,
    const CHECKPOINTS: usize,
    const INTENTS: usize,
    const TASKS: usize,
    const RUN_QUEUE: usize,
> {
    pub(crate) core: KernelCore<
        RESOURCES,
        CAPS,
        EVENTS,
        ACTIONS,
        OBSERVATIONS,
        CHECKPOINTS,
        INTENTS,
        TASKS,
        RUN_QUEUE,
    >,
}

impl<
        const RESOURCES: usize,
        const CAPS: usize,
        const EVENTS: usize,
        const ACTIONS: usize,
        const OBSERVATIONS: usize,
        const CHECKPOINTS: usize,
        const INTENTS: usize,
        const TASKS: usize,
        const RUN_QUEUE: usize,
    >
    AgentKernel<
        RESOURCES,
        CAPS,
        EVENTS,
        ACTIONS,
        OBSERVATIONS,
        CHECKPOINTS,
        INTENTS,
        TASKS,
        RUN_QUEUE,
    >
{
    pub const fn new() -> Self {
        Self {
            core: KernelCore::new(),
        }
    }

    pub fn sys_register_resource(
        &mut self,
        kind: ResourceKind,
        parent: Option<ResourceId>,
    ) -> Result<ResourceId, KernelError> {
        self.core.register_resource(kind, parent)
    }

    pub fn sys_grant(
        &mut self,
        agent: AgentId,
        resource: ResourceId,
        operations: OperationSet,
    ) -> Result<CapabilityId, KernelError> {
        self.core.grant_capability(agent, resource, operations)
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
        const RESOURCES: usize,
        const CAPS: usize,
        const EVENTS: usize,
        const ACTIONS: usize,
        const OBSERVATIONS: usize,
        const CHECKPOINTS: usize,
        const INTENTS: usize,
        const TASKS: usize,
        const RUN_QUEUE: usize,
    > Default
    for AgentKernel<
        RESOURCES,
        CAPS,
        EVENTS,
        ACTIONS,
        OBSERVATIONS,
        CHECKPOINTS,
        INTENTS,
        TASKS,
        RUN_QUEUE,
    >
{
    fn default() -> Self {
        Self::new()
    }
}
