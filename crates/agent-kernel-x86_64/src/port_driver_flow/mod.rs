//! Interrupt-driven physical Port flow for one kernel-owned Driver Invocation.
//!
//! This x86 boot-layer adapter admits the Driver Agent, converts a validated
//! architecture interrupt signal into a Device Event, and runs a causal write
//! to terminal command and invocation records. It owns native Port authority
//! but never constructs request identity or edits kernel state directly.

mod setup;
mod terminal;

use agent_kernel_core::{
    AgentId, CapabilityId, DeviceEventId, DriverCommandId, DriverCommandRequest, DriverInvocationId,
};
use agent_kernel_x86_64::{port::PortIoBackend, NativePortIo};

pub(super) const DRIVER: AgentId = AgentId::new(2);
pub(super) const INVOCATION_QUANTUM: u64 = 2;

pub struct PortDriverSetup {
    backend: PortIoBackend<NativePortIo>,
    driver_capability: CapabilityId,
}

pub struct PortCommandFlow {
    backend: PortIoBackend<NativePortIo>,
    driver_capability: CapabilityId,
    event: DeviceEventId,
    invocation: DriverInvocationId,
    command: DriverCommandId,
    request: DriverCommandRequest,
}
