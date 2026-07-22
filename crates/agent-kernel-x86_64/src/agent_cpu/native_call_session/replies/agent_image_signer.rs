//! Acknowledgement binding one authenticated typed message to a rotation.

use agent_kernel_core::AgentImageSignerRotation;
use agent_kernel_x86_64::{
    agent_call::AgentCallRequest, typed_call_data::AgentImageSignerRotationMessage,
};

use super::{PendingAgentCallCpu, ResumableAgentCpu};

impl PendingAgentCallCpu {
    pub(crate) fn acknowledge_agent_image_signer_rotation(
        mut self,
        rotation: AgentImageSignerRotation,
        message: AgentImageSignerRotationMessage,
        signer_count: usize,
    ) -> Option<ResumableAgentCpu> {
        let nonce = self.authenticated_nonce_for(|request| {
            matches!(
                request,
                AgentCallRequest::RotateAgentImageSignerFromMemory { generation, .. }
                    if generation == message.generation()
                        && rotation.previous().signer_id == message.previous_signer_id()
                        && rotation.replacement().public_key == message.replacement_public_key()
                        && rotation.generation()
                            == message.expected_policy_generation().checked_add(1).unwrap_or(0)
            )
        })?;
        self.session
            .context
            .encode_agent_image_signer_rotation_reply(
                self.session.frame.frame_mut(),
                nonce,
                rotation.generation(),
                signer_count,
            )
            .ok()?;
        Some(ResumableAgentCpu(self.session))
    }
}
