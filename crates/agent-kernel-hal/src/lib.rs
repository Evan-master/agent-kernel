#![no_std]
//! Architecture-neutral device backend contract for Agent Kernel drivers.
//!
//! The kernel produces authorized immutable requests. Implementations execute
//! the external side effect and return a fixed-width outcome for the runtime to
//! report back through the kernel facade.

use agent_kernel_core::{DriverCommandRequest, DriverCommandResult};

mod durable_state;
mod tpm;

pub use durable_state::{
    DurableFlush, DurableSlotReadback, DurableSlotRegion, DurableSlotTarget,
    DurableSlotTargetError, DurableSlotWrite, DurableSlotWriteError, DurableStateBackend,
    DurableStateBackendError, DURABLE_SLOT_BODY_BYTES, DURABLE_SLOT_BYTES,
    DURABLE_SLOT_FOOTER_BYTES, DURABLE_SLOT_HEADER_BYTES,
};
pub use tpm::TpmCommandTransport;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DriverCommandOutcome {
    Completed(DriverCommandResult),
    Failed(DriverCommandResult),
}

impl DriverCommandOutcome {
    pub const fn result(self) -> DriverCommandResult {
        match self {
            Self::Completed(result) | Self::Failed(result) => result,
        }
    }
}

pub trait DriverBackend {
    fn execute(&mut self, request: DriverCommandRequest) -> DriverCommandOutcome;
}
