//! Semantic batch for native workers backed by reclaimed address spaces.
//!
//! Admission uses public kernel calls and binds one immutable Capsule to each
//! Agent task. Child modules validate queued and terminal semantic evidence;
//! the boot adapter retains physical frame and CPU ownership.

mod admission;
mod evidence;

use agent_kernel_core::{AgentId, AgentImageId, CapabilityId, IntentId, RunQueueEntry, TaskId};
use agent_kernel_x86_64::{
    agent_call::AgentCallContext,
    agent_image::{AgentImageTrustPolicy, VerifiedAgentImage},
};

use crate::X86BootedKernel;

pub(super) const REUSE_WORKER_BATCHES: [[AgentId; 2]; 2] = [
    [AgentId::new(10), AgentId::new(11)],
    [AgentId::new(13), AgentId::new(14)],
];
pub(super) const REUSE_WORKERS: [AgentId; 4] = [
    AgentId::new(10),
    AgentId::new(11),
    AgentId::new(13),
    AgentId::new(14),
];

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
        let policy = AgentImageTrustPolicy::new(booted.kernel().agent_image_signers());
        VerifiedAgentImage::verify_signed(
            booted.kernel().agent_image(self.image).ok()?,
            bytes,
            &policy,
        )
        .ok()
    }

    pub(super) const fn admission_target(&self) -> (AgentId, TaskId, AgentImageId) {
        (self.agent, self.task, self.image)
    }

    pub(super) fn batch_queued(booted: &X86BootedKernel, flows: &[Self; 2]) -> bool {
        Self::valid_batch(flows)
            && booted.kernel().run_queue()
                == [flows[0].run_queue_entry(), flows[1].run_queue_entry()]
    }

    pub(super) fn batch_unqueued(booted: &X86BootedKernel, flows: &[Self; 2]) -> bool {
        Self::valid_batch(flows)
            && flows.iter().all(|flow| {
                !booted
                    .kernel()
                    .run_queue()
                    .contains(&flow.run_queue_entry())
            })
    }

    fn valid_batch(flows: &[Self; 2]) -> bool {
        REUSE_WORKER_BATCHES
            .iter()
            .any(|batch| flows[0].agent == batch[0] && flows[1].agent == batch[1])
    }

    const fn run_queue_entry(&self) -> RunQueueEntry {
        RunQueueEntry {
            task: self.task,
            agent: self.agent,
        }
    }
}
