//! MemoryCell record retirement acknowledgement for an authenticated call.

use agent_kernel_core::MemoryCellRecordRetirement;
use agent_kernel_x86_64::agent_call::AgentCallRequest;

use super::{PendingAgentCallCpu, ResumableAgentCpu};

impl PendingAgentCallCpu {
    pub(crate) fn acknowledge_memory_cell_record_retirement(
        mut self,
        receipt: MemoryCellRecordRetirement,
    ) -> Option<ResumableAgentCpu> {
        let nonce = self.authenticated_nonce_for(|request| {
            matches!(
                request,
                AgentCallRequest::RetireMemoryCellRecord { target, .. }
                    if target == receipt.memory_cell()
            )
        })?;
        self.session
            .context
            .encode_memory_cell_record_retirement_reply(
                self.session.frame.frame_mut(),
                nonce,
                receipt.record(),
            )
            .ok()?;
        Some(ResumableAgentCpu(self.session))
    }
}
