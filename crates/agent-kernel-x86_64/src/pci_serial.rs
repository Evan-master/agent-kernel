//! Capability-bound 16550 backend for a claimed PCI I/O BAR.
//!
//! This x86_64 architecture backend consumes one immutable Driver Command,
//! validates it against the kernel-issued endpoint Resource, performs bounded
//! transmitter readiness polling, and emits at most one native port write.

use agent_kernel_core::{
    DriverCommandKind, DriverCommandRequest, DriverCommandResult, DriverEndpointKind,
    DriverEndpointRecord, ResourceId,
};
use agent_kernel_hal::{DriverBackend, DriverCommandOutcome};

use crate::port::PortIo;

const UART_REGISTER_SPAN: u64 = 8;
const UART_TRANSMIT_OFFSET: u16 = 0;
const UART_INTERRUPT_ENABLE_OFFSET: u16 = 1;
const UART_INTERRUPT_IDENTIFICATION_OFFSET: u16 = 2;
const UART_LINE_STATUS_OFFSET: u16 = 5;
const UART_INTERRUPT_ENABLE_THRE: u8 = 0x02;
const UART_LINE_STATUS_THRE: u8 = 0x20;

pub const PCI_SERIAL_COMMAND_ARM_THRE_INTERRUPT: u16 = 1;
pub const PCI_SERIAL_RESULT_OK: u16 = 0;
pub const PCI_SERIAL_RESULT_RESOURCE_MISMATCH: u16 = 1;
pub const PCI_SERIAL_RESULT_UNSUPPORTED_COMMAND: u16 = 2;
pub const PCI_SERIAL_RESULT_INVALID_REGISTER: u16 = 3;
pub const PCI_SERIAL_RESULT_VALUE_OUT_OF_RANGE: u16 = 4;
pub const PCI_SERIAL_RESULT_TRANSMIT_TIMEOUT: u16 = 5;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PciSerialBackendError {
    EndpointKindMismatch,
    EndpointDescriptorInvalid,
    PollBudgetInvalid,
}

pub struct PciSerialBackend<I> {
    resource: ResourceId,
    base: u16,
    transmit_poll_budget: u32,
    io: I,
}

impl<I> PciSerialBackend<I> {
    pub fn new(
        endpoint: DriverEndpointRecord,
        io: I,
        transmit_poll_budget: u32,
    ) -> Result<Self, PciSerialBackendError> {
        let descriptor = endpoint.descriptor;
        if descriptor.kind != DriverEndpointKind::Port {
            return Err(PciSerialBackendError::EndpointKindMismatch);
        }
        if descriptor.span < UART_REGISTER_SPAN {
            return Err(PciSerialBackendError::EndpointDescriptorInvalid);
        }
        let end = descriptor
            .span
            .checked_sub(1)
            .and_then(|last_offset| descriptor.base.checked_add(last_offset))
            .ok_or(PciSerialBackendError::EndpointDescriptorInvalid)?;
        if end > u64::from(u16::MAX) {
            return Err(PciSerialBackendError::EndpointDescriptorInvalid);
        }
        if transmit_poll_budget == 0 {
            return Err(PciSerialBackendError::PollBudgetInvalid);
        }
        Ok(Self {
            resource: endpoint.resource,
            base: descriptor.base as u16,
            transmit_poll_budget,
            io,
        })
    }

    pub const fn resource(&self) -> ResourceId {
        self.resource
    }

    pub const fn base(&self) -> u16 {
        self.base
    }

    pub const fn transmit_poll_budget(&self) -> u32 {
        self.transmit_poll_budget
    }

    pub const fn io(&self) -> &I {
        &self.io
    }

    pub fn into_io(self) -> I {
        self.io
    }

    const fn failed(code: u16) -> DriverCommandOutcome {
        DriverCommandOutcome::Failed(DriverCommandResult { code, value: 0 })
    }
}

impl<I: PortIo> DriverBackend for PciSerialBackend<I> {
    fn execute(&mut self, request: DriverCommandRequest) -> DriverCommandOutcome {
        if request.resource != self.resource {
            return Self::failed(PCI_SERIAL_RESULT_RESOURCE_MISMATCH);
        }
        match request.kind {
            DriverCommandKind::Configure => self.configure(request),
            DriverCommandKind::Write => self.write(request),
            DriverCommandKind::Read | DriverCommandKind::Reset => {
                Self::failed(PCI_SERIAL_RESULT_UNSUPPORTED_COMMAND)
            }
        }
    }
}

impl<I: PortIo> PciSerialBackend<I> {
    fn configure(&mut self, request: DriverCommandRequest) -> DriverCommandOutcome {
        if request.payload.opcode != PCI_SERIAL_COMMAND_ARM_THRE_INTERRUPT
            || request.payload.value != 0
        {
            return Self::failed(PCI_SERIAL_RESULT_UNSUPPORTED_COMMAND);
        }
        self.io
            .read_u8(self.base + UART_INTERRUPT_IDENTIFICATION_OFFSET);
        self.io.write_u8(
            self.base + UART_INTERRUPT_ENABLE_OFFSET,
            UART_INTERRUPT_ENABLE_THRE,
        );
        DriverCommandOutcome::Completed(DriverCommandResult {
            code: PCI_SERIAL_RESULT_OK,
            value: 0,
        })
    }

    fn write(&mut self, request: DriverCommandRequest) -> DriverCommandOutcome {
        if request.payload.opcode != UART_TRANSMIT_OFFSET {
            return Self::failed(PCI_SERIAL_RESULT_INVALID_REGISTER);
        }
        let Ok(value) = u8::try_from(request.payload.value) else {
            return Self::failed(PCI_SERIAL_RESULT_VALUE_OUT_OF_RANGE);
        };
        let line_status_port = self.base + UART_LINE_STATUS_OFFSET;
        for _ in 0..self.transmit_poll_budget {
            if self.io.read_u8(line_status_port) & UART_LINE_STATUS_THRE != 0 {
                self.io.write_u8(self.base + UART_TRANSMIT_OFFSET, value);
                return DriverCommandOutcome::Completed(DriverCommandResult {
                    code: PCI_SERIAL_RESULT_OK,
                    value: u64::from(value),
                });
            }
        }
        Self::failed(PCI_SERIAL_RESULT_TRANSMIT_TIMEOUT)
    }
}
