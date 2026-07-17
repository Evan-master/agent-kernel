//! Admission and terminal proof for the native ring-3 Resource Manager Agent.
//!
//! This bare-metal boot adapter owns Manager task metadata and lifecycle glue.
//! Resource and capability mutation remains inside the public kernel facade;
//! this module only queues the admitted Agent and validates final evidence.

mod evidence;
mod setup;

use agent_kernel_core::{
    AgentId, AgentImageDigest, AgentImageId, AgentImageRecord, CapabilityId, EventKind,
    RunQueueEntry, TaskId,
};
use agent_kernel_x86_64::agent_call::AgentCallContext;

use crate::{
    boot_agent_images::BootResourceManagerImage, native_agent_executor::NativeExecutionReport,
    X86BootedKernel,
};

pub(super) const RESOURCE_MANAGER: AgentId = AgentId::new(8);

#[derive(Copy, Clone)]
struct ResourceManagerTask {
    task: TaskId,
    image: AgentImageId,
    task_capability: CapabilityId,
    resource_authority: CapabilityId,
}

impl ResourceManagerTask {
    const fn call_context(self) -> Option<AgentCallContext> {
        AgentCallContext::new(
            RESOURCE_MANAGER,
            self.task,
            self.image,
            self.task_capability,
        )
    }
}

pub(super) struct ResourceManagerFlow;

pub(super) struct PreparedResourceManagerFlow {
    manager: ResourceManagerTask,
}

impl ResourceManagerFlow {
    pub(super) fn prepare(
        booted: &mut X86BootedKernel,
        digest: AgentImageDigest,
    ) -> Option<PreparedResourceManagerFlow> {
        Some(PreparedResourceManagerFlow {
            manager: setup::prepare(booted, digest)?,
        })
    }
}

impl PreparedResourceManagerFlow {
    pub(super) fn call_context(&self) -> Option<AgentCallContext> {
        self.manager.call_context()
    }

    pub(super) fn image_record(&self, booted: &X86BootedKernel) -> Option<AgentImageRecord> {
        booted.kernel().agent_image(self.manager.image).ok()
    }

    const fn run_queue_entry(&self) -> RunQueueEntry {
        RunQueueEntry {
            task: self.manager.task,
            agent: RESOURCE_MANAGER,
        }
    }

    pub(super) fn queue_for_runtime(&self, booted: &mut X86BootedKernel) -> Option<()> {
        if !booted.kernel().run_queue().is_empty() {
            return None;
        }
        let event = booted
            .kernel_mut()
            .sys_enqueue_task(RESOURCE_MANAGER, self.manager.task)
            .ok()?;
        (event.kind == EventKind::TaskQueued
            && event.agent == RESOURCE_MANAGER
            && event.task == Some(self.manager.task)
            && booted.kernel().run_queue() == [self.run_queue_entry()])
        .then_some(())
    }

    pub(super) fn completed_after_runtime(
        &self,
        booted: &X86BootedKernel,
        report: &NativeExecutionReport,
        image: BootResourceManagerImage,
    ) -> Option<()> {
        evidence::completed(booted, report, self.manager, image).then_some(())
    }
}
