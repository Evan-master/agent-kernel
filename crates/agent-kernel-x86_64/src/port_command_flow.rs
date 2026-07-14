//! Kernel-dispatched bare-metal Port command proof.
//!
//! This x86 boot-layer adapter creates the minimal Driver Agent admission and
//! binding state, obtains an immutable command request from the kernel, executes
//! it through native Port I/O, and returns the outcome through a terminal
//! command syscall. It never constructs request identity or mutates records.

use agent_kernel_core::{
    AgentEntryKind, AgentId, AgentImageDigest, AgentImageKind, CapabilityId, DriverCommandId,
    DriverCommandKind, DriverCommandPayload, DriverCommandRequest, DriverCommandStatus,
    DriverEndpointDescriptor, Operation, OperationSet,
};
use agent_kernel_hal::{DriverBackend, DriverCommandOutcome};
use agent_kernel_x86_64::{port::PortIoBackend, NativePortIo};

use crate::X86BootedKernel;

const DRIVER: AgentId = AgentId::new(2);

pub struct PortCommandFlow {
    backend: PortIoBackend<NativePortIo>,
    driver_capability: CapabilityId,
    command: DriverCommandId,
    request: DriverCommandRequest,
}

impl PortCommandFlow {
    pub fn prepare(booted: &mut X86BootedKernel, base: u16, value: u8) -> Option<Self> {
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
                OperationSet::only(Operation::Act),
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
                DriverCommandKind::Write,
                DriverCommandPayload {
                    opcode: 0,
                    value: u64::from(value),
                },
            )
            .ok()?;
        let request = kernel
            .sys_dispatch_driver_command(DRIVER, driver_capability, command)
            .ok()?;
        let endpoint = kernel.driver_endpoint(report.bootstrap_resource).ok()?;
        let io = unsafe { NativePortIo::new() };
        let backend = PortIoBackend::new(endpoint, io).ok()?;

        Some(Self {
            backend,
            driver_capability,
            command,
            request,
        })
    }

    pub fn execute_and_record(&mut self, booted: &mut X86BootedKernel) -> bool {
        let outcome = self.backend.execute(self.request);
        let result = outcome.result();
        let (status, transition) = match outcome {
            DriverCommandOutcome::Completed(_) => (
                DriverCommandStatus::Completed,
                booted.kernel_mut().sys_complete_driver_command(
                    DRIVER,
                    self.driver_capability,
                    self.command,
                    result,
                ),
            ),
            DriverCommandOutcome::Failed(_) => (
                DriverCommandStatus::Failed,
                booted.kernel_mut().sys_fail_driver_command(
                    DRIVER,
                    self.driver_capability,
                    self.command,
                    result,
                ),
            ),
        };
        if transition.is_err() {
            return false;
        }

        let Some(record) = booted
            .kernel()
            .driver_commands()
            .iter()
            .find(|record| record.id == self.command)
        else {
            return false;
        };
        matches!(outcome, DriverCommandOutcome::Completed(_))
            && record.status == status
            && record.result == Some(result)
    }
}
