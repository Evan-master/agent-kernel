#![cfg_attr(not(test), no_std)]
//! Deterministic Agent Kernel core primitives.
//!
//! This crate owns the no_std resource, capability, operation, checkpoint, and
//! event model. It performs no host I/O and keeps state in fixed-capacity
//! stores so the kernel facade can replay and inspect behavior deterministically.

mod capability;
mod core;
mod error;
mod event;
mod id;
mod operation;
mod resource;

pub use capability::Capability;
pub use core::KernelCore;
pub use error::KernelError;
pub use event::{Event, EventKind};
pub use id::{AgentId, CapabilityId, CheckpointId, ResourceId};
pub use operation::{Operation, OperationSet};
pub use resource::{Resource, ResourceKind};
