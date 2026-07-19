//! Stable operation identities for the native Agent Call transcript.
//!
//! This architecture-library module owns the copyable operation vocabulary
//! shared by request decoding, context authentication, and exact execution
//! evidence. Numeric wire values remain in the parent ABI module.

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
    AllocateMemoryPage,
    InspectMemoryPage,
    ReleaseMemoryPage,
    AllocateMemoryRegion,
    InspectMemoryRegion,
    ReleaseMemoryRegion,
    RequestRuntimeAdmission,
    DiscoverRuntimeAdmission,
    CompactRuntimeAdmissions,
    CompactTasks,
    CompactIntents,
    CompactCapability,
    RetireAgentEntry,
    RetireMessage,
}
