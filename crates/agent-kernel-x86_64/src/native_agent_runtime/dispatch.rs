//! Kernel-selected semantic commit and physical context transfer.
//!
//! This bare-metal runtime child holds one mutable registry borrow across
//! readiness, commit, and guarded take. It returns the actual parked variant
//! selected by the core and accepts no caller-supplied Agent or state kind.

use agent_kernel_core::{AgentId, DriverInvocationId, DriverInvocationStatus, RunQueueEntry};

use super::{NativeAgentContext, NativeAgentRuntime};
use crate::X86BootedKernel;

pub(crate) struct DispatchedNativeAgent {
    entry: RunQueueEntry,
    context: NativeAgentContext,
}

pub(crate) struct DispatchedNativeDriver {
    invocation: DriverInvocationId,
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

    pub(crate) fn dispatch_next_driver(
        &mut self,
        booted: &mut X86BootedKernel,
        driver: AgentId,
        quantum: u64,
    ) -> Option<DispatchedNativeDriver> {
        let queued = booted
            .kernel()
            .driver_invocations()
            .iter()
            .find(|record| {
                record.driver == driver && record.status == DriverInvocationStatus::Queued
            })?
            .id;
        if !self
            .contexts
            .contains_matching(driver, |parked| parked.matches_driver(driver, queued))
        {
            return None;
        }

        let invocation = booted
            .kernel_mut()
            .sys_dispatch_next_driver_invocation(driver, quantum)
            .ok()?;
        if invocation != queued {
            return None;
        }
        let context = self
            .contexts
            .take_matching(driver, |parked| parked.matches_driver(driver, invocation))
            .ok()?;
        Some(DispatchedNativeDriver {
            invocation,
            context,
        })
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

impl DispatchedNativeDriver {
    pub(crate) const fn invocation(&self) -> DriverInvocationId {
        self.invocation
    }

    pub(crate) fn into_context(self) -> NativeAgentContext {
        self.context
    }
}
