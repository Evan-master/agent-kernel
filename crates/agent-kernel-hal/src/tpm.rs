//! Allocation-free TPM command transport boundary.
//!
//! The HAL owns only one synchronous command/response contract. Architecture
//! drivers retain locality, register, timeout, and interrupt policy while
//! callers retain both buffers and all TPM wire interpretation.

pub trait TpmCommandTransport {
    type Error;

    fn execute(&mut self, command: &[u8], response: &mut [u8]) -> Result<usize, Self::Error>;
}
