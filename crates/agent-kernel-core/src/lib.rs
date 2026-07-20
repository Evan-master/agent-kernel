#![cfg_attr(not(test), no_std)]
//! Deterministic Agent Kernel core primitives.
//!
//! This crate owns the no_std resource, capability, operation, checkpoint, and
//! event model. It performs no host I/O and keeps state in fixed-capacity
//! stores so the kernel facade can replay and inspect behavior deterministically.

mod action;
mod action_store;
mod agent;
mod agent_admission;
mod agent_entry;
mod agent_entry_retirement;
mod agent_event;
mod agent_execution;
mod agent_execution_store;
mod agent_image;
mod agent_image_event;
mod agent_image_record_retirement;
mod agent_image_store;
mod agent_launch;
mod agent_lifecycle;
mod agent_management;
mod agent_record_reference;
mod agent_record_retirement;
mod agent_store;
mod authorization;
mod capability;
mod capability_cleanup_revocation;
mod capability_compaction;
mod capability_derivation;
mod capability_revocation;
mod capability_store;
mod checkpoint;
mod checkpoint_store;
mod core;
mod device_event;
mod driver;
mod driver_command;
mod driver_command_event;
mod driver_command_runtime;
mod driver_command_submit;
mod driver_endpoint;
mod driver_endpoint_event;
mod driver_endpoint_store;
mod driver_event;
mod driver_invocation;
mod driver_invocation_event;
mod driver_invocation_runtime;
mod driver_invocation_tick;
mod driver_runtime_event;
mod error;
mod event;
mod event_archive;
mod event_archive_checkpoint;
mod event_archive_digest;
mod event_log;
mod fault;
mod fault_compaction;
mod fault_handler;
mod fault_handler_event;
mod fault_handler_store;
mod fault_policy;
mod fault_policy_event;
mod fault_policy_store;
mod fault_store;
mod id;
mod intent;
mod intent_compaction;
mod intent_event;
mod intent_store;
mod lookup;
mod mailbox_event;
mod mailbox_store;
mod mailbox_wait_event;
mod mailbox_wait_store;
mod memory;
mod memory_cell_record_reference;
mod memory_cell_record_retirement;
mod memory_store;
mod message;
mod message_retirement;
mod namespace;
mod namespace_entry_retirement;
mod namespace_lookup;
mod namespace_store;
mod observation;
mod observation_store;
mod operation;
mod orphaned_message_retirement;
mod resource;
mod resource_cleanup_authorization;
mod resource_ownership;
mod resource_record_reference;
#[cfg(test)]
mod resource_record_reference_tests;
mod resource_record_retirement;
mod resource_store;
mod run_queue;
mod runtime_admission;
mod runtime_admission_compaction;
mod runtime_admission_event;
mod runtime_admission_release;
mod runtime_admission_request;
mod runtime_admission_transition;
mod scheduler;
mod scheduler_dispatch;
mod scheduler_event;
mod scheduler_tick;
mod signal;
mod signal_event;
mod signal_store;
mod task;
mod task_compaction;
mod task_completion;
mod task_event;
mod task_lookup;
mod task_result_store;
mod task_store;
mod waiter_compaction;

pub use action::{ActionRecord, ActionStatus};
pub use agent::{AgentRecord, AgentRecordRetirement, AgentStatus};
pub use agent_entry::{AgentEntryKind, AgentEntryRecord, AgentEntryRetirement};
pub use agent_execution::{AgentExecutionContext, AgentExecutionState};
pub use agent_image::{
    AgentImageDigest, AgentImageKind, AgentImageRecord, AgentImageRecordRetirement,
    AgentImageStatus,
};
pub use capability::{Capability, CapabilityCompaction};
pub use checkpoint::{CheckpointRecord, CheckpointStatus};
pub use core::KernelCore;
pub use device_event::{DeviceEventKind, DeviceEventPayload, DeviceEventRecord, DeviceEventStatus};
pub use driver::DriverBindingRecord;
pub use driver_command::{
    DriverCommandKind, DriverCommandPayload, DriverCommandRecord, DriverCommandRequest,
    DriverCommandResult, DriverCommandStatus,
};
pub use driver_endpoint::{DriverEndpointDescriptor, DriverEndpointKind, DriverEndpointRecord};
pub use driver_invocation::{DriverInvocationRecord, DriverInvocationStatus};
pub use error::KernelError;
pub use event::{Event, EventKind};
pub use event_archive::{EventArchiveCheckpoint, EventArchiveDigest, EventArchiveProposal};
pub use fault::{FaultCompaction, FaultKind, FaultRecord};
pub use fault_handler::FaultHandlerRecord;
pub use fault_policy::{FaultPolicyAction, FaultPolicyOutcome, FaultPolicyRecord};
pub use id::{
    ActionId, AgentId, AgentImageId, CapabilityId, CheckpointId, DeviceEventId, DriverBindingId,
    DriverCommandId, DriverInvocationId, FaultHandlerId, FaultId, FaultPolicyId, IntentId,
    MemoryCellId, MessageId, NamespaceEntryId, ObservationId, ResourceId, RuntimeAdmissionId,
    TaskId, WaiterId,
};
pub use intent::{Intent, IntentCompaction, IntentKind, IntentStatus, VerificationRequirement};
pub use memory::{MemoryCellRecord, MemoryCellRecordRetirement, MemoryValue};
pub use message::{
    MessageKind, MessagePayload, MessageReceiveOutcome, MessageRecord, MessageRetirement,
    MessageStatus, OrphanedMessageRetirement,
};
pub use namespace::{
    NamespaceEntryRecord, NamespaceEntryRetirement, NamespaceKey, NamespaceObject,
};
pub use observation::ObservationRecord;
pub use operation::{Operation, OperationSet};
pub use resource::{
    Resource, ResourceCreateOutcome, ResourceKind, ResourceRecordRetirement, ResourceStatus,
};
pub use run_queue::{RunQueueEntry, TaskDispatchPermit};
pub use runtime_admission::{
    RuntimeAdmissionCompaction, RuntimeAdmissionFailure, RuntimeAdmissionPermit,
    RuntimeAdmissionRecord, RuntimeAdmissionReleaseBatch, RuntimeAdmissionStatus,
};
pub use signal::{SignalKey, SignalOutcome, WaiterCompaction, WaiterKind, WaiterRecord};
pub use task::{Task, TaskCompaction, TaskResult, TaskStatus};
