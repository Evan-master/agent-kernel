//! Prepared x86 Agent CPU ownership selected by kernel dispatch results.
//!
//! This bare-metal adapter registers verified `PreparedAgentCpu` values by the
//! Agent ID in their trusted call context. It consumes a value only when the
//! kernel returns a matching Agent/Task dispatch. The generic fixed store owns
//! capacity and compaction; this layer owns the scheduler/CPU identity check.

use agent_kernel_core::RunQueueEntry;
use agent_kernel_x86_64::native_runtime::NativeAgentRuntimeStore;

use crate::agent_cpu::PreparedAgentCpu;

const NATIVE_AGENT_CAPACITY: usize = 3;

pub(crate) struct NativeAgentRuntime {
    prepared: NativeAgentRuntimeStore<PreparedAgentCpu, NATIVE_AGENT_CAPACITY>,
}

impl NativeAgentRuntime {
    pub(crate) fn new() -> Self {
        Self {
            prepared: NativeAgentRuntimeStore::new(),
        }
    }

    pub(crate) fn register(&mut self, cpu: PreparedAgentCpu) -> Option<PreparedAgentCpu> {
        match self.prepared.insert(cpu.context().agent(), cpu) {
            Ok(()) => None,
            Err((_error, rejected)) => Some(rejected),
        }
    }

    pub(crate) fn take_dispatched(
        &mut self,
        dispatched: RunQueueEntry,
    ) -> Option<PreparedAgentCpu> {
        let context = self.prepared.get(dispatched.agent).ok()?.context();
        if context.agent() != dispatched.agent || context.task() != dispatched.task {
            return None;
        }
        self.prepared.take(dispatched.agent).ok()
    }

    pub(crate) const fn len(&self) -> usize {
        self.prepared.len()
    }

    pub(crate) const fn is_empty(&self) -> bool {
        self.prepared.is_empty()
    }
}
