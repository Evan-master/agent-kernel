//! Fixed x86_64 CPU runtime for admitted Agent task contexts.
//!
//! Privilege runtime owns the one active RSP0 boundary, storage owns evidence
//! mailboxes, assembly owns raw register mechanics, and runtime exposes owned,
//! validated type-state transitions for multiple suspended contexts.

mod address_space_reclamation;
mod assembly;
mod call;
mod fault;
mod native_call_session;
mod runtime;
mod storage;
mod validation;

pub(super) use fault::FaultedAgentCpu;
pub(super) use native_call_session::{
    AgentRunOutcome, CompletedAgentCpu, PendingAgentCallCpu, ResumableAgentCpu, WaitingAgentCallCpu,
};
pub(super) use runtime::{AgentCpuRuntime, PreemptedAgentCpu, PreparedAgentCpu};
