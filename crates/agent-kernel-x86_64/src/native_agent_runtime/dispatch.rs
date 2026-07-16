//! Kernel-selected semantic commit and physical context transfer.
//!
//! This bare-metal runtime child holds one mutable registry borrow across
//! readiness, commit, and guarded take. It returns the actual parked variant
//! selected by the core and accepts no caller-supplied Agent or state kind.

use agent_kernel_core::RunQueueEntry;

use super::{NativeAgentContext, NativeAgentRuntime};
use crate::X86BootedKernel;

pub(crate) struct DispatchedNativeAgent {
    entry: RunQueueEntry,
    context: NativeAgentContext,
}

impl NativeAgentRuntime {
    pub(crate) fn dispatch_next(
        &mut self,
        booted: &mut X86BootedKernel,
        quantum: u64,
    ) -> Option<DispatchedNativeAgent> {
        let permit = booted
            .kernel()
            .sys_prepare_next_ready_dispatch_with_quantum(quantum)
            .ok()?;
        let entry = permit.entry();
        if !self
            .contexts
            .contains_matching(entry.agent, |parked| parked.matches_entry(entry))
        {
            return None;
        }

        let dispatched = booted.kernel_mut().sys_commit_ready_dispatch(permit).ok()?;
        if dispatched != entry {
            return None;
        }
        let context = self
            .contexts
            .take_matching(entry.agent, |parked| parked.matches_entry(entry))
            .ok()?;
        Some(DispatchedNativeAgent { entry, context })
    }
}

impl DispatchedNativeAgent {
    pub(crate) const fn entry(&self) -> RunQueueEntry {
        self.entry
    }

    pub(crate) fn into_context(self) -> NativeAgentContext {
        self.context
    }
}
