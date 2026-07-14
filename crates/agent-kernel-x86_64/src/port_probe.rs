//! Bare-metal proof adapter for the bounded port backend.
//!
//! This x86 boot-layer module registers one architecture endpoint through the
//! kernel facade, constructs native I/O authority from the resulting record,
//! and emits one byte through the normal HAL backend contract.

use agent_kernel_core::{
    DriverBindingId, DriverCommandId, DriverCommandKind, DriverCommandPayload,
    DriverCommandRequest, DriverCommandResult, DriverEndpointDescriptor,
};
use agent_kernel_hal::{DriverBackend, DriverCommandOutcome};
use agent_kernel_x86_64::{
    port::{PortIoBackend, PORT_IO_RESULT_OK},
    NativePortIo,
};

use crate::X86BootedKernel;

pub struct PortProbe {
    backend: PortIoBackend<NativePortIo>,
    request: DriverCommandRequest,
}

impl PortProbe {
    pub fn prepare(booted: &mut X86BootedKernel, base: u16) -> Option<Self> {
        let report = *booted.report();
        booted
            .kernel_mut()
            .sys_register_driver_endpoint(
                report.bootstrap_agent,
                report.bootstrap_capability,
                report.bootstrap_resource,
                DriverEndpointDescriptor::port(u64::from(base), 8),
            )
            .ok()?;
        let endpoint = booted
            .kernel()
            .driver_endpoint(report.bootstrap_resource)
            .ok()?;
        let io = unsafe { NativePortIo::new() };
        let backend = PortIoBackend::new(endpoint, io).ok()?;
        let request = DriverCommandRequest {
            command: DriverCommandId::new(1),
            binding: DriverBindingId::new(1),
            resource: report.bootstrap_resource,
            driver: report.bootstrap_agent,
            cause: None,
            invocation: None,
            kind: DriverCommandKind::Write,
            payload: DriverCommandPayload {
                opcode: 0,
                value: 0,
            },
        };
        Some(Self { backend, request })
    }

    pub fn write_byte(&mut self, value: u8) -> bool {
        self.request.payload.value = u64::from(value);
        self.backend.execute(self.request)
            == DriverCommandOutcome::Completed(DriverCommandResult {
                code: PORT_IO_RESULT_OK,
                value: u64::from(value),
            })
    }
}
