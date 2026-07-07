#![cfg_attr(not(test), no_std)]
//! no_std boot handoff boundary for Agent Kernel.
//!
//! This crate will own the deterministic handoff contract between future
//! architecture-specific boot entries and the kernel facade.

use agent_kernel::AgentKernel;
use agent_kernel_core::{
    ActionId, AgentEntryKind, AgentId, AgentImageDigest, AgentImageId, AgentImageKind,
    CapabilityId, KernelError, Operation, OperationSet, ResourceId, ResourceKind,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BootPhase {
    EnteredKernel,
    KernelInitialized,
    SupervisorHandoffReady,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct BootConfig {
    pub bootstrap_agent: AgentId,
    pub bootstrap_resource_kind: ResourceKind,
    pub boot_action: ActionId,
}

impl BootConfig {
    pub const fn new(
        bootstrap_agent: AgentId,
        bootstrap_resource_kind: ResourceKind,
        boot_action: ActionId,
    ) -> Self {
        Self {
            bootstrap_agent,
            bootstrap_resource_kind,
            boot_action,
        }
    }

    pub const fn with_boot_action(self, boot_action: ActionId) -> Self {
        Self {
            boot_action,
            ..self
        }
    }
}

impl Default for BootConfig {
    fn default() -> Self {
        Self::new(AgentId::new(1), ResourceKind::Device, ActionId::new(1))
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct BootReport {
    pub phases: [BootPhase; 3],
    pub bootstrap_agent: AgentId,
    pub bootstrap_resource: ResourceId,
    pub bootstrap_capability: CapabilityId,
    pub bootstrap_image: AgentImageId,
    pub boot_action: ActionId,
}

#[derive(Debug)]
pub struct BootedKernel<
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
> {
    kernel: AgentKernel<
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
    >,
    report: BootReport,
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
    >
    BootedKernel<
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
    >
{
    pub fn boot(config: BootConfig) -> Result<Self, KernelError> {
        let mut kernel = AgentKernel::new();
        kernel.sys_register_agent(config.bootstrap_agent)?;
        let resource = kernel.sys_register_resource(config.bootstrap_resource_kind, None)?;
        let capability = kernel.sys_grant(
            config.bootstrap_agent,
            resource,
            OperationSet::empty()
                .with(Operation::Observe)
                .with(Operation::Act)
                .with(Operation::Verify),
        )?;
        let image = kernel.sys_register_agent_image(
            config.bootstrap_agent,
            capability,
            resource,
            AgentImageKind::Bootstrap,
            AgentImageDigest::new([0; 32]),
            1,
            1,
        )?;
        kernel.sys_verify_agent_image(config.bootstrap_agent, capability, image)?;
        kernel.sys_launch_agent(
            config.bootstrap_agent,
            capability,
            resource,
            image,
            AgentEntryKind::Bootstrap,
            None,
        )?;

        kernel.sys_observe(config.bootstrap_agent, capability, resource)?;
        kernel.sys_act(
            config.bootstrap_agent,
            capability,
            config.boot_action,
            resource,
        )?;
        kernel.sys_verify(
            config.bootstrap_agent,
            capability,
            config.boot_action,
            resource,
        )?;

        Ok(Self {
            kernel,
            report: BootReport {
                phases: [
                    BootPhase::EnteredKernel,
                    BootPhase::KernelInitialized,
                    BootPhase::SupervisorHandoffReady,
                ],
                bootstrap_agent: config.bootstrap_agent,
                bootstrap_resource: resource,
                bootstrap_capability: capability,
                bootstrap_image: image,
                boot_action: config.boot_action,
            },
        })
    }

    pub const fn report(&self) -> &BootReport {
        &self.report
    }

    pub const fn kernel(
        &self,
    ) -> &AgentKernel<
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
    > {
        &self.kernel
    }
}
