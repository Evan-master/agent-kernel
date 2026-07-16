//! Parked x86 Agent CPU ownership selected by kernel dispatch results.
//!
//! This bare-metal adapter stores prepared, PIT-preempted, and mailbox-waiting
//! CPU tokens under the Agent ID in their trusted call context. Every take is
//! guarded by the kernel-returned Agent/Task identity and expected physical
//! state. Scheduler policy stays in the core; this module owns only non-running
//! physical context ownership.

use agent_kernel_core::RunQueueEntry;
use agent_kernel_x86_64::{agent_call::AgentCallContext, native_runtime::NativeAgentRuntimeStore};

use crate::agent_cpu::{PreemptedAgentCpu, PreparedAgentCpu, WaitingMessageReceiveCpu};

const NATIVE_AGENT_CAPACITY: usize = 3;

#[derive(Copy, Clone, PartialEq, Eq)]
enum NativeAgentContextKind {
    Prepared,
    Preempted,
    WaitingMailbox,
}

pub(crate) enum NativeAgentContext {
    Prepared(PreparedAgentCpu),
    Preempted(PreemptedAgentCpu),
    WaitingMailbox(WaitingMessageReceiveCpu),
}

impl NativeAgentContext {
    fn context(&self) -> AgentCallContext {
        match self {
            Self::Prepared(cpu) => cpu.context(),
            Self::Preempted(cpu) => cpu.context(),
            Self::WaitingMailbox(cpu) => cpu.context(),
        }
    }

    const fn kind(&self) -> NativeAgentContextKind {
        match self {
            Self::Prepared(_) => NativeAgentContextKind::Prepared,
            Self::Preempted(_) => NativeAgentContextKind::Preempted,
            Self::WaitingMailbox(_) => NativeAgentContextKind::WaitingMailbox,
        }
    }
}

pub(crate) struct NativeAgentRuntime {
    contexts: NativeAgentRuntimeStore<NativeAgentContext, NATIVE_AGENT_CAPACITY>,
}

impl NativeAgentRuntime {
    pub(crate) fn new() -> Self {
        Self {
            contexts: NativeAgentRuntimeStore::new(),
        }
    }

    pub(crate) fn register_prepared(
        &mut self,
        cpu: PreparedAgentCpu,
    ) -> Option<NativeAgentContext> {
        self.park(NativeAgentContext::Prepared(cpu))
    }

    pub(crate) fn park_preempted(&mut self, cpu: PreemptedAgentCpu) -> Option<NativeAgentContext> {
        self.park(NativeAgentContext::Preempted(cpu))
    }

    pub(crate) fn park_waiting_mailbox(
        &mut self,
        cpu: WaitingMessageReceiveCpu,
    ) -> Option<NativeAgentContext> {
        self.park(NativeAgentContext::WaitingMailbox(cpu))
    }

    pub(crate) fn take_prepared(&mut self, dispatched: RunQueueEntry) -> Option<PreparedAgentCpu> {
        match self.take(dispatched, NativeAgentContextKind::Prepared)? {
            NativeAgentContext::Prepared(cpu) => Some(cpu),
            _ => None,
        }
    }

    pub(crate) fn take_preempted(
        &mut self,
        dispatched: RunQueueEntry,
    ) -> Option<PreemptedAgentCpu> {
        match self.take(dispatched, NativeAgentContextKind::Preempted)? {
            NativeAgentContext::Preempted(cpu) => Some(cpu),
            _ => None,
        }
    }

    pub(crate) fn take_waiting_mailbox(
        &mut self,
        dispatched: RunQueueEntry,
    ) -> Option<WaitingMessageReceiveCpu> {
        match self.take(dispatched, NativeAgentContextKind::WaitingMailbox)? {
            NativeAgentContext::WaitingMailbox(cpu) => Some(cpu),
            _ => None,
        }
    }

    pub(crate) const fn len(&self) -> usize {
        self.contexts.len()
    }

    pub(crate) const fn is_empty(&self) -> bool {
        self.contexts.is_empty()
    }

    fn park(&mut self, context: NativeAgentContext) -> Option<NativeAgentContext> {
        let agent = context.context().agent();
        match self.contexts.insert(agent, context) {
            Ok(()) => None,
            Err((_error, rejected)) => Some(rejected),
        }
    }

    fn take(
        &mut self,
        dispatched: RunQueueEntry,
        expected: NativeAgentContextKind,
    ) -> Option<NativeAgentContext> {
        self.contexts
            .take_matching(dispatched.agent, |parked| {
                let context = parked.context();
                context.agent() == dispatched.agent
                    && context.task() == dispatched.task
                    && parked.kind() == expected
            })
            .ok()
    }
}
