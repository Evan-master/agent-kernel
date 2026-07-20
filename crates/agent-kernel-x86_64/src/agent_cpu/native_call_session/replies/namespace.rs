//! Namespace acknowledgement binding an authenticated request to one record.

use agent_kernel_core::{NamespaceEntryRecord, NamespaceEntryRetirement, NamespacePathResolution};
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

    pub(crate) fn acknowledge_namespace_path_resolution(
        mut self,
        resolution: NamespacePathResolution,
    ) -> Option<ResumableAgentCpu> {
        let record = resolution.terminal();
        let nonce = self.authenticated_nonce_for(|request| match request {
            AgentCallRequest::ResolveNamespacePath {
                root,
                first,
                second,
                ..
            } => {
                let terminal = second.unwrap_or(first);
                let depth = if second.is_some() { 2 } else { 1 };
                resolution.root() == root
                    && resolution.depth() == depth
                    && record.capability == terminal.authority()
                    && record.key == terminal.key()
            }
            _ => false,
        })?;
        self.session
            .context
            .encode_namespace_path_resolution_reply(self.session.frame.frame_mut(), nonce, record)
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

    pub(crate) fn acknowledge_namespace_compare_rebinding(
        mut self,
        record: NamespaceEntryRecord,
    ) -> Option<ResumableAgentCpu> {
        let nonce = self.authenticated_nonce_for(|request| {
            matches!(
                request,
                AgentCallRequest::CompareAndRebindNamespaceEntry {
                    entry,
                    expected_revision,
                    object,
                    ..
                } if record.id == entry
                    && record.object == object
                    && expected_revision.checked_add(1) == Some(record.revision)
            )
        })?;
        self.session
            .context
            .encode_namespace_compare_rebinding_reply(self.session.frame.frame_mut(), nonce, record)
            .ok()?;
        Some(ResumableAgentCpu(self.session))
    }

    pub(crate) fn acknowledge_namespace_compare_retirement(
        mut self,
        receipt: NamespaceEntryRetirement,
    ) -> Option<ResumableAgentCpu> {
        let nonce = self.authenticated_nonce_for(|request| {
            matches!(
                request,
                AgentCallRequest::CompareAndRetireNamespaceEntry {
                    agent,
                    authority,
                    entry,
                    expected_revision,
                    ..
                } if receipt.actor() == agent
                    && receipt.authority() == authority
                    && receipt.namespace_entry() == entry
                    && receipt.record().revision == expected_revision
            )
        })?;
        self.session
            .context
            .encode_namespace_compare_retirement_reply(
                self.session.frame.frame_mut(),
                nonce,
                receipt.record(),
            )
            .ok()?;
        Some(ResumableAgentCpu(self.session))
    }
}
