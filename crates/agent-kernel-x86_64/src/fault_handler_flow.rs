//! Admission and decision evidence for the native ring-3 Fault Handler Agent.
//!
//! This bare-metal boot adapter owns Handler task metadata and lifecycle glue.
//! Core fault routing and mailbox state remain authoritative; page-table repair
//! accepts only the opaque approval produced by this module's terminal proof.

mod evidence;
mod setup;

use agent_kernel_core::{
    AgentId, AgentImageDigest, AgentImageId, AgentImageRecord, CapabilityId, EventKind, FaultId,
    RunQueueEntry, TaskId,
};
use agent_kernel_x86_64::agent_call::AgentCallContext;

use crate::{
    boot_agent_images::BootFaultHandlerImage, fault_task_flow::RoutedFault,
    native_agent_executor::NativeExecutionReport, X86BootedKernel,
};

pub(super) const FAULT_HANDLER: AgentId = AgentId::new(7);

#[derive(Copy, Clone)]
struct FaultHandlerTask {
    task: TaskId,
    image: AgentImageId,
    capability: CapabilityId,
}

impl FaultHandlerTask {
    const fn call_context(self) -> Option<AgentCallContext> {
        AgentCallContext::new(FAULT_HANDLER, self.task, self.image, self.capability)
    }
}

pub(super) struct FaultHandlerFlow;

pub(super) struct PreparedFaultHandlerFlow {
    handler: FaultHandlerTask,
}

pub(crate) struct ApprovedFaultRepair {
    fault: FaultId,
}

impl FaultHandlerFlow {
    pub(super) fn prepare(
        booted: &mut X86BootedKernel,
        digest: AgentImageDigest,
    ) -> Option<PreparedFaultHandlerFlow> {
        Some(PreparedFaultHandlerFlow {
            handler: setup::prepare(booted, digest)?,
        })
    }
}

impl PreparedFaultHandlerFlow {
    pub(super) fn call_context(&self) -> Option<AgentCallContext> {
        self.handler.call_context()
    }

    pub(super) fn image_record(&self, booted: &X86BootedKernel) -> Option<AgentImageRecord> {
        booted.kernel().agent_image(self.handler.image).ok()
    }

    pub(super) const fn run_queue_entry(&self) -> RunQueueEntry {
        RunQueueEntry {
            task: self.handler.task,
            agent: FAULT_HANDLER,
        }
    }

    pub(super) fn queue_for_runtime(&self, booted: &mut X86BootedKernel) -> Option<()> {
        if !booted.kernel().run_queue().is_empty() {
            return None;
        }
        let event = booted
            .kernel_mut()
            .sys_enqueue_task(FAULT_HANDLER, self.handler.task)
            .ok()?;
        (event.kind == EventKind::TaskQueued
            && event.agent == FAULT_HANDLER
            && event.task == Some(self.handler.task)
            && booted.kernel().run_queue() == [self.run_queue_entry()])
        .then_some(())
    }

    pub(super) fn waiting_after_runtime(&self, booted: &X86BootedKernel) -> bool {
        evidence::waiting(booted, self.handler)
    }

    pub(super) fn approve_after_runtime(
        &self,
        booted: &X86BootedKernel,
        report: &NativeExecutionReport,
        image: BootFaultHandlerImage,
        routed: RoutedFault,
    ) -> Option<ApprovedFaultRepair> {
        evidence::approved(booted, report, self.handler, image, routed)
    }
}

impl ApprovedFaultRepair {
    pub(crate) const fn fault(&self) -> FaultId {
        self.fault
    }
}
