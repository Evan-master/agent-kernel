//! Native x86_64 Agent Call ABI register contracts.
//!
//! This architecture-library module decodes bounded call requests from an
//! already-captured privilege frame and encodes read-only context replies. It
//! performs no privileged operation and trusts identity only from an explicit
//! scheduler-owned `AgentCallContext`.

mod agent_management;
mod capability;
mod capability_compaction;
mod context;
mod intent_compaction;
mod mailbox;
mod memory_page;
mod memory_region;
mod operation;
mod request;
mod resource;
mod runtime_admission;
mod task_compaction;
mod task_lifecycle;
mod transcript;

use agent_kernel_core::{AgentId, AgentImageId, TaskId, TaskResult};

use crate::context::PrivilegeInterruptStackFrame;

pub use context::AgentCallContext;
pub use operation::AgentCallOperation;
pub use request::AgentCallRequest;
pub use transcript::{AgentCallTranscript, AgentCallTranscriptError};

pub const AGENT_CALL_ABI_MAGIC: u64 = u64::from_le_bytes(*b"AGNTCALL");
pub const AGENT_CALL_ABI_VERSION: u64 = 1;
pub const AGENT_CALL_DESCRIBE_CONTEXT: u64 = 1;
pub const AGENT_CALL_YIELD: u64 = 2;
pub const AGENT_CALL_COMPLETE_TASK: u64 = 3;
pub const AGENT_CALL_SUBMIT_TASK_RESULT: u64 = 4;
pub const AGENT_CALL_INSPECT_TASK_RESULT: u64 = 5;
pub const AGENT_CALL_VERIFY_TASK: u64 = 6;
pub const AGENT_CALL_SEND_MESSAGE: u64 = 7;
pub const AGENT_CALL_RECEIVE_MESSAGE: u64 = 8;
pub const AGENT_CALL_ACKNOWLEDGE_MESSAGE: u64 = 9;
pub const AGENT_CALL_CREATE_RESOURCE: u64 = 10;
pub const AGENT_CALL_RETIRE_RESOURCE: u64 = 11;
pub const AGENT_CALL_DERIVE_CAPABILITY: u64 = 12;
pub const AGENT_CALL_REVOKE_DERIVED_CAPABILITY: u64 = 13;
pub const AGENT_CALL_DECLARE_INTENT: u64 = 14;
pub const AGENT_CALL_CREATE_TASK: u64 = 15;
pub const AGENT_CALL_DELEGATE_TASK: u64 = 16;
pub const AGENT_CALL_REGISTER_MANAGED_AGENT: u64 = 17;
pub const AGENT_CALL_SUSPEND_MANAGED_AGENT: u64 = 18;
pub const AGENT_CALL_RESUME_MANAGED_AGENT: u64 = 19;
pub const AGENT_CALL_RETIRE_MANAGED_AGENT: u64 = 20;
pub const AGENT_CALL_ALLOCATE_MEMORY_PAGE: u64 = 21;
pub const AGENT_CALL_INSPECT_MEMORY_PAGE: u64 = 22;
pub const AGENT_CALL_RELEASE_MEMORY_PAGE: u64 = 23;
pub const AGENT_CALL_MEMORY_PAGE_BYTES: u64 = 4096;
pub const AGENT_CALL_ALLOCATE_MEMORY_REGION: u64 = 24;
pub const AGENT_CALL_INSPECT_MEMORY_REGION: u64 = 25;
pub const AGENT_CALL_RELEASE_MEMORY_REGION: u64 = 26;
pub const AGENT_CALL_REQUEST_RUNTIME_ADMISSION: u64 = 27;
pub const AGENT_CALL_DISCOVER_RUNTIME_ADMISSION: u64 = 28;
pub const AGENT_CALL_COMPACT_RUNTIME_ADMISSIONS: u64 = 29;
pub const AGENT_CALL_COMPACT_TASKS: u64 = 30;
pub const AGENT_CALL_COMPACT_INTENTS: u64 = 31;
pub const AGENT_CALL_COMPACT_CAPABILITY: u64 = 32;
pub const AGENT_CALL_MEMORY_REGION_PAGE_BYTES: u64 = 4096;
pub const AGENT_CALL_MEMORY_REGION_MAX_PAGES: u64 = 4;
pub const AGENT_CALL_MESSAGE_NOTIFY: u64 = 1;
pub const AGENT_CALL_MESSAGE_REQUEST: u64 = 2;
pub const AGENT_CALL_MESSAGE_RESPONSE: u64 = 3;
pub const AGENT_CALL_MESSAGE_FAULT: u64 = 4;
pub const AGENT_CALL_RESOURCE_WORKSPACE: u64 = 1;
pub const AGENT_CALL_RESOURCE_MEMORY: u64 = 2;
pub const AGENT_CALL_RESOURCE_SERVICE: u64 = 3;
pub const AGENT_CALL_RESOURCE_NETWORK: u64 = 4;
pub const AGENT_CALL_RESOURCE_DEVICE: u64 = 5;
pub const AGENT_CALL_INTENT_OBSERVE: u64 = 1;
pub const AGENT_CALL_INTENT_ACT: u64 = 2;
pub const AGENT_CALL_INTENT_VERIFY: u64 = 3;
pub const AGENT_CALL_INTENT_CHECKPOINT: u64 = 4;
pub const AGENT_CALL_INTENT_ROLLBACK: u64 = 5;
pub const AGENT_CALL_VERIFICATION_OPTIONAL: u64 = 1;
pub const AGENT_CALL_VERIFICATION_REQUIRED: u64 = 2;
pub const AGENT_CALL_AGENT_ACTIVE: u64 = 1;
pub const AGENT_CALL_AGENT_SUSPENDED: u64 = 2;
pub const AGENT_CALL_AGENT_RETIRED: u64 = 3;
pub const AGENT_CALL_STATUS_OK: u64 = 0;

