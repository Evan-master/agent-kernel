//! Canonical reply encoding for Resource record retirement.

use agent_kernel_core::{Resource, ResourceKind, ResourceStatus};

use super::AgentCallContext;
use crate::{
    agent_call::{
        AgentCallDecodeError, AGENT_CALL_RESOURCE_DEVICE, AGENT_CALL_RESOURCE_FILE,
        AGENT_CALL_RESOURCE_MEMORY, AGENT_CALL_RESOURCE_NETWORK, AGENT_CALL_RESOURCE_PROCESS,
        AGENT_CALL_RESOURCE_SERVICE, AGENT_CALL_RESOURCE_WORKSPACE,
        AGENT_CALL_RETIRE_RESOURCE_RECORD,
    },
    context::PrivilegeInterruptStackFrame,
};

impl AgentCallContext {
    pub fn encode_resource_record_retirement_reply(
        self,
        frame: &mut PrivilegeInterruptStackFrame,
        nonce: u64,
        record: Resource,
    ) -> Result<(), AgentCallDecodeError> {
        if record.id.raw() == 0 || record.status != ResourceStatus::Retired {
            return Err(AgentCallDecodeError::InvalidPayload);
        }
        self.encode_reply(frame, nonce, AGENT_CALL_RETIRE_RESOURCE_RECORD)?;
        frame.r10 = record.id.raw();
        frame.r11 = resource_kind_code(record.kind);
        frame.r12 = record.parent.map_or(0, |parent| parent.raw());
        frame.r13 = record.owner.map_or(0, |owner| owner.raw());
        Ok(())
    }
}

const fn resource_kind_code(kind: ResourceKind) -> u64 {
    match kind {
        ResourceKind::Workspace => AGENT_CALL_RESOURCE_WORKSPACE,
        ResourceKind::Memory => AGENT_CALL_RESOURCE_MEMORY,
        ResourceKind::Service => AGENT_CALL_RESOURCE_SERVICE,
        ResourceKind::Network => AGENT_CALL_RESOURCE_NETWORK,
        ResourceKind::Device => AGENT_CALL_RESOURCE_DEVICE,
        ResourceKind::File => AGENT_CALL_RESOURCE_FILE,
        ResourceKind::Process => AGENT_CALL_RESOURCE_PROCESS,
    }
}
