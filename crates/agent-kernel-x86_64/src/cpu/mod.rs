//! Allocator-free CPU topology and lifecycle contracts.
//!
//! The x86_64 architecture layer owns logical CPU identity, firmware topology,
//! online membership, and boot lifecycle. Hardware table parsing and AP startup
//! consume these types without leaking firmware identifiers into Agent state.

mod identity;
mod mask;
mod registry;
mod startup;
mod topology;

pub use identity::{
    ApicId, CpuIndex, FirmwareCpuFlags, FirmwareProcessor, ProcessorSource, MAX_CPU_COUNT,
};
pub use mask::{CpuMask, CPU_MASK_WORD_COUNT};
pub use registry::{CpuLifecycleState, CpuRegistry, CpuRegistryError};
pub use startup::{
    ApStartupDescriptor, ApStartupEvidence, ApStartupHandoff, ApStartupHandoffError,
    ApStartupStatus, AP_HANDOFF_APIC_ID_OFFSET, AP_HANDOFF_CPU_INDEX_OFFSET, AP_HANDOFF_CR3_OFFSET,
    AP_HANDOFF_ENTRY_OFFSET, AP_HANDOFF_GENERATION_OFFSET, AP_HANDOFF_LOCAL_APIC_BASE_OFFSET,
    AP_HANDOFF_PHYSICAL_OFFSET_OFFSET, AP_HANDOFF_STACK_TOP_OFFSET, AP_HANDOFF_STATUS_OFFSET,
    AP_STARTUP_STATUS_PREPARED,
};
pub use topology::{CpuDescriptor, CpuTopology, CpuTopologyBuilder, TopologyError, TopologyInsert};
