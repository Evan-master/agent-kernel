#![no_std]
//! x86_64 architecture adapters for Agent Kernel.
//!
//! This crate owns bounded hardware endpoint execution. Architecture-neutral
//! authorization and command state remain in the core and facade crates.

pub mod address_space;
pub mod agent_call;
pub mod agent_image;
pub mod context;
pub mod interrupt;
pub mod port;
pub mod privilege;
pub mod user_memory;

#[cfg(target_arch = "x86_64")]
mod native_port_io;

#[cfg(target_arch = "x86_64")]
pub use native_port_io::NativePortIo;
