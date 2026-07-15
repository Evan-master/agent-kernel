//! Fixed x86_64 CPU runtime for one admitted Agent task.
//!
//! Privilege runtime owns RSP0, storage owns evidence mailboxes, assembly owns
//! raw register mechanics, and runtime exposes validated type-state transitions.

mod assembly;
mod runtime;
mod storage;
mod validation;

pub(super) use runtime::{PreemptedAgentCpu, PreparedAgentCpu, YieldedAgentCpu};
