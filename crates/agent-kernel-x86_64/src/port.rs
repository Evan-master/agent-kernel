//! Bounded byte-wide x86 port backend.
//!
//! This architecture-layer module translates relative Driver command offsets
//! within a validated `Port` endpoint. It owns no kernel authority and performs
//! no I/O until resource, command, range, and value checks have succeeded.

use agent_kernel_core::{
    DriverCommandKind, DriverCommandRequest, DriverCommandResult, DriverEndpointKind,
    DriverEndpointRecord, ResourceId,
};
use agent_kernel_hal::{DriverBackend, DriverCommandOutcome};

pub const PORT_IO_RESULT_OK: u16 = 0;
pub const PORT_IO_RESULT_RESOURCE_MISMATCH: u16 = 1;
pub const PORT_IO_RESULT_OFFSET_OUT_OF_RANGE: u16 = 2;
pub const PORT_IO_RESULT_VALUE_OUT_OF_RANGE: u16 = 3;
pub const PORT_IO_RESULT_UNSUPPORTED_COMMAND: u16 = 4;

pub trait PortIo {
    fn read_u8(&mut self, port: u16) -> u8;

    fn write_u8(&mut self, port: u16, value: u8);
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PortIoBackendError {
    EndpointKindMismatch,
    EndpointDescriptorInvalid,
}

pub struct PortIoBackend<I> {
    resource: ResourceId,
    base: u16,
    span: u64,
    io: I,
}

impl<I> PortIoBackend<I> {
    pub fn new(endpoint: DriverEndpointRecord, io: I) -> Result<Self, PortIoBackendError> {
        let descriptor = endpoint.descriptor;
        if descriptor.kind != DriverEndpointKind::Port {
            return Err(PortIoBackendError::EndpointKindMismatch);
        }
        let end = descriptor
            .span
            .checked_sub(1)
            .and_then(|last_offset| descriptor.base.checked_add(last_offset))
            .ok_or(PortIoBackendError::EndpointDescriptorInvalid)?;
        if end > u16::MAX as u64 {
            return Err(PortIoBackendError::EndpointDescriptorInvalid);
        }

        Ok(Self {
            resource: endpoint.resource,
            base: descriptor.base as u16,
            span: descriptor.span,
            io,
        })
    }

    pub const fn resource(&self) -> ResourceId {
        self.resource
    }

    pub const fn base(&self) -> u16 {
        self.base
    }

    pub const fn span(&self) -> u64 {
        self.span
    }

    pub const fn io(&self) -> &I {
        &self.io
    }

    pub fn into_io(self) -> I {
        self.io
    }

    fn resolve_port(&self, offset: u16) -> Option<u16> {
        let offset = u64::from(offset);
        if offset >= self.span {
            return None;
        }
        Some((u64::from(self.base) + offset) as u16)
    }

    const fn failed(code: u16) -> DriverCommandOutcome {
        DriverCommandOutcome::Failed(DriverCommandResult { code, value: 0 })
    }
}

impl<I: PortIo> DriverBackend for PortIoBackend<I> {
    fn execute(&mut self, request: DriverCommandRequest) -> DriverCommandOutcome {
        if request.resource != self.resource {
            return Self::failed(PORT_IO_RESULT_RESOURCE_MISMATCH);
        }
        if !matches!(
            request.kind,
            DriverCommandKind::Read | DriverCommandKind::Write
        ) {
            return Self::failed(PORT_IO_RESULT_UNSUPPORTED_COMMAND);
        }
        let Some(port) = self.resolve_port(request.payload.opcode) else {
            return Self::failed(PORT_IO_RESULT_OFFSET_OUT_OF_RANGE);
        };

        let value = match request.kind {
            DriverCommandKind::Read => u64::from(self.io.read_u8(port)),
            DriverCommandKind::Write => {
                let Ok(value) = u8::try_from(request.payload.value) else {
                    return Self::failed(PORT_IO_RESULT_VALUE_OUT_OF_RANGE);
                };
                self.io.write_u8(port, value);
                u64::from(value)
            }
            DriverCommandKind::Configure | DriverCommandKind::Reset => {
                return Self::failed(PORT_IO_RESULT_UNSUPPORTED_COMMAND);
            }
        };

        DriverCommandOutcome::Completed(DriverCommandResult {
            code: PORT_IO_RESULT_OK,
            value,
        })
    }
}
