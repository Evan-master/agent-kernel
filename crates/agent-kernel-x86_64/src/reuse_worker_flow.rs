//! Semantic batch for native workers backed by reclaimed address spaces.
//!
//! Admission uses public kernel calls and binds one immutable Capsule to each
//! Agent task. Child modules validate queued and terminal semantic evidence;
//! the boot adapter retains physical frame and CPU ownership.

mod admission;
mod evidence;

use agent_kernel_core::{AgentId, AgentImageId, CapabilityId, IntentId, RunQueueEntry, TaskId};
use agent_kernel_x86_64::{agent_call::AgentCallContext, agent_image::VerifiedAgentImage};

use crate::X86BootedKernel;

pub(super) const REUSE_WORKERS: [AgentId; 2] = [AgentId::new(10), AgentId::new(11)];

pub(super) struct PreparedReuseWorkerFlow {
    agent: AgentId,
    intent: IntentId,
    task: TaskId,
    image: AgentImageId,
    capability: CapabilityId,
}

impl PreparedReuseWorkerFlow {
    pub(super) const fn admitted_call_context(
        &self,
        requester: AgentId,
    ) -> Option<AgentCallContext> {
        AgentCallContext::new_admitted(
            self.agent,
            self.task,
            self.image,
            self.capability,
            requester,
        )
    }

    pub(super) fn verified_image<'a>(
        &self,
        booted: &X86BootedKernel,
        bytes: &'a [u8],
    ) -> Option<VerifiedAgentImage<'a>> {
        VerifiedAgentImage::verify(booted.kernel().agent_image(self.image).ok()?, bytes).ok()
    }

    pub(super) const fn admission_target(&self) -> (AgentId, TaskId, AgentImageId) {
        (self.agent, self.task, self.image)
    }

    pub(super) fn batch_queued(booted: &X86BootedKernel, flows: &[Self; 2]) -> bool {
        flows[0].agent == REUSE_WORKERS[0]
            && flows[1].agent == REUSE_WORKERS[1]
            && booted.kernel().run_queue()
                == [flows[0].run_queue_entry(), flows[1].run_queue_entry()]
    }

    pub(super) fn batch_unqueued(booted: &X86BootedKernel, flows: &[Self; 2]) -> bool {
        flows[0].agent == REUSE_WORKERS[0]
            && flows[1].agent == REUSE_WORKERS[1]
            && booted.kernel().run_queue().is_empty()
    }

    const fn run_queue_entry(&self) -> RunQueueEntry {
        RunQueueEntry {
            task: self.task,
            agent: self.agent,
        }
    }
}
