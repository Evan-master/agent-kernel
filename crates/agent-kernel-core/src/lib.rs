#![cfg_attr(not(test), no_std)]
//! Deterministic Agent Kernel core primitives.
//!
//! This crate owns the no_std resource, capability, operation, checkpoint, and
//! event model. It performs no host I/O and keeps state in fixed-capacity
//! stores so the kernel facade can replay and inspect behavior deterministically.

mod action;
mod action_store;
mod agent;
mod agent_store;
mod authorization;
mod capability;
mod capability_store;
mod checkpoint;
mod checkpoint_store;
mod core;
mod error;
mod event;
mod event_log;
mod id;
mod intent;
mod intent_event;
mod intent_store;
mod lookup;
mod observation;
mod observation_store;
mod operation;
mod resource;
mod resource_store;
mod run_queue;
mod scheduler;
mod task;
mod task_event;
mod task_lookup;
mod task_store;

pub use action::{ActionRecord, ActionStatus};
pub use agent::{AgentRecord, AgentStatus};
pub use capability::Capability;
pub use checkpoint::{CheckpointRecord, CheckpointStatus};
pub use core::KernelCore;
pub use error::KernelError;
pub use event::{Event, EventKind};
pub use id::{
    ActionId, AgentId, CapabilityId, CheckpointId, IntentId, ObservationId, ResourceId, TaskId,
};
pub use intent::{Intent, IntentKind, IntentStatus, VerificationRequirement};
pub use observation::ObservationRecord;
pub use operation::{Operation, OperationSet};
pub use resource::{Resource, ResourceKind};
pub use run_queue::RunQueueEntry;
pub use task::{Task, TaskStatus};
