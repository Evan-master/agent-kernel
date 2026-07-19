//! Operation-specific acknowledgements for a generic native call session.
//!
//! Each conversion authenticates the pending request and writes only the reply
//! associated with that operation. Scheduler and mailbox mutations must have
//! completed before callers invoke these methods.

mod agent_entry_retirement;
mod agent_management;
mod capability_compaction;
mod intent_compaction;
mod memory_page;
mod memory_region;
mod runtime_admission;
mod task_compaction;
mod task_lifecycle;

use agent_kernel_core::{
    CapabilityId, MessageId, MessageRecord, ResourceCreateOutcome, ResourceId, TaskResult, WaiterId,
};
use agent_kernel_x86_64::{
    agent_call::AgentCallRequest, runtime_reclamation::RuntimeReclamationLog,
};

use super::{CompletedAgentCpu, PendingAgentCallCpu, ResumableAgentCpu, WaitingAgentCallCpu};

impl PendingAgentCallCpu {
    pub(crate) fn acknowledge_describe(mut self) -> Option<ResumableAgentCpu> {
        let AgentCallRequest::DescribeContext { nonce } = self.request else {
            return None;
        };
        if self.session.progress.nonce.is_some() || self.call_count() != 1 {
            return None;
        }
        self.session
            .context
            .encode_describe_reply(self.session.frame.frame_mut(), nonce)
            .ok()?;
        self.session.progress.nonce = Some(nonce);
        Some(ResumableAgentCpu(self.session))
    }

    pub(crate) fn acknowledge_task_result(mut self) -> Option<ResumableAgentCpu> {
        let nonce = self.authenticated_nonce_for(|request| {
            matches!(request, AgentCallRequest::SubmitTaskResult { .. })
        })?;
        self.session
            .context
            .encode_task_result_reply(self.session.frame.frame_mut(), nonce)
            .ok()?;
        Some(ResumableAgentCpu(self.session))
    }

    pub(crate) fn acknowledge_yield(mut self) -> Option<ResumableAgentCpu> {
        let nonce = self
            .authenticated_nonce_for(|request| matches!(request, AgentCallRequest::Yield { .. }))?;
        self.session
            .context
            .encode_yield_reply(self.session.frame.frame_mut(), nonce)
            .ok()?;
        Some(ResumableAgentCpu(self.session))
    }

    pub(crate) fn acknowledge_task_inspection(
        mut self,
        result: TaskResult,
    ) -> Option<ResumableAgentCpu> {
        let nonce = self.authenticated_nonce_for(|request| {
            matches!(request, AgentCallRequest::InspectTaskResult { .. })
        })?;
        self.session
            .context
            .encode_task_result_inspection_reply(self.session.frame.frame_mut(), nonce, result)
            .ok()?;
        Some(ResumableAgentCpu(self.session))
    }

    pub(crate) fn acknowledge_task_verification(mut self) -> Option<ResumableAgentCpu> {
        let nonce = self.authenticated_nonce_for(|request| {
            matches!(request, AgentCallRequest::VerifyTask { .. })
        })?;
        self.session
            .context
            .encode_task_verification_reply(self.session.frame.frame_mut(), nonce)
            .ok()?;
        Some(ResumableAgentCpu(self.session))
    }

    pub(crate) fn acknowledge_message_send(
        mut self,
        message: MessageId,
    ) -> Option<ResumableAgentCpu> {
        let nonce = self.authenticated_nonce_for(|request| {
            matches!(request, AgentCallRequest::SendMessage { .. })
        })?;
        self.session
            .context
            .encode_message_send_reply(self.session.frame.frame_mut(), nonce, message)
            .ok()?;
        Some(ResumableAgentCpu(self.session))
    }

    pub(crate) fn acknowledge_message_receive(
        mut self,
        message: MessageRecord,
    ) -> Option<ResumableAgentCpu> {
        let nonce = self.authenticated_nonce_for(|request| {
            matches!(request, AgentCallRequest::ReceiveMessage { .. })
        })?;
        self.session
            .context
            .encode_message_receive_reply(self.session.frame.frame_mut(), nonce, message)
            .ok()?;
        Some(ResumableAgentCpu(self.session))
    }

    pub(crate) fn acknowledge_message(mut self) -> Option<ResumableAgentCpu> {
        let nonce = self.authenticated_nonce_for(|request| {
            matches!(request, AgentCallRequest::AcknowledgeMessage { .. })
        })?;
        self.session
            .context
            .encode_message_acknowledgement_reply(self.session.frame.frame_mut(), nonce)
            .ok()?;
        Some(ResumableAgentCpu(self.session))
    }