impl AgentCallRequest {
    pub fn decode(frame: &PrivilegeInterruptStackFrame) -> Result<Self, AgentCallDecodeError> {
        if frame.rax != AGENT_CALL_ABI_MAGIC {
            return Err(AgentCallDecodeError::InvalidMagic);
        }
        if frame.rbx != AGENT_CALL_ABI_VERSION {
            return Err(AgentCallDecodeError::UnsupportedVersion);
        }
        let operation = match frame.rcx {
            AGENT_CALL_DESCRIBE_CONTEXT => AgentCallOperation::DescribeContext,
            AGENT_CALL_YIELD => AgentCallOperation::Yield,
            AGENT_CALL_COMPLETE_TASK => AgentCallOperation::CompleteTask,
            AGENT_CALL_SUBMIT_TASK_RESULT => AgentCallOperation::SubmitTaskResult,
            AGENT_CALL_INSPECT_TASK_RESULT => AgentCallOperation::InspectTaskResult,
            AGENT_CALL_VERIFY_TASK => AgentCallOperation::VerifyTask,
            AGENT_CALL_SEND_MESSAGE => AgentCallOperation::SendMessage,
            AGENT_CALL_RECEIVE_MESSAGE => AgentCallOperation::ReceiveMessage,
            AGENT_CALL_ACKNOWLEDGE_MESSAGE => AgentCallOperation::AcknowledgeMessage,
            AGENT_CALL_CREATE_RESOURCE => AgentCallOperation::CreateResource,
            AGENT_CALL_RETIRE_RESOURCE => AgentCallOperation::RetireResource,
            AGENT_CALL_DERIVE_CAPABILITY => AgentCallOperation::DeriveCapability,
            AGENT_CALL_REVOKE_DERIVED_CAPABILITY => AgentCallOperation::RevokeDerivedCapability,
            AGENT_CALL_DECLARE_INTENT => AgentCallOperation::DeclareIntent,
            AGENT_CALL_CREATE_TASK => AgentCallOperation::CreateTask,
            AGENT_CALL_DELEGATE_TASK => AgentCallOperation::DelegateTask,
            AGENT_CALL_REGISTER_MANAGED_AGENT => AgentCallOperation::RegisterManagedAgent,
            AGENT_CALL_SUSPEND_MANAGED_AGENT => AgentCallOperation::SuspendManagedAgent,
            AGENT_CALL_RESUME_MANAGED_AGENT => AgentCallOperation::ResumeManagedAgent,
            AGENT_CALL_RETIRE_MANAGED_AGENT => AgentCallOperation::RetireManagedAgent,
            AGENT_CALL_ALLOCATE_MEMORY_PAGE => AgentCallOperation::AllocateMemoryPage,
            AGENT_CALL_INSPECT_MEMORY_PAGE => AgentCallOperation::InspectMemoryPage,
            AGENT_CALL_RELEASE_MEMORY_PAGE => AgentCallOperation::ReleaseMemoryPage,
            AGENT_CALL_ALLOCATE_MEMORY_REGION => AgentCallOperation::AllocateMemoryRegion,
            AGENT_CALL_INSPECT_MEMORY_REGION => AgentCallOperation::InspectMemoryRegion,
            AGENT_CALL_RELEASE_MEMORY_REGION => AgentCallOperation::ReleaseMemoryRegion,
            AGENT_CALL_REQUEST_RUNTIME_ADMISSION => AgentCallOperation::RequestRuntimeAdmission,
            AGENT_CALL_DISCOVER_RUNTIME_ADMISSION => AgentCallOperation::DiscoverRuntimeAdmission,
            AGENT_CALL_COMPACT_RUNTIME_ADMISSIONS => AgentCallOperation::CompactRuntimeAdmissions,
            AGENT_CALL_COMPACT_TASKS => AgentCallOperation::CompactTasks,
            AGENT_CALL_COMPACT_INTENTS => AgentCallOperation::CompactIntents,
            AGENT_CALL_COMPACT_CAPABILITY => AgentCallOperation::CompactCapability,
            _ => return Err(AgentCallDecodeError::UnsupportedOperation),
        };
        if frame.rdx != 0 {
            return Err(AgentCallDecodeError::UnsupportedFlags);
        }
        match operation {
            AgentCallOperation::DescribeContext => {
                ensure_reserved_zero(frame)?;
                if frame.rsi == 0 || frame.rdi != 0 || frame.r8 != 0 || frame.r9 != 0 {
                    return Err(AgentCallDecodeError::InvalidPayload);
                }
                Ok(Self::DescribeContext { nonce: frame.rsi })
            }
            AgentCallOperation::Yield => {
                ensure_reserved_zero(frame)?;
                let (agent, task, image, nonce) = decode_context_payload(frame)?;
                Ok(Self::Yield {
                    agent,
                    task,
                    image,
                    nonce,
                })
            }
            AgentCallOperation::CompleteTask => {
                ensure_reserved_zero(frame)?;
                let (agent, task, image, nonce) = decode_context_payload(frame)?;
                Ok(Self::CompleteTask {
                    agent,
                    task,
                    image,
                    nonce,
                })
            }
            AgentCallOperation::SubmitTaskResult => {
                ensure_extended_reserved_zero(frame)?;
                let (agent, task, image, nonce) = decode_context_payload(frame)?;
                let code =
                    u16::try_from(frame.r10).map_err(|_| AgentCallDecodeError::InvalidPayload)?;
                Ok(Self::SubmitTaskResult {
                    agent,
                    task,
                    image,
                    nonce,
                    result: TaskResult {
                        code,
                        value: frame.r11,
                    },
                })
            }
            AgentCallOperation::InspectTaskResult => {
                let (agent, task, image, nonce, target_task) = decode_verifier_payload(frame)?;
                Ok(Self::InspectTaskResult {
                    agent,
                    task,
                    image,
                    nonce,
                    target_task,
                })
            }
            AgentCallOperation::VerifyTask => {
                let (agent, task, image, nonce, target_task) = decode_verifier_payload(frame)?;
                Ok(Self::VerifyTask {
                    agent,
                    task,
                    image,
                    nonce,
                    target_task,
                })
            }
            AgentCallOperation::SendMessage => mailbox::decode_send(frame),
            AgentCallOperation::ReceiveMessage => mailbox::decode_receive(frame),
            AgentCallOperation::AcknowledgeMessage => mailbox::decode_acknowledgement(frame),
            AgentCallOperation::CreateResource => resource::decode_create(frame),
            AgentCallOperation::RetireResource => resource::decode_retire(frame),
            AgentCallOperation::DeriveCapability => capability::decode_derive(frame),
            AgentCallOperation::RevokeDerivedCapability => capability::decode_revoke(frame),
            AgentCallOperation::DeclareIntent => task_lifecycle::decode_declare(frame),
            AgentCallOperation::CreateTask => task_lifecycle::decode_create(frame),
            AgentCallOperation::DelegateTask => task_lifecycle::decode_delegate(frame),
            AgentCallOperation::RegisterManagedAgent => agent_management::decode_register(frame),
            AgentCallOperation::SuspendManagedAgent
            | AgentCallOperation::ResumeManagedAgent
            | AgentCallOperation::RetireManagedAgent => {
                agent_management::decode_lifecycle(frame, operation)
            }
            AgentCallOperation::AllocateMemoryPage => memory_page::decode_allocate(frame),
            AgentCallOperation::InspectMemoryPage | AgentCallOperation::ReleaseMemoryPage => {
                memory_page::decode_existing(frame, operation)
            }
            AgentCallOperation::AllocateMemoryRegion => memory_region::decode_allocate(frame),
            AgentCallOperation::InspectMemoryRegion | AgentCallOperation::ReleaseMemoryRegion => {
                memory_region::decode_existing(frame, operation)
            }
            AgentCallOperation::RequestRuntimeAdmission => runtime_admission::decode_request(frame),
            AgentCallOperation::DiscoverRuntimeAdmission => {
                runtime_admission::decode_discovery(frame)
            }
            AgentCallOperation::CompactRuntimeAdmissions => {
                runtime_admission::decode_compaction(frame)
            }
            AgentCallOperation::CompactTasks => task_compaction::decode(frame),
            AgentCallOperation::CompactIntents => intent_compaction::decode(frame),
            AgentCallOperation::CompactCapability => capability_compaction::decode(frame),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AgentCallDecodeError {
    InvalidMagic,
    UnsupportedVersion,
    UnsupportedOperation,
    UnsupportedFlags,
    ReservedNotZero,
    InvalidPayload,
    RuntimeAdmissionContextUnavailable,
}

fn ensure_reserved_zero(frame: &PrivilegeInterruptStackFrame) -> Result<(), AgentCallDecodeError> {
    if frame.r10 == 0 && frame.r11 == 0 {
        ensure_extended_reserved_zero(frame)
    } else {
        Err(AgentCallDecodeError::ReservedNotZero)
    }
}

fn ensure_extended_reserved_zero(
    frame: &PrivilegeInterruptStackFrame,
) -> Result<(), AgentCallDecodeError> {
    if frame.r12 == 0 && frame.r13 == 0 && frame.r14 == 0 && frame.r15 == 0 && frame.rbp == 0 {
        Ok(())
    } else {
        Err(AgentCallDecodeError::ReservedNotZero)
    }
}

fn decode_context_payload(
    frame: &PrivilegeInterruptStackFrame,
) -> Result<(AgentId, TaskId, AgentImageId, u64), AgentCallDecodeError> {
    if frame.rsi == 0 || frame.rdi == 0 || frame.r8 == 0 || frame.r9 == 0 {
        Err(AgentCallDecodeError::InvalidPayload)
    } else {
        Ok((
            AgentId::new(frame.rsi),
            TaskId::new(frame.rdi),
            AgentImageId::new(frame.r8),
            frame.r9,
        ))
    }
}

fn decode_verifier_payload(
    frame: &PrivilegeInterruptStackFrame,
) -> Result<(AgentId, TaskId, AgentImageId, u64, TaskId), AgentCallDecodeError> {
    if frame.r11 != 0 {
        return Err(AgentCallDecodeError::ReservedNotZero);
    }
    ensure_extended_reserved_zero(frame)?;
    let (agent, task, image, nonce) = decode_context_payload(frame)?;
    if frame.r10 == 0 {
        return Err(AgentCallDecodeError::InvalidPayload);
    }
    Ok((agent, task, image, nonce, TaskId::new(frame.r10)))
}
