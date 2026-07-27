//! Capability-bound physical command flow for the claimed PCI serial BAR.
//!
//! This bare-metal x86 adapter composes Core admission and immutable Driver
//! records with the architecture backend. Native I/O starts only after the
//! exact claim, delegated Capability, endpoint, and command all agree.

mod admission;
mod evidence;

use agent_kernel_core::{
    AgentId, AgentImageId, DeviceEventKind, DeviceEventPayload, DriverCommandResult,
};
use agent_kernel_x86_64::{
    agent_call::AgentCallContext,
    agent_image::{AgentImageFormat, VerifiedAgentImage},
    pci::{PciBarKind, PciFunctionClaim},
    pci_serial::{PciSerialBackend, PCI_SERIAL_RESULT_OK},
    NativePortIo,
};

use crate::{
    agent_cpu::AgentCpuRuntime,
    agent_memory::{NativeAddressSpaceFramePool, RuntimeMemoryPool},
    boot_agent_images,
    native_address_space_service::NativeAddressSpaceService,
    native_agent_runtime::NativeAgentRuntime,
    native_driver_executor, pci_serial_profile, serial_write_line,
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
    let context =
        AgentCallContext::new_driver(DRIVER, invocation, admission.image, driver_capability)?;
    let initial_pool_len = address_space_pool.len();
    let native_admission = NativeAddressSpaceService::admit(
        address_space_pool,
        runtime,
        cpu_runtime,
        memory_pool,
        verified_image,
        context,
    )?
    .ok()?;
    if native_admission.agent() != DRIVER
        || runtime.len() != 1
        || !runtime.contains(DRIVER)
        || address_space_pool.len() + native_admission.identity().owned_frame_count()
            != initial_pool_len
    {
        return None;
    }

    let execution = native_driver_executor::run(booted, runtime, DRIVER, invocation, &mut backend)?;
    let command = execution.command();
    let result = execution.result();
    let completed = execution.completed();
    if execution.dispatches() != 2
        || execution.quantum_expiries() != 1
        || completed.context() != context
        || completed.nonce() != image_contract.nonce()
        || completed.call_count() != 5
        || completed.operations() != image_contract.expected_operations()
        || completed.return_offsets() != image_contract.expected_return_offsets()
        || completed.physical_quantum_generation() != 1
        || completed.restart_generation() != 0
        || !completed.reclamation_log().is_empty()
        || result
            != (DriverCommandResult {
                code: PCI_SERIAL_RESULT_OK,
                value: u64::from(pci_serial_profile::TRANSMIT_BYTE),
            })
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_PCI_SERIAL_RING3_DRIVER_OK");
    serial_write_line("AGENT_KERNEL_PCI_SERIAL_PHYSICAL_IO_OK");

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

    let identity = native_admission.identity();
    let completed = execution.into_completed();
    let reclamation = completed.prepare_address_space_reclamation(address_space_pool)?;
    if reclamation.identity() != identity {
        return None;
    }
    let quarantined = completed.quarantine_address_space(address_space_pool, reclamation)?;
    let shootdown = smp
        .shootdown_address_space(quarantined.tlb_address_space())
        .ok()?;
    let reclaimed = quarantined.reclaim_after_shootdown(address_space_pool, shootdown)?;
    if !reclaimed.matches(DRIVER, identity)
        || !runtime.is_empty()
        || address_space_pool.len() != initial_pool_len
        || !address_space_pool.all_reclaimed_and_zero()
    {
        return None;
    }
    serial_write_line("AGENT_KERNEL_PCI_SERIAL_ADDRESS_SPACE_RECLAIMED_OK");
    serial_write_line("AGENT_KERNEL_PCI_SERIAL_DRIVER_OK");
    Some(())
}
