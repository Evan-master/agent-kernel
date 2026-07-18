//! Semantic lifecycle for the ring-3 Runtime Admission Supervisor.

mod evidence;
mod setup;

use agent_kernel_core::{AgentId, AgentImageDigest, AgentImageId, CapabilityId, TaskId};
use agent_kernel_x86_64::{agent_call::AgentCallContext, agent_image::VerifiedAgentImage};

use crate::X86BootedKernel;

pub(super) const ADMISSION_SUPERVISOR: AgentId = AgentId::new(12);

#[derive(Copy, Clone)]
struct AdmissionSupervisorTask {
    intent: agent_kernel_core::IntentId,
    task: TaskId,
    image: AgentImageId,
    task_capability: CapabilityId,
    admission_authority: CapabilityId,
}

pub(super) struct PreparedAdmissionSupervisorFlow {
    supervisor: AdmissionSupervisorTask,
}

impl PreparedAdmissionSupervisorFlow {
    pub(super) fn prepare(booted: &mut X86BootedKernel, digest: AgentImageDigest) -> Option<Self> {
        Some(Self {
            supervisor: setup::prepare(booted, digest)?,
        })
    }

    pub(super) const fn call_context(&self) -> Option<AgentCallContext> {
        AgentCallContext::new(
            ADMISSION_SUPERVISOR,
            self.supervisor.task,
            self.supervisor.image,
            self.supervisor.task_capability,
        )
    }

    pub(super) fn verified_image<'a>(
        &self,
        booted: &X86BootedKernel,
        bytes: &'a [u8],
    ) -> Option<VerifiedAgentImage<'a>> {
        VerifiedAgentImage::verify(
            booted.kernel().agent_image(self.supervisor.image).ok()?,
            bytes,
        )
        .ok()
    }
}
