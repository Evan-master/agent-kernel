//! Fixed x86_64 CPU runtime for one admitted Agent task.
//!
//! Storage owns the dedicated stack and evidence mailbox, assembly owns raw
//! register mechanics, and runtime exposes validated type-state transitions.

mod assembly;
mod runtime;
mod storage;

pub(super) use runtime::{PreemptedAgentCpu, PreparedAgentCpu, YieldedAgentCpu};
