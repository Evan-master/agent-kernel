//! Reusable host-side adapters for Agent Kernel supervisor flows.
//!
//! The binary owns demonstration orchestration. This library exposes bounded,
//! deterministic adapters that integration tests and future supervisor flows
//! can drive without moving host behavior into kernel crates.

pub mod durable_state_backend;
