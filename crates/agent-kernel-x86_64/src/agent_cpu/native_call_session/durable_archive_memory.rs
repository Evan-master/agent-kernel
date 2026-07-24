//! Private call-data access for the kernel-mediated State Signer.
//!
//! This CPU-session child authenticates Call 56 before snapshotting or
//! replacing the fixed durable request. The stopped Agent owns no concurrent
//! mutation path while these methods execute.

use agent_kernel_x86_64::{
    agent_call::AgentCallRequest, durable_archive_request::DURABLE_ARCHIVE_REQUEST_BYTES,
};

use super::PendingAgentCallCpu;

impl PendingAgentCallCpu {
    pub(crate) fn authenticated_signable_durable_archive_request(
        &self,
    ) -> Option<[u8; DURABLE_ARCHIVE_REQUEST_BYTES]> {
        let generation = match self.authenticated_request()? {
            AgentCallRequest::SignDurableArchive { generation, .. } => generation,
            _ => return None,
        };
        self.session
            .memory
            .snapshot_durable_archive_request(generation)
    }

    pub(crate) fn replace_signable_durable_archive_request(
        &mut self,
        expected: &[u8; DURABLE_ARCHIVE_REQUEST_BYTES],
        replacement: &[u8; DURABLE_ARCHIVE_REQUEST_BYTES],
    ) -> bool {
        matches!(
            self.authenticated_request(),
            Some(AgentCallRequest::SignDurableArchive { .. })
        ) && self
            .session
            .memory
            .replace_durable_archive_request_if_unchanged(expected, replacement)
    }
}
