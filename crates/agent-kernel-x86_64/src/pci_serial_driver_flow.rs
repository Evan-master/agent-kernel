//! Capability-bound physical command flow for the claimed PCI serial BAR.
//!
//! This bare-metal x86 adapter composes Core admission and immutable Driver
//! records with the architecture backend. Native I/O starts only after the
//! exact claim, delegated Capability, endpoint, and command all agree.

mod admission;
mod evidence;
mod native_invocation;

use agent_kernel_core::{
    AgentId, AgentImageId, DeviceEventKind, DeviceEventPayload, DriverCommandKind,
    DriverCommandPayload, DriverCommandResult, FaultKind,
};
use agent_kernel_x86_64::{
    agent_call::AgentCallContext,
    agent_image::{AgentImageFormat, VerifiedAgentImage},
    pci::{PciBarKind, PciFunctionClaim},
    pci_serial::{PciSerialBackend, PCI_SERIAL_COMMAND_ARM_THRE_INTERRUPT, PCI_SERIAL_RESULT_OK},
    NativePortIo,
};

use crate::{
    agent_cpu::AgentCpuRuntime,
    agent_memory::{NativeAddressSpaceFramePool, RuntimeMemoryPool},
    boot_agent_images,
    native_agent_runtime::NativeAgentRuntime,
    native_driver_executor::DriverRecoveryAuthority,
    pci_serial_interrupt, pci_serial_profile, serial_write_line,
    smp_boot::SmpBootstrap,
    X86BootedKernel,
};

const PCI_COMMAND_IO_SPACE_ENABLE: u16 = 1;
const DRIVER: AgentId = AgentId::new(10);
const RECLAIMABLE_IMAGE: AgentImageId = AgentImageId::new(15);