    pub(crate) fn acknowledge_message_retirement(
        mut self,
        message: MessageId,
    ) -> Option<ResumableAgentCpu> {
        let nonce = self.authenticated_nonce_for(|request| {
            matches!(request, AgentCallRequest::RetireMessage { .. })
        })?;
        self.session
            .context
            .encode_message_retirement_reply(self.session.frame.frame_mut(), nonce, message)
            .ok()?;
        Some(ResumableAgentCpu(self.session))
    }

    pub(crate) fn acknowledge_resource_created(
        mut self,
        outcome: ResourceCreateOutcome,
    ) -> Option<ResumableAgentCpu> {
        let nonce = self.authenticated_nonce_for(|request| {
            matches!(request, AgentCallRequest::CreateResource { .. })
        })?;
        self.session
            .context
            .encode_resource_created_reply(self.session.frame.frame_mut(), nonce, outcome)
            .ok()?;
        Some(ResumableAgentCpu(self.session))
    }

    pub(crate) fn acknowledge_resource_retired(
        mut self,
        resource: ResourceId,
        capability: CapabilityId,
    ) -> Option<ResumableAgentCpu> {
        let nonce = self.authenticated_nonce_for(|request| {
            matches!(request, AgentCallRequest::RetireResource { .. })
        })?;
        self.session
            .context
            .encode_resource_retired_reply(
                self.session.frame.frame_mut(),
                nonce,
                resource,
                capability,
            )
            .ok()?;
        Some(ResumableAgentCpu(self.session))
    }

    pub(crate) fn acknowledge_capability_derived(
        mut self,
        capability: CapabilityId,
    ) -> Option<ResumableAgentCpu> {
        let nonce = self.authenticated_nonce_for(|request| {
            matches!(request, AgentCallRequest::DeriveCapability { .. })
        })?;
        self.session
            .context
            .encode_capability_derived_reply(self.session.frame.frame_mut(), nonce, capability)
            .ok()?;
        Some(ResumableAgentCpu(self.session))
    }

    pub(crate) fn acknowledge_capability_revoked(
        mut self,
        source: CapabilityId,
        target: CapabilityId,
    ) -> Option<ResumableAgentCpu> {
        let nonce = self.authenticated_nonce_for(|request| {
            matches!(request, AgentCallRequest::RevokeDerivedCapability { .. })
        })?;
        self.session
            .context
            .encode_capability_revoked_reply(self.session.frame.frame_mut(), nonce, source, target)
            .ok()?;
        Some(ResumableAgentCpu(self.session))
    }

    pub(crate) fn wait(self, waiter: WaiterId) -> Option<WaitingAgentCallCpu> {
        (waiter.raw() != 0
            && self
                .authenticated_request()
                .is_some_and(|request| matches!(request, AgentCallRequest::ReceiveMessage { .. }))
            && self.session.memory.agent_call_is_released())
        .then_some(WaitingAgentCallCpu {
            pending: self,
            waiter,
        })
    }

    pub(crate) fn complete(self, reclamation: RuntimeReclamationLog) -> Option<CompletedAgentCpu> {
        let nonce = self.authenticated_nonce_for(|request| {
            matches!(request, AgentCallRequest::CompleteTask { .. })
        })?;
        if !self.session.memory.runtime_memory_is_clear() {
            return None;
        }
        let runtime_page_generation = self.session.memory.runtime_page_generation();
        let runtime_page_released = self
            .session
            .memory
            .runtime_page_released(runtime_page_generation);
        let runtime_page_observation = self.session.memory.runtime_page_observation();
        let runtime_region_generation = self.session.memory.runtime_region_generation();
        let runtime_regions_released = self
            .session
            .memory
            .runtime_regions_released(runtime_region_generation);
        let runtime_region_observations = self.session.memory.runtime_region_observations();
        let physical_quantum_generation = self.session.memory.physical_quantum_generation();
        let restart_generation = self.session.memory.restart_generation();
        let lazy_data_byte = self.session.memory.lazy_data_byte();
        Some(CompletedAgentCpu {
            memory: self.session.memory,
            context: self.session.context,
            nonce,
            transcript: self.session.progress.transcript,
            physical_quantum_generation,
            restart_generation,
            lazy_data_byte,
            runtime_page_generation,
            runtime_page_released,
            runtime_page_observation,
            runtime_region_generation,
            runtime_regions_released,
            runtime_region_observations,
            reclamation,
        })
    }

    fn authenticated_nonce_for(
        &self,
        matches_operation: impl FnOnce(AgentCallRequest) -> bool,
    ) -> Option<u64> {
        let request = self.authenticated_request()?;
        matches_operation(request).then_some(self.session.progress.nonce?)
    }
}

impl WaitingAgentCallCpu {
    pub(crate) fn acknowledge_message_receive(
        self,
        message: MessageRecord,
    ) -> Option<ResumableAgentCpu> {
        self.pending.acknowledge_message_receive(message)
    }
}
