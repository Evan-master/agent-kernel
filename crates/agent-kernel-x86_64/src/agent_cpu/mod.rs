//! Fixed x86_64 CPU runtime for admitted Agent task contexts.
//!
//! Privilege runtime owns the one active RSP0 boundary, storage owns evidence
//! mailboxes, assembly owns raw register mechanics, and runtime exposes owned,
//! validated type-state transitions for multiple suspended contexts.

mod assembly;
mod call;
mod call_flow;
mod runtime;
mod storage;
mod validation;

pub(super) use call_flow::{AcknowledgedTaskResultCpu, CompletedAgentCpu, RequestedTaskResultCpu};
pub(super) use runtime::{AgentCpuRuntime, PreemptedAgentCpu};
