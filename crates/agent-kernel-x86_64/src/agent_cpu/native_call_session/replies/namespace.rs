//! Namespace acknowledgement binding an authenticated request to one record.

use agent_kernel_core::{NamespaceEntryRecord, NamespaceEntryRetirement};
use agent_kernel_x86_64::agent_call::AgentCallRequest;

use super::{PendingAgentCallCpu, ResumableAgentCpu};

impl PendingAgentCallCpu {
    pub(crate) fn acknowledge_namespace_binding(
        mut self,
        record: NamespaceEntryRecord,
    ) -> Option<ResumableAgentCpu> {
        let nonce = self.authenticated_nonce_for(|request| {
            matches!(
                request,
                AgentCallRequest::BindNamespaceEntry {
                    agent,
                    authority,
                    namespace,
                    key,
                    object,
                    ..
                } if record.owner == agent
                    && record.capability == authority
                    && record.namespace == namespace
                    && record.key == key
                    && record.object == object
            )
        })?;
        self.session
            .context
            .encode_namespace_binding_reply(self.session.frame.frame_mut(), nonce, record)
            .ok()?;
        Some(ResumableAgentCpu(self.session))
    }

    pub(crate) fn acknowledge_namespace_resolution(
        mut self,
        record: NamespaceEntryRecord,
    ) -> Option<ResumableAgentCpu> {
        let nonce = self.authenticated_nonce_for(|request| {
            matches!(
                request,
                AgentCallRequest::ResolveNamespaceEntry { namespace, key, .. }
                    if record.namespace == namespace && record.key == key
            )
        })?;
        self.session
            .context
            .encode_namespace_resolution_reply(self.session.frame.frame_mut(), nonce, record)
            .ok()?;
        Some(ResumableAgentCpu(self.session))
    }

    pub(crate) fn acknowledge_namespace_rebinding(
        mut self,
        record: NamespaceEntryRecord,
    ) -> Option<ResumableAgentCpu> {
        let nonce = self.authenticated_nonce_for(|request| {
            matches!(
                request,
                AgentCallRequest::RebindNamespaceEntry { entry, object, .. }
                    if record.id == entry && record.object == object
            )
        })?;
        self.session
            .context
            .encode_namespace_rebinding_reply(self.session.frame.frame_mut(), nonce, record)
            .ok()?;
        Some(ResumableAgentCpu(self.session))
    }

    pub(crate) fn acknowledge_namespace_retirement(
        mut self,
        receipt: NamespaceEntryRetirement,
    ) -> Option<ResumableAgentCpu> {
        let nonce = self.authenticated_nonce_for(|request| {
            matches!(
                request,
                AgentCallRequest::RetireNamespaceEntry {
                    agent,
                    authority,
                    entry,
                    ..
                } if receipt.actor() == agent
                    && receipt.authority() == authority
                    && receipt.namespace_entry() == entry
            )
        })?;
        self.session
            .context
            .encode_namespace_retirement_reply(
                self.session.frame.frame_mut(),
                nonce,
                receipt.record(),
            )
            .ok()?;
        Some(ResumableAgentCpu(self.session))
    }
}