pub(crate) fn run(
    booted: &mut X86BootedKernel,
    claim: PciFunctionClaim,
    cpu_runtime: &AgentCpuRuntime,
    runtime: &mut NativeAgentRuntime,
    memory_pool: &RuntimeMemoryPool,
    address_space_pool: &mut NativeAddressSpaceFramePool,
    smp: &mut SmpBootstrap,
) -> Option<()> {
    let selector = pci_serial_profile::selector()?;
    let function = claim.function();
    if !selector.matches(function)
        || function.command() & PCI_COMMAND_IO_SPACE_ENABLE == 0
        || function.interrupt_line() != pci_serial_profile::INTERRUPT_LINE
        || function.interrupt_pin() != Some(pci_serial_profile::INTERRUPT_PIN)
    {
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
    let image_contract = boot_agent_images::pci_serial_driver();
    let admission = admission::prepare(booted, region, image_contract)?;
    let driver_capability = admission.capability;
    let binding = admission.binding;
    let image_record = booted.kernel().agent_image(admission.image).ok()?;
    let verified_image = VerifiedAgentImage::verify(image_record, image_contract.bytes()).ok()?;
    if verified_image.format() != AgentImageFormat::CapsuleV1 {
        return None;
    }
    serial_write_line("AGENT_KERNEL_PCI_SERIAL_DRIVER_IMAGE_OK");
    // SAFETY: Core returned the immutable endpoint for the capability-bound BAR.
    let mut backend = PciSerialBackend::new(
        admission.endpoint,
        unsafe { NativePortIo::new() },
        pci_serial_profile::TRANSMIT_POLL_BUDGET,
    )
    .ok()?;
    pci_serial_interrupt::configure(backend.base())?;
    serial_write_line("AGENT_KERNEL_PCI_SERIAL_IRQ_CAPTURE_READY_OK");
    let recovery_authority =
        DriverRecoveryAuthority::new(report.bootstrap_agent, region.capability())?;

    let state_payload = DeviceEventPayload {
        code: pci_serial_profile::VENDOR_ID,
        value: u64::from(pci_serial_profile::DEVICE_ID),
    };
    let state_event = booted
        .kernel_mut()
        .sys_raise_device_event(
            report.bootstrap_agent,
            region.capability(),
            region.resource(),
            DeviceEventKind::StateChanged,
            state_payload,
        )
        .ok()?;
    let state_invocation = booted
        .kernel_mut()
        .sys_deliver_device_event(DRIVER, driver_capability, state_event)
        .ok()?;
    let state_context =
        AgentCallContext::new_driver(DRIVER, state_invocation, admission.image, driver_capability)?;
    serial_write_line("AGENT_KERNEL_PCI_SERIAL_STATE_INVOCATION_READY_OK");
    let state_execution = native_invocation::execute_and_reclaim(
        booted,
        runtime,
        cpu_runtime,
        memory_pool,
        address_space_pool,
        smp,
        verified_image,
        state_context,
        image_contract,
        recovery_authority,
        &mut backend,
    )?;
    let state_fault = state_execution.fault?;
    let state_result = DriverCommandResult {
        code: PCI_SERIAL_RESULT_OK,
        value: 0,
    };
    if state_execution.result != state_result
        || state_execution.dispatches != 4
        || state_execution.quantum_expiries != 2
        || state_execution.restart_generation != 1
        || state_fault.detail() != 6
        || state_fault.offset() != image_contract.expected_fault_offset()
        || state_fault.nonce() != image_contract.nonce()
        || state_fault.physical_quantum_generation() != 1
        || !evidence::terminal_matches(
            booted,
            evidence::TerminalEvidence {
                driver: DRIVER,
                resource: region.resource(),
                binding,
                event: state_event,
                event_kind: DeviceEventKind::StateChanged,
                event_payload: state_payload,
                command: state_execution.command,
                command_kind: DriverCommandKind::Configure,
                command_payload: DriverCommandPayload {
                    opcode: PCI_SERIAL_COMMAND_ARM_THRE_INTERRUPT,
                    value: 0,
                },
                invocation: state_invocation,
                result: state_result,
                run_ticks: 2,
                restart_generation: 1,
                fault: Some((FaultKind::ExecutionTrap, 6)),
            },
        )
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_PCI_SERIAL_DRIVER_FAULT_CONTAINED_OK");
    serial_write_line("AGENT_KERNEL_PCI_SERIAL_DRIVER_RESTARTED_OK");
    serial_write_line("AGENT_KERNEL_PCI_SERIAL_INTERRUPT_CONFIGURED_OK");

    let interrupt = pci_serial_interrupt::wait_for_thre(smp)?;
    serial_write_line("AGENT_KERNEL_PCI_SERIAL_INTX_OK");
    let interrupt_payload = DeviceEventPayload {
        code: u16::from(interrupt.iir),
        value: u64::from(interrupt.line_status),
    };
    let interrupt_event = booted
        .kernel_mut()
        .sys_raise_device_event(
            report.bootstrap_agent,
            region.capability(),
            region.resource(),
            DeviceEventKind::Interrupt,
            interrupt_payload,
        )
        .ok()?;
    let interrupt_invocation = booted
        .kernel_mut()
        .sys_deliver_device_event(DRIVER, driver_capability, interrupt_event)
        .ok()?;
    let interrupt_context = AgentCallContext::new_driver(
        DRIVER,
        interrupt_invocation,
        admission.image,
        driver_capability,
    )?;
    let interrupt_execution = native_invocation::execute_and_reclaim(
        booted,
        runtime,
        cpu_runtime,
        memory_pool,
        address_space_pool,
        smp,
        verified_image,
        interrupt_context,
        image_contract,
        recovery_authority,
        &mut backend,
    )?;
    let interrupt_result = DriverCommandResult {
        code: PCI_SERIAL_RESULT_OK,
        value: u64::from(pci_serial_profile::TRANSMIT_BYTE),
    };
    if interrupt_execution.result != interrupt_result
        || interrupt_execution.dispatches != 2
        || interrupt_execution.quantum_expiries != 1
        || interrupt_execution.restart_generation != 0
        || interrupt_execution.fault.is_some()
        || !evidence::terminal_matches(
            booted,
            evidence::TerminalEvidence {
                driver: DRIVER,
                resource: region.resource(),
                binding,
                event: interrupt_event,
                event_kind: DeviceEventKind::Interrupt,
                event_payload: interrupt_payload,
                command: interrupt_execution.command,
                command_kind: DriverCommandKind::Write,
                command_payload: DriverCommandPayload {
                    opcode: 0,
                    value: u64::from(pci_serial_profile::TRANSMIT_BYTE),
                },
                invocation: interrupt_invocation,
                result: interrupt_result,
                run_ticks: 1,
                restart_generation: 0,
                fault: None,
            },
        )
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_PCI_SERIAL_RING3_DRIVER_OK");
    serial_write_line("AGENT_KERNEL_PCI_SERIAL_PHYSICAL_IO_OK");
    serial_write_line("AGENT_KERNEL_PCI_SERIAL_ADDRESS_SPACE_RECLAIMED_OK");
    serial_write_line("AGENT_KERNEL_PCI_SERIAL_DRIVER_OK");
    Some(())
}
