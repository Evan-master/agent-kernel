//! Driver-call acknowledgements and terminal CPU ownership transfer.
//!
//! This CPU-session child authenticates only Driver-scoped requests. Core
//! mutation and physical command execution must already be complete before a
//! reply is encoded or the session becomes terminal.

use agent_kernel_core::{
    DeviceEventId, DeviceEventKind, DeviceEventPayload, DriverBindingId, DriverCommandId,
    DriverCommandResult, ResourceId,
};
use agent_kernel_x86_64::{
    agent_call::AgentCallRequest, runtime_reclamation::RuntimeReclamationLog,
};

use super::{CompletedAgentCpu, PendingAgentCallCpu, ResumableAgentCpu};

impl PendingAgentCallCpu {
    pub(crate) fn acknowledge_driver_invocation(
        mut self,
        event: DeviceEventId,
        resource: ResourceId,
        binding: DriverBindingId,
        kind: DeviceEventKind,
        payload: DeviceEventPayload,
    ) -> Option<ResumableAgentCpu> {
        let nonce = self.authenticated_nonce_for(|request| {
            matches!(request, AgentCallRequest::InspectDriverInvocation { .. })
        })?;
        self.session
            .context
            .encode_driver_invocation_reply(
                self.session.frame.frame_mut(),
                nonce,
                event,
                resource,
                binding,
                kind,
                payload,
            )
            .ok()?;
        Some(ResumableAgentCpu(self.session))
    }

    pub(crate) fn acknowledge_device_event(
        mut self,
        event: DeviceEventId,
    ) -> Option<ResumableAgentCpu> {
        let nonce = self.authenticated_nonce_for(|request| {
            matches!(request, AgentCallRequest::AcknowledgeDeviceEvent { .. })
        })?;
        self.session
            .context
            .encode_device_event_acknowledgement_reply(self.session.frame.frame_mut(), nonce, event)
            .ok()?;
        Some(ResumableAgentCpu(self.session))
    }

    pub(crate) fn acknowledge_driver_command(
        mut self,
        command: DriverCommandId,
        result: DriverCommandResult,
    ) -> Option<ResumableAgentCpu> {
        let nonce = self.authenticated_nonce_for(|request| {
            matches!(request, AgentCallRequest::SubmitDriverCommand { .. })
        })?;
        self.session
            .context
            .encode_driver_command_reply(self.session.frame.frame_mut(), nonce, command, result)
            .ok()?;
        Some(ResumableAgentCpu(self.session))
    }

    pub(crate) fn complete_driver(self) -> Option<CompletedAgentCpu> {
        let nonce = self.authenticated_nonce_for(|request| {
            matches!(request, AgentCallRequest::CompleteDriverInvocation { .. })
        })?;
        if self.session.context.driver_invocation().is_none()
            || !self.session.memory.runtime_memory_is_clear()
        {
            return None;
        }
        let physical_quantum_generation = self.session.memory.physical_quantum_generation();
        let restart_generation = self.session.memory.restart_generation();
        let lazy_data_byte = self.session.memory.lazy_data_byte();
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
            reclamation: RuntimeReclamationLog::new(),
        })
    }
}
