//! Parked x86 Agent CPU ownership selected by kernel dispatch results.
//!
//! This bare-metal adapter stores prepared, PIT-preempted, and mailbox-waiting
//! CPU tokens under the Agent ID in their trusted call context. Read-only
//! readiness checks match the core permit before semantic commit, and every
//! take is guarded again by the committed Agent/Task identity and expected
//! physical state. Scheduler policy stays in the core; this module owns only
//! non-running physical context ownership.

use agent_kernel_core::{RunQueueEntry, TaskDispatchPermit};
use agent_kernel_x86_64::{agent_call::AgentCallContext, native_runtime::NativeAgentRuntimeStore};

use crate::{
    agent_cpu::{PreemptedAgentCpu, PreparedAgentCpu, WaitingMessageReceiveCpu},
    X86BootedKernel,
};

const NATIVE_AGENT_CAPACITY: usize = 3;

#[derive(Copy, Clone, PartialEq, Eq)]
pub(crate) enum NativeAgentContextKind {
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

    fn matches(&self, entry: RunQueueEntry, expected: NativeAgentContextKind) -> bool {
        let context = self.context();
        context.agent() == entry.agent && context.task() == entry.task && self.kind() == expected
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

    pub(crate) fn commit_ready_dispatch(
        &self,
        booted: &mut X86BootedKernel,
        quantum: u64,
        expected_entry: RunQueueEntry,
        expected_kind: NativeAgentContextKind,
    ) -> Option<RunQueueEntry> {
        let permit = booted
            .kernel()
            .sys_prepare_next_ready_dispatch_with_quantum(quantum)
            .ok()?;
        if permit.entry() != expected_entry || !self.ready_for(permit, expected_kind) {
            return None;
        }
        let dispatched = booted.kernel_mut().sys_commit_ready_dispatch(permit).ok()?;
        (dispatched == expected_entry).then_some(dispatched)
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
                parked.matches(dispatched, expected)
            })
            .ok()
    }

    fn ready_for(&self, permit: TaskDispatchPermit, expected: NativeAgentContextKind) -> bool {
        let entry = permit.entry();
        self.contexts
            .contains_matching(entry.agent, |parked| parked.matches(entry, expected))
    }
}
