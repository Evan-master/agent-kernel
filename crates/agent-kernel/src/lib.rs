#![cfg_attr(not(test), no_std)]
//! Syscall-style facade for the Agent Kernel prototype.
//!
//! This crate owns the kernel boundary over `agent-kernel-core`. It exposes
//! deterministic methods that a user-space supervisor can call without
//! reaching into core state directly.

use agent_kernel_core::{
    ActionId, AgentId, CapabilityId, CheckpointId, Event, KernelCore, KernelError, Operation,
    OperationSet, ResourceId, ResourceKind, TaskId,
};

#[derive(Debug)]
pub struct AgentKernel<const RESOURCES: usize, const CAPS: usize, const EVENTS: usize> {
    core: KernelCore<RESOURCES, CAPS, EVENTS>,
}

impl<const RESOURCES: usize, const CAPS: usize, const EVENTS: usize>
    AgentKernel<RESOURCES, CAPS, EVENTS>
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
        self.core
            .authorize(agent, capability, resource, Operation::Observe)
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

    pub fn events(&self) -> &[Event] {
        self.core.events()
    }

    pub fn sys_delegate(
        &mut self,
        agent: AgentId,
        capability: CapabilityId,
        task: TaskId,
        resource: ResourceId,
        target_agent: AgentId,
    ) -> Result<Event, KernelError> {
        self.core
            .delegate(agent, capability, task, resource, target_agent)
    }
}

impl<const RESOURCES: usize, const CAPS: usize, const EVENTS: usize> Default
    for AgentKernel<RESOURCES, CAPS, EVENTS>
{
    fn default() -> Self {
        Self::new()
    }
}
