//! Typed rejection diagnostics for cross-CPU Agent runtime rebinding.
//!
//! This x86 execution-layer helper classifies CR3, privilege, and stack
//! invariants without mutating the prepared Agent context. AP startup uses the
//! markers only on failure; successful dispatch remains silent.

use super::AgentCpuRuntime;
use crate::{
    agent_memory::PreparedAgentMemory,
    privilege_runtime::{current_privilege_level, stack_canary_valid},
};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum AgentCpuRuntimeRejection {
    MemoryRoot,
    TransitionRoot,
    InactiveKernelAddressSpace,
    PrivilegeLevel,
    StackCanary,
}

impl AgentCpuRuntimeRejection {
    pub(crate) const fn marker(self) -> &'static str {
        match self {
            Self::MemoryRoot => "AGENT_KERNEL_AP_WORKER_MEMORY_ROOT_ERROR",
            Self::TransitionRoot => "AGENT_KERNEL_AP_WORKER_TRANSITION_ROOT_ERROR",
            Self::InactiveKernelAddressSpace => {
                "AGENT_KERNEL_AP_WORKER_INACTIVE_KERNEL_ADDRESS_SPACE_ERROR"
            }
            Self::PrivilegeLevel => "AGENT_KERNEL_AP_WORKER_PRIVILEGE_LEVEL_ERROR",
            Self::StackCanary => "AGENT_KERNEL_AP_WORKER_STACK_CANARY_ERROR",
        }
    }
}

impl AgentCpuRuntime {
    pub(super) fn rejection_for(
        self,
        memory: &PreparedAgentMemory,
    ) -> Option<AgentCpuRuntimeRejection> {
        if memory.roots().kernel_cr3() != self.kernel_cr3 {
            return Some(AgentCpuRuntimeRejection::MemoryRoot);
        }
        if self.transition.kernel_cr3() != self.kernel_cr3 {
            return Some(AgentCpuRuntimeRejection::TransitionRoot);
        }
        if !memory.kernel_address_space_active() {
            return Some(AgentCpuRuntimeRejection::InactiveKernelAddressSpace);
        }
        if current_privilege_level() != 0 {
            return Some(AgentCpuRuntimeRejection::PrivilegeLevel);
        }
        (!stack_canary_valid(self.kernel_stack)).then_some(AgentCpuRuntimeRejection::StackCanary)
    }
}
