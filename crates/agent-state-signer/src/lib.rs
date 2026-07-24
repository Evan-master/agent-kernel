#![no_std]
//! User-space policy boundary for signing durable Agent Kernel archives.

mod agent;
mod native_provider;
mod policy;
mod provider;

pub use agent::{StateSignerAgent, StateSignerAgentError};
pub use native_provider::{
    NativeStateSignerProvider, NativeStateSignerProviderEntry, NativeStateSignerProviderError,
    NATIVE_STATE_SIGNER_PROVIDER_STATUS_OK,
};
pub use policy::{StateSignerPolicy, StateSignerPolicyError};
pub use provider::StateSignerProvider;
