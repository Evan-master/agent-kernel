//! Canonical complete-record replies for native Namespace Agent Calls.

use agent_kernel_core::NamespaceEntryRecord;

use super::AgentCallContext;
use crate::{
    agent_call::{
        encode_namespace_object, AgentCallDecodeError, AGENT_CALL_BIND_NAMESPACE_ENTRY,
        AGENT_CALL_COMPARE_AND_REBIND_NAMESPACE_ENTRY,
        AGENT_CALL_COMPARE_AND_RETIRE_NAMESPACE_ENTRY, AGENT_CALL_REBIND_NAMESPACE_ENTRY,
        AGENT_CALL_RESOLVE_NAMESPACE_ENTRY, AGENT_CALL_RESOLVE_NAMESPACE_PATH,
        AGENT_CALL_RESOLVE_NAMESPACE_PATH_FROM_MEMORY, AGENT_CALL_RETIRE_NAMESPACE_ENTRY,
    },
    context::PrivilegeInterruptStackFrame,
};

impl AgentCallContext {
    pub fn encode_namespace_binding_reply(
        self,
        frame: &mut PrivilegeInterruptStackFrame,
        nonce: u64,
        record: NamespaceEntryRecord,
    ) -> Result<(), AgentCallDecodeError> {
        self.encode_namespace_entry_reply(frame, nonce, AGENT_CALL_BIND_NAMESPACE_ENTRY, record)
    }

    pub fn encode_namespace_resolution_reply(
        self,
        frame: &mut PrivilegeInterruptStackFrame,
        nonce: u64,
        record: NamespaceEntryRecord,
    ) -> Result<(), AgentCallDecodeError> {
        self.encode_namespace_entry_reply(frame, nonce, AGENT_CALL_RESOLVE_NAMESPACE_ENTRY, record)
    }

    pub fn encode_namespace_rebinding_reply(
        self,
        frame: &mut PrivilegeInterruptStackFrame,
        nonce: u64,
        record: NamespaceEntryRecord,
    ) -> Result<(), AgentCallDecodeError> {
        self.encode_namespace_entry_reply(frame, nonce, AGENT_CALL_REBIND_NAMESPACE_ENTRY, record)
    }

    pub fn encode_namespace_retirement_reply(
        self,
        frame: &mut PrivilegeInterruptStackFrame,
        nonce: u64,
        record: NamespaceEntryRecord,
    ) -> Result<(), AgentCallDecodeError> {
        self.encode_namespace_entry_reply(frame, nonce, AGENT_CALL_RETIRE_NAMESPACE_ENTRY, record)
    }

    pub fn encode_namespace_compare_rebinding_reply(
        self,
        frame: &mut PrivilegeInterruptStackFrame,
        nonce: u64,
        record: NamespaceEntryRecord,
    ) -> Result<(), AgentCallDecodeError> {
        self.encode_namespace_entry_reply(
            frame,
            nonce,
            AGENT_CALL_COMPARE_AND_REBIND_NAMESPACE_ENTRY,
            record,
        )
    }

    pub fn encode_namespace_compare_retirement_reply(
        self,
        frame: &mut PrivilegeInterruptStackFrame,
        nonce: u64,
        record: NamespaceEntryRecord,
    ) -> Result<(), AgentCallDecodeError> {
        self.encode_namespace_entry_reply(
            frame,
            nonce,
            AGENT_CALL_COMPARE_AND_RETIRE_NAMESPACE_ENTRY,
            record,
        )
    }

    pub fn encode_namespace_path_resolution_reply(
        self,
        frame: &mut PrivilegeInterruptStackFrame,
        nonce: u64,
        record: NamespaceEntryRecord,
    ) -> Result<(), AgentCallDecodeError> {
        self.encode_namespace_entry_reply(frame, nonce, AGENT_CALL_RESOLVE_NAMESPACE_PATH, record)
    }

    pub fn encode_namespace_memory_path_resolution_reply(
        self,
        frame: &mut PrivilegeInterruptStackFrame,
        nonce: u64,
        record: NamespaceEntryRecord,
    ) -> Result<(), AgentCallDecodeError> {
        self.encode_namespace_entry_reply(
            frame,
            nonce,
            AGENT_CALL_RESOLVE_NAMESPACE_PATH_FROM_MEMORY,
            record,
        )
    }

    fn encode_namespace_entry_reply(
        self,
        frame: &mut PrivilegeInterruptStackFrame,
        nonce: u64,
        operation: u64,
        record: NamespaceEntryRecord,
    ) -> Result<(), AgentCallDecodeError> {
        let object =
            encode_namespace_object(record.object).ok_or(AgentCallDecodeError::InvalidPayload)?;
        if record.id.raw() == 0
            || record.owner.raw() == 0
            || record.namespace.raw() == 0
            || record.capability.raw() == 0
            || record.revision == 0
        {
            return Err(AgentCallDecodeError::InvalidPayload);
        }
        self.encode_reply(frame, nonce, operation)?;
        frame.r10 = record.id.raw();
        frame.r11 = record.owner.raw();
        frame.r12 = record.namespace.raw();
        frame.r13 = record.capability.raw();
        frame.r14 = record.key.raw();
        frame.r15 = object;
        frame.rbp = record.revision;
        Ok(())
    }
}
