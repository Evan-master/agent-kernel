//! Capability-bound physical command flow for the claimed PCI serial BAR.
//!
//! This bare-metal x86 adapter composes Core admission and immutable Driver
//! records with the architecture backend. Native I/O starts only after the
//! exact claim, delegated Capability, endpoint, and command all agree.

mod admission;
mod evidence;

use agent_kernel_core::{
    AgentId, AgentImageId, DeviceEventKind, DeviceEventPayload, DriverCommandKind,
    DriverCommandPayload, DriverCommandResult,
};
use agent_kernel_hal::{DriverBackend, DriverCommandOutcome};
use agent_kernel_x86_64::{
    pci::{PciBarKind, PciFunctionClaim},
    pci_serial::{PciSerialBackend, PCI_SERIAL_RESULT_OK},
    NativePortIo,
};

use crate::{
    pci_serial_profile, port_driver_flow::record_command_outcome, serial_write_line,
    X86BootedKernel,
};

const INVOCATION_QUANTUM: u64 = 2;
const PCI_COMMAND_IO_SPACE_ENABLE: u16 = 1;
const DRIVER: AgentId = AgentId::new(10);
const RECLAIMABLE_IMAGE: AgentImageId = AgentImageId::new(15);

pub(crate) fn run(booted: &mut X86BootedKernel, claim: PciFunctionClaim) -> Option<()> {
    let selector = pci_serial_profile::selector()?;
    let function = claim.function();
    if !selector.matches(function) || function.command() & PCI_COMMAND_IO_SPACE_ENABLE == 0 {
        return None;
    }

    let bar_index = pci_serial_profile::bar_index()?;
    let bar = claim.bars().get(bar_index)?;
    if bar.kind() != PciBarKind::Io || bar.size() != pci_serial_profile::BAR_SPAN {
        return None;
    }
    let region = claim.bar_region(bar_index)?;
    if region.descriptor().base != bar.base()
        || region.descriptor().span != bar.size()
        || region.slot() != bar_index.number()
    {
        return None;
    }

    let report = *booted.report();
    let admission = admission::prepare(booted, region)?;
    let driver_capability = admission.capability;
    let binding = admission.binding;
    // SAFETY: Core returned the immutable endpoint for the capability-bound BAR.
    let mut backend = PciSerialBackend::new(
        admission.endpoint,
        unsafe { NativePortIo::new() },
        pci_serial_profile::TRANSMIT_POLL_BUDGET,
    )
    .ok()?;

    let event = booted
        .kernel_mut()
        .sys_raise_device_event(
            report.bootstrap_agent,
            region.capability(),
            region.resource(),
            DeviceEventKind::StateChanged,
            DeviceEventPayload {
                code: pci_serial_profile::VENDOR_ID,
                value: u64::from(pci_serial_profile::DEVICE_ID),
            },
        )
        .ok()?;
    let invocation = booted
        .kernel_mut()
        .sys_deliver_device_event(DRIVER, driver_capability, event)
        .ok()?;
    if booted
        .kernel_mut()
        .sys_dispatch_next_driver_invocation(DRIVER, INVOCATION_QUANTUM)
        .ok()?
        != invocation
    {
        return None;
    }
    booted
        .kernel_mut()
        .sys_tick_driver_invocation(DRIVER, invocation)
        .ok()?;
    booted
        .kernel_mut()
        .sys_acknowledge_device_event(DRIVER, driver_capability, event)
        .ok()?;

    let command = booted
        .kernel_mut()
        .sys_submit_driver_command(
            DRIVER,
            driver_capability,
            region.resource(),
            Some(event),
            DriverCommandKind::Write,
            DriverCommandPayload {
                opcode: 0,
                value: u64::from(pci_serial_profile::TRANSMIT_BYTE),
            },
        )
        .ok()?;
    let request = booted
        .kernel_mut()
        .sys_dispatch_driver_command(DRIVER, driver_capability, command)
        .ok()?;
    if request.command != command
        || request.binding != binding
        || request.resource != region.resource()
        || request.driver != DRIVER
        || request.cause != Some(event)
        || request.invocation != Some(invocation)
        || request.kind != DriverCommandKind::Write
        || request.payload.opcode != 0
        || request.payload.value != u64::from(pci_serial_profile::TRANSMIT_BYTE)
    {
        return None;
    }

    let outcome = backend.execute(request);
    let result = outcome.result();
    if !record_command_outcome(booted, DRIVER, driver_capability, command, outcome)
        || !matches!(outcome, DriverCommandOutcome::Completed(_))
        || result
            != (DriverCommandResult {
                code: PCI_SERIAL_RESULT_OK,
                value: u64::from(pci_serial_profile::TRANSMIT_BYTE),
            })
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_PCI_SERIAL_PHYSICAL_IO_OK");
    booted
        .kernel_mut()
        .sys_complete_driver_invocation(DRIVER, driver_capability, invocation)
        .ok()?;

    if !evidence::terminal_matches(
        booted,
        evidence::TerminalEvidence {
            driver: DRIVER,
            resource: region.resource(),
            binding,
            event,
            command,
            invocation,
            result,
        },
    ) {
        return None;
    }

    serial_write_line("AGENT_KERNEL_PCI_SERIAL_DRIVER_OK");
    Some(())
}
