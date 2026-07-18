//! Typed native Agent Call requests and operation identities.
//!
//! This x86 architecture module owns the copyable request vocabulary shared by
//! strict register decoders, scheduler authentication, transcript capture, and
//! the bare-metal executor. It performs no mutation or privileged operation.

use agent_kernel_core::{
    AgentId, AgentImageId, CapabilityId, IntentId, IntentKind, MessageId, MessageKind,
    MessagePayload, OperationSet, ResourceId, ResourceKind, TaskId, TaskResult,
    VerificationRequirement,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AgentCallOperation {
    DescribeContext,
    Yield,
    CompleteTask,
    SubmitTaskResult,
    InspectTaskResult,
    VerifyTask,
    SendMessage,
    ReceiveMessage,
    AcknowledgeMessage,
    CreateResource,
    RetireResource,
    DeriveCapability,
    RevokeDerivedCapability,
    DeclareIntent,
    CreateTask,
    DelegateTask,
    RegisterManagedAgent,
    SuspendManagedAgent,
    ResumeManagedAgent,
    RetireManagedAgent,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AgentCallRequest {
    DescribeContext {
        nonce: u64,
    },
    Yield {
        agent: AgentId,
        task: TaskId,
        image: AgentImageId,
        nonce: u64,
    },
    CompleteTask {
        agent: AgentId,
        task: TaskId,
        image: AgentImageId,
        nonce: u64,
    },
    SubmitTaskResult {
        agent: AgentId,
        task: TaskId,
        image: AgentImageId,
        nonce: u64,
        result: TaskResult,
    },
    InspectTaskResult {
        agent: AgentId,
        task: TaskId,
        image: AgentImageId,
        nonce: u64,
        target_task: TaskId,
    },
    VerifyTask {
        agent: AgentId,
        task: TaskId,
        image: AgentImageId,
        nonce: u64,
        target_task: TaskId,
    },
    SendMessage {
        agent: AgentId,
        task: TaskId,
        image: AgentImageId,
        nonce: u64,
        recipient: AgentId,
        kind: MessageKind,
        payload: MessagePayload,
    },
    ReceiveMessage {
        agent: AgentId,
        task: TaskId,
        image: AgentImageId,
        nonce: u64,
    },
    AcknowledgeMessage {
        agent: AgentId,
        task: TaskId,
        image: AgentImageId,
        nonce: u64,
        message: MessageId,
    },
    CreateResource {
        agent: AgentId,
        task: TaskId,
        image: AgentImageId,
        nonce: u64,
        authority: CapabilityId,
        parent: ResourceId,
        kind: ResourceKind,
        operations: OperationSet,
    },
    RetireResource {
        agent: AgentId,
        task: TaskId,
        image: AgentImageId,
        nonce: u64,
        resource: ResourceId,
        capability: CapabilityId,
    },
    DeriveCapability {
        agent: AgentId,
        task: TaskId,
        image: AgentImageId,
        nonce: u64,
        source: CapabilityId,
        target: AgentId,
        operations: OperationSet,
    },
    RevokeDerivedCapability {
        agent: AgentId,
        task: TaskId,
        image: AgentImageId,
        nonce: u64,
        source: CapabilityId,
        target: CapabilityId,
    },
    DeclareIntent {
        agent: AgentId,
        task: TaskId,
        image: AgentImageId,
        nonce: u64,
        authority: CapabilityId,
        resource: ResourceId,
        kind: IntentKind,
        verification: VerificationRequirement,
    },
    CreateTask {
        agent: AgentId,
        task: TaskId,
        image: AgentImageId,
        nonce: u64,
        authority: CapabilityId,
        intent: IntentId,
    },
    DelegateTask {
        agent: AgentId,
        task: TaskId,
        image: AgentImageId,
        nonce: u64,
        authority: CapabilityId,
        delegated_task: TaskId,
        target: AgentId,
    },
    RegisterManagedAgent {
        agent: AgentId,
        task: TaskId,
        image: AgentImageId,
        nonce: u64,
        authority: CapabilityId,
        resource: ResourceId,
        target: AgentId,
    },
    SuspendManagedAgent {
        agent: AgentId,
        task: TaskId,
        image: AgentImageId,
        nonce: u64,
        authority: CapabilityId,
        target: AgentId,
    },
    ResumeManagedAgent {
        agent: AgentId,
        task: TaskId,
        image: AgentImageId,
        nonce: u64,
        authority: CapabilityId,
        target: AgentId,
    },
    RetireManagedAgent {
        agent: AgentId,
        task: TaskId,
        image: AgentImageId,
        nonce: u64,
        authority: CapabilityId,
        target: AgentId,
    },
}

impl AgentCallRequest {
    pub const fn operation(self) -> AgentCallOperation {
        match self {
            Self::DescribeContext { .. } => AgentCallOperation::DescribeContext,
            Self::Yield { .. } => AgentCallOperation::Yield,
            Self::CompleteTask { .. } => AgentCallOperation::CompleteTask,
            Self::SubmitTaskResult { .. } => AgentCallOperation::SubmitTaskResult,
            Self::InspectTaskResult { .. } => AgentCallOperation::InspectTaskResult,
            Self::VerifyTask { .. } => AgentCallOperation::VerifyTask,
            Self::SendMessage { .. } => AgentCallOperation::SendMessage,
            Self::ReceiveMessage { .. } => AgentCallOperation::ReceiveMessage,
            Self::AcknowledgeMessage { .. } => AgentCallOperation::AcknowledgeMessage,
            Self::CreateResource { .. } => AgentCallOperation::CreateResource,
            Self::RetireResource { .. } => AgentCallOperation::RetireResource,
            Self::DeriveCapability { .. } => AgentCallOperation::DeriveCapability,
            Self::RevokeDerivedCapability { .. } => AgentCallOperation::RevokeDerivedCapability,
            Self::DeclareIntent { .. } => AgentCallOperation::DeclareIntent,
            Self::CreateTask { .. } => AgentCallOperation::CreateTask,
            Self::DelegateTask { .. } => AgentCallOperation::DelegateTask,
            Self::RegisterManagedAgent { .. } => AgentCallOperation::RegisterManagedAgent,
            Self::SuspendManagedAgent { .. } => AgentCallOperation::SuspendManagedAgent,
            Self::ResumeManagedAgent { .. } => AgentCallOperation::ResumeManagedAgent,
            Self::RetireManagedAgent { .. } => AgentCallOperation::RetireManagedAgent,
        }
    }
}
