//! Validated, allocator-free ACPI machine topology discovery.
//!
//! The x86_64 architecture layer uses upstream ACPI table discovery, then
//! converts MADT bytes through a bounded parser before firmware identities or
//! interrupt routes enter kernel state.

mod discover;
mod handler;
mod parser;
mod types;

pub use discover::load_acpi_topology;
pub use handler::DirectAcpiHandler;
pub use parser::parse_madt;
pub use types::{
    AcpiMachineTopology, AcpiTopologyError, InterruptPolarity, InterruptSourceOverride,
    InterruptTrigger, IoApicDescriptor, MAX_INTERRUPT_OVERRIDES, MAX_IO_APICS,
};
