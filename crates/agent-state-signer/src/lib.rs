#![no_std]
//! User-space policy boundary for signing durable Agent Kernel archives.

mod agent;
mod policy;
mod provider;

pub use agent::{StateSignerAgent, StateSignerAgentError};
pub use policy::{StateSignerPolicy, StateSignerPolicyError};
pub use provider::StateSignerProvider;
