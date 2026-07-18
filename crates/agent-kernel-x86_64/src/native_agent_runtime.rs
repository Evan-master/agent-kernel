//! Parked x86 Agent CPU ownership selected by kernel dispatch results.
//!
//! This bare-metal adapter stores prepared, PIT-preempted, mailbox-waiting,
//! cooperatively yielded, and repaired-fault contexts under trusted Agent
//! identity. Scheduler policy remains in the core; the registry owns only
//! non-running physical state.

mod dispatch;

use agent_kernel_core::{AgentId, RunQueueEntry};
use agent_kernel_x86_64::{agent_call::AgentCallContext, native_runtime::NativeAgentRuntimeStore};

use crate::agent_cpu::{
    PreemptedAgentCpu, PreparedAgentCpu, ResumableAgentCpu, WaitingAgentCallCpu,
};

const NATIVE_AGENT_CAPACITY: usize = 6;

pub(crate) enum NativeAgentContext {
    Prepared(PreparedAgentCpu),
    Preempted(PreemptedAgentCpu),
    WaitingCall(WaitingAgentCallCpu),
    YieldedCall(ResumableAgentCpu),
    RecoveredFault(ResumableAgentCpu),
}

impl NativeAgentContext {
    fn context(&self) -> AgentCallContext {
        match self {
            Self::Prepared(cpu) => cpu.context(),
            Self::Preempted(cpu) => cpu.context(),
            Self::WaitingCall(cpu) => cpu.context(),
            Self::YieldedCall(cpu) => cpu.context(),
            Self::RecoveredFault(cpu) => cpu.context(),
        }
    }

    fn matches_entry(&self, entry: RunQueueEntry) -> bool {
        let context = self.context();
        context.agent() == entry.agent && context.task() == entry.task
    }

    pub(crate) fn into_prepared(self) -> Option<PreparedAgentCpu> {
        match self {
            Self::Prepared(cpu) => Some(cpu),
            _ => None,
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

    pub(crate) fn park_waiting_call(
        &mut self,
        cpu: WaitingAgentCallCpu,
    ) -> Option<NativeAgentContext> {
        self.park(NativeAgentContext::WaitingCall(cpu))
    }

    pub(crate) fn park_yielded_call(
        &mut self,
        cpu: ResumableAgentCpu,
    ) -> Option<NativeAgentContext> {
        self.park(NativeAgentContext::YieldedCall(cpu))
    }

    pub(crate) fn park_recovered_fault(
        &mut self,
        cpu: ResumableAgentCpu,
    ) -> Option<NativeAgentContext> {
        self.park(NativeAgentContext::RecoveredFault(cpu))
    }

    pub(crate) const fn len(&self) -> usize {
        self.contexts.len()
    }

    pub(crate) const fn is_empty(&self) -> bool {
        self.contexts.is_empty()
    }

    pub(crate) fn contains(&self, agent: AgentId) -> bool {
        self.contexts.get(agent).is_ok()
    }

    fn park(&mut self, context: NativeAgentContext) -> Option<NativeAgentContext> {
        let agent = context.context().agent();
        match self.contexts.insert(agent, context) {
            Ok(()) => None,
            Err((_error, rejected)) => Some(rejected),
        }
    }
}
