//! Allocator-free CPU topology and lifecycle contracts.
//!
//! The x86_64 architecture layer owns logical CPU identity, firmware topology,
//! online membership, and boot lifecycle. Hardware table parsing and AP startup
//! consume these types without leaking firmware identifiers into Agent state.

mod identity;
mod mask;
mod registry;
mod topology;

pub use identity::{
    ApicId, CpuIndex, FirmwareCpuFlags, FirmwareProcessor, ProcessorSource, MAX_CPU_COUNT,
};
pub use mask::CpuMask;
pub use registry::{CpuLifecycleState, CpuRegistry, CpuRegistryError};
pub use topology::{CpuDescriptor, CpuTopology, CpuTopologyBuilder, TopologyError, TopologyInsert};
