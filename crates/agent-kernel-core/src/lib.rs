#![cfg_attr(not(test), no_std)]
//! Deterministic Agent Kernel core primitives.
//!
//! This crate owns the no_std resource, capability, operation, checkpoint, and
//! event model. It performs no host I/O and keeps state in fixed-capacity
//! stores so the kernel facade can replay and inspect behavior deterministically.

mod authorization;
mod capability;
mod capability_store;
mod core;
mod error;
mod event;
mod event_log;
mod id;
mod lookup;
mod operation;
mod resource;
mod resource_store;
mod run_queue;
mod scheduler;
mod task;
mod task_store;

pub use capability::Capability;
pub use core::KernelCore;
pub use error::KernelError;
pub use event::{Event, EventKind};
pub use id::{ActionId, AgentId, CapabilityId, CheckpointId, ResourceId, TaskId};
pub use operation::{Operation, OperationSet};
pub use resource::{Resource, ResourceKind};
pub use run_queue::RunQueueEntry;
pub use task::{Task, TaskStatus};
