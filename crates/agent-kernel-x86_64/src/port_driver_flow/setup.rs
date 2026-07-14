//! Driver admission, physical status polling, and invocation dispatch phase.

use agent_kernel_core::{
    AgentEntryKind, AgentImageDigest, AgentImageKind, DeviceEventKind, DeviceEventPayload,
    DriverCommandKind, DriverCommandPayload, DriverEndpointDescriptor, Operation, OperationSet,
};
use agent_kernel_hal::{DriverBackend, DriverCommandOutcome};
use agent_kernel_x86_64::{
    port::{PortIoBackend, PORT_IO_RESULT_OK},
    NativePortIo,
};

use super::{
    terminal::record_command_outcome, PortCommandFlow, PortPoll, DRIVER, INVOCATION_QUANTUM,
    LINE_STATUS_OFFSET, TRANSMITTER_EMPTY,
};
use crate::X86BootedKernel;

impl PortPoll {
    pub fn prepare(booted: &mut X86BootedKernel, base: u16) -> Option<Self> {
        let report = *booted.report();
        let kernel = booted.kernel_mut();
        kernel
            .sys_register_driver_endpoint(
                report.bootstrap_agent,
                report.bootstrap_capability,
                report.bootstrap_resource,
                DriverEndpointDescriptor::port(u64::from(base), 8),
            )
            .ok()?;
        kernel.sys_register_agent(DRIVER).ok()?;
        let driver_capability = kernel
            .sys_derive_capability(
                report.bootstrap_agent,
                report.bootstrap_capability,
                DRIVER,
                OperationSet::empty()
                    .with(Operation::Observe)
                    .with(Operation::Act),
            )
            .ok()?;
        let image = kernel
            .sys_register_agent_image(
                report.bootstrap_agent,
                report.bootstrap_capability,
                report.bootstrap_resource,
                AgentImageKind::Driver,
                AgentImageDigest::new([0x44; 32]),
                1,
                1,
            )
            .ok()?;
        kernel
            .sys_verify_agent_image(report.bootstrap_agent, report.bootstrap_capability, image)
            .ok()?;
        kernel
            .sys_launch_agent(
                DRIVER,
                driver_capability,
                report.bootstrap_resource,
                image,
                AgentEntryKind::Driver,
                None,
            )
            .ok()?;
        kernel
            .sys_bind_driver(
                report.bootstrap_agent,
                report.bootstrap_capability,
                report.bootstrap_resource,
                DRIVER,
            )
            .ok()?;
        let command = kernel
            .sys_submit_driver_command(
                DRIVER,
                driver_capability,
                report.bootstrap_resource,
                None,
                DriverCommandKind::Read,
                DriverCommandPayload {
                    opcode: LINE_STATUS_OFFSET,
                    value: 0,
                },
            )
            .ok()?;
        let request = kernel
            .sys_dispatch_driver_command(DRIVER, driver_capability, command)
            .ok()?;
        let endpoint = kernel.driver_endpoint(report.bootstrap_resource).ok()?;
        // SAFETY: the ring-0 boot adapter binds authority only to this validated endpoint.
        let backend = PortIoBackend::new(endpoint, unsafe { NativePortIo::new() }).ok()?;

        Some(Self {
            backend,
            driver_capability,
            command,
            request,
        })
    }

    pub fn poll_and_dispatch(
        mut self,
        booted: &mut X86BootedKernel,
        value: u8,
    ) -> Option<PortCommandFlow> {
        let outcome = self.backend.execute(self.request);
        let result = outcome.result();
        if !record_command_outcome(booted, self.driver_capability, self.command, outcome)
            || !matches!(outcome, DriverCommandOutcome::Completed(_))
            || result.code != PORT_IO_RESULT_OK
            || result.value & TRANSMITTER_EMPTY == 0
        {
            return None;
        }

        let report = *booted.report();
        let kernel = booted.kernel_mut();
        let event = kernel
            .sys_raise_device_event(
                report.bootstrap_agent,
                report.bootstrap_capability,
                report.bootstrap_resource,
                DeviceEventKind::StateChanged,
                DeviceEventPayload {
                    code: LINE_STATUS_OFFSET,
                    value: result.value,
                },
            )
            .ok()?;
        let invocation = kernel
            .sys_deliver_device_event(DRIVER, self.driver_capability, event)
            .ok()?;
        if kernel
            .sys_dispatch_next_driver_invocation(DRIVER, INVOCATION_QUANTUM)
            .ok()?
            != invocation
        {
            return None;
        }
        kernel.sys_tick_driver_invocation(DRIVER, invocation).ok()?;
        kernel
            .sys_acknowledge_device_event(DRIVER, self.driver_capability, event)
            .ok()?;
        let command = kernel
            .sys_submit_driver_command(
                DRIVER,
                self.driver_capability,
                report.bootstrap_resource,
                Some(event),
                DriverCommandKind::Write,
                DriverCommandPayload {
                    opcode: 0,
                    value: u64::from(value),
                },
            )
            .ok()?;
        let request = kernel
            .sys_dispatch_driver_command(DRIVER, self.driver_capability, command)
            .ok()?;
        if request.cause != Some(event) || request.invocation != Some(invocation) {
            return None;
        }

        Some(PortCommandFlow {
            backend: self.backend,
            driver_capability: self.driver_capability,
            event,
            invocation,
            command,
            request,
        })
    }
}
