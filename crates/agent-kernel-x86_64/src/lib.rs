#![no_std]
//! x86_64 architecture adapters for Agent Kernel.
//!
//! This crate owns bounded hardware endpoint execution. Architecture-neutral
//! authorization and command state remain in the core and facade crates.

pub mod acpi_topology;
pub mod address_space;
pub mod address_space_reclamation;
pub mod agent_call;
pub mod agent_image;
pub mod apic;
pub mod ata;
pub mod context;
pub mod cpu;
pub mod durable_state;
pub mod interrupt;
pub mod namespace_path_buffer;
pub mod native_runtime;
pub mod per_cpu;
pub mod port;
pub mod privilege;
pub mod runtime_frame_pool;
pub mod runtime_page;
pub mod runtime_reclamation;
pub mod runtime_region;
pub mod sync;
pub mod tlb;
pub mod typed_call_data;
pub mod user_memory;

mod namespace_object_wire;

#[cfg(target_arch = "x86_64")]
mod native_port_io;

#[cfg(target_arch = "x86_64")]
pub use native_port_io::NativePortIo;
