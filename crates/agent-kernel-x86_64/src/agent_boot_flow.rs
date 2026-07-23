//! End-to-end multi-Agent execution, fault containment, and Driver proof.
//!
//! This architecture boot adapter prepares private memory and CPU contexts,
//! binds physical evidence to semantic task transitions, then completes the
//! existing UART Driver flow. All failures terminate through explicit markers.

mod address_space_reuse;
mod runtime_loop;

use agent_kernel_boot::BootConfig;
use agent_kernel_core::{
    ActionId, AgentId, AgentImageSignerStatus, EventKind, ResourceKind, TaskId, TaskStatus,
};
use agent_kernel_x86_64::agent_image::{
    AgentImageCapsule, AgentImageFormat, AgentImageLoadError, AgentImageTrust,
    AgentImageTrustPolicy, VerifiedAgentImage,
};
use bootloader_api::BootInfo;

use crate::{
    agent_cpu::AgentCpuRuntime,
    agent_memory::{NativeAddressSpaceFramePool, PreparedAgentMemory, RuntimeMemoryPool},
    boot_agent_images,
    boot_agent_trust::{
        RESOURCE_MANAGER_PUBLIC_KEY, RESOURCE_MANAGER_SCOPE, RESOURCE_MANAGER_SIGNER_ID,
    },
    event_trace, exception_runtime, exit_qemu, fatal_boot,
    fault_handler_flow::FaultHandlerFlow,
    fault_task_flow::FaultTaskFlow,
    halt_forever,
    native_agent_runtime::NativeAgentRuntime,
    port_driver_flow::PortDriverSetup,
    privilege_runtime::PrivilegeBoundary,
    resource_manager_flow::ResourceManagerFlow,
    serial_transmit_empty, serial_write_line, serial_write_str,
    smp_boot::SmpBootstrap,
    timer_task_flow::TimerTaskFlow,
    uart_interrupt,
    verifier_task_flow::VerifierTaskFlow,
    X86BootedKernel, COM1,
};

const INITIAL_ADDRESS_SPACE_FRAME_INVENTORY: usize = 77;

pub(super) fn run(
    boot_info: &'static mut BootInfo,
    privilege_boundary: PrivilegeBoundary,
    mut smp_bootstrap: SmpBootstrap,
) -> ! {
    if smp_bootstrap.prepare_apic_mmio(boot_info).is_err() {
        fatal_boot("AGENT_KERNEL_APIC_MMIO_ERROR");
    }
    serial_write_line("AGENT_KERNEL_APIC_MMIO_OK");
    serial_write_line("AGENT_KERNEL_IO_APIC_IRQ_ROUTING_OK");
    serial_write_line("AGENT_KERNEL_LEGACY_PIC_DISABLED_OK");
    if smp_bootstrap.prepare_trampoline(boot_info).is_err() {
        fatal_boot("AGENT_KERNEL_AP_TRAMPOLINE_ERROR");
    }
    serial_write_line("AGENT_KERNEL_AP_TRAMPOLINE_OK");
    if !smp_bootstrap.ready_for_agent_boot() {
        fatal_boot("AGENT_KERNEL_SMP_BOOTSTRAP_ERROR");
    }
    let worker_a = boot_agent_images::worker_a();
    let worker_b = boot_agent_images::worker_b();
    let verifier_image = boot_agent_images::verifier();
    let fault_image = boot_agent_images::fault_worker();
    let fault_handler_image = boot_agent_images::fault_handler();
    let resource_manager_image = boot_agent_images::resource_manager();
    let reuse_worker_image = boot_agent_images::reuse_worker();
    let admission_supervisor_image = boot_agent_images::admission_supervisor();
    let boot_config = BootConfig::new(AgentId::new(1), ResourceKind::Workspace, ActionId::new(1));
    let Ok(mut booted) = X86BootedKernel::boot(boot_config) else {
        fatal_boot("AGENT_KERNEL_BOOT_ERROR");
    };
    let report = *booted.report();
    let Ok(resource_manager_signer) = booted.kernel_mut().sys_trust_agent_image_signer(
        report.bootstrap_agent,
        report.bootstrap_capability,
        report.bootstrap_resource,
        RESOURCE_MANAGER_PUBLIC_KEY,
        RESOURCE_MANAGER_SCOPE,
        1,
        1,
    ) else {
        fatal_boot("AGENT_KERNEL_BOOT_TRUST_IMPORT_ERROR");
    };
    if resource_manager_signer.signer_id != RESOURCE_MANAGER_SIGNER_ID
        || booted.kernel().agent_image_signer_policy_generation() != 1
    {
        fatal_boot("AGENT_KERNEL_BOOT_TRUST_IMPORT_ERROR");
    }
    serial_write_line("AGENT_KERNEL_NATIVE_TRUST_STORE_OK");
    let Some(driver_setup) = PortDriverSetup::prepare(&mut booted, COM1) else {
        fatal_boot("AGENT_KERNEL_PORT_DRIVER_SETUP_ERROR");
    };
    let Some(queued_timer_flow) = TimerTaskFlow::prepare(
        &mut booted,
        worker_a.digest(),
        worker_b.digest(),
        worker_a.result(),
        worker_b.result(),
    ) else {
        fatal_boot("AGENT_KERNEL_TIMER_TASK_SETUP_ERROR");
    };
    let subject = queued_timer_flow.verification_subject();
    if subject.task().raw() != verifier_image.target()
        || subject.result() != verifier_image.result()
    {
        fatal_boot("AGENT_KERNEL_VERIFIER_SUBJECT_ERROR");
    }
    let Some(verifier_flow) =
        VerifierTaskFlow::prepare(&mut booted, subject, verifier_image.digest())
    else {
        fatal_boot("AGENT_KERNEL_VERIFIER_TASK_SETUP_ERROR");
    };
    let Some(fault_flow) = FaultTaskFlow::prepare(&mut booted, fault_image.digest()) else {
        fatal_boot("AGENT_KERNEL_FAULT_TASK_SETUP_ERROR");
    };
    let Some(fault_handler_flow) =
        FaultHandlerFlow::prepare(&mut booted, fault_handler_image.digest())
    else {
        fatal_boot("AGENT_KERNEL_FAULT_HANDLER_SETUP_ERROR");
    };
    let Some(resource_manager_flow) =
        ResourceManagerFlow::prepare(&mut booted, resource_manager_image)
    else {
        fatal_boot("AGENT_KERNEL_RESOURCE_MANAGER_SETUP_ERROR");
    };
    let Some((agent_a_record, agent_b_record)) = queued_timer_flow.image_records(&booted) else {
        fatal_boot("AGENT_KERNEL_AGENT_IMAGE_RECORD_ERROR");
    };
    let Some((agent_a_context, agent_b_context)) = queued_timer_flow.call_contexts() else {
        fatal_boot("AGENT_KERNEL_AGENT_CALL_CONTEXT_ERROR");
    };
    let Some(verifier_record) = verifier_flow.image_record(&booted) else {
        fatal_boot("AGENT_KERNEL_VERIFIER_IMAGE_RECORD_ERROR");
    };
    let Some(verifier_context) = verifier_flow.call_context() else {
        fatal_boot("AGENT_KERNEL_VERIFIER_CALL_CONTEXT_ERROR");
    };
    let Some(fault_record) = fault_flow.image_record(&booted) else {
        fatal_boot("AGENT_KERNEL_FAULT_IMAGE_RECORD_ERROR");
    };
    let Some(fault_context) = fault_flow.call_context() else {
        fatal_boot("AGENT_KERNEL_FAULT_CALL_CONTEXT_ERROR");
    };
    let Some(fault_handler_record) = fault_handler_flow.image_record(&booted) else {
        fatal_boot("AGENT_KERNEL_FAULT_HANDLER_IMAGE_RECORD_ERROR");
    };
    let Some(fault_handler_context) = fault_handler_flow.call_context() else {
        fatal_boot("AGENT_KERNEL_FAULT_HANDLER_CALL_CONTEXT_ERROR");
    };
    let Some(resource_manager_record) = resource_manager_flow.image_record(&booted) else {
        fatal_boot("AGENT_KERNEL_RESOURCE_MANAGER_IMAGE_RECORD_ERROR");
    };
    let Some(resource_manager_context) = resource_manager_flow.call_context() else {
        fatal_boot("AGENT_KERNEL_RESOURCE_MANAGER_CALL_CONTEXT_ERROR");
    };
    if AgentImageCapsule::parse(worker_a.bytes()).is_err()
        || AgentImageCapsule::parse(worker_b.bytes()).is_err()
        || AgentImageCapsule::parse(verifier_image.bytes()).is_err()
        || AgentImageCapsule::parse(fault_image.bytes()).is_err()
        || AgentImageCapsule::parse(fault_handler_image.bytes()).is_err()
        || AgentImageCapsule::parse(resource_manager_image.bytes()).is_err()
        || AgentImageCapsule::parse(reuse_worker_image.bytes()).is_err()
        || AgentImageCapsule::parse(admission_supervisor_image.bytes()).is_err()
    {
        fatal_boot("AGENT_KERNEL_AGENT_IMAGE_FORMAT_ERROR");
    }
    serial_write_line("AGENT_KERNEL_AGENT_IMAGE_FORMAT_OK");
    let Ok(agent_a_image) = VerifiedAgentImage::verify(agent_a_record, worker_a.bytes()) else {
        fatal_boot("AGENT_KERNEL_AGENT_IMAGE_DIGEST_ERROR");
    };
    let Ok(agent_b_image) = VerifiedAgentImage::verify(agent_b_record, worker_b.bytes()) else {
        fatal_boot("AGENT_KERNEL_AGENT_IMAGE_DIGEST_ERROR");
    };
    let Ok(verifier_verified_image) =
        VerifiedAgentImage::verify(verifier_record, verifier_image.bytes())
    else {
        fatal_boot("AGENT_KERNEL_VERIFIER_IMAGE_DIGEST_ERROR");
    };
    let Ok(fault_verified_image) = VerifiedAgentImage::verify(fault_record, fault_image.bytes())
    else {
        fatal_boot("AGENT_KERNEL_FAULT_IMAGE_DIGEST_ERROR");
    };
    let Ok(fault_handler_verified_image) =
        VerifiedAgentImage::verify(fault_handler_record, fault_handler_image.bytes())
    else {
        fatal_boot("AGENT_KERNEL_FAULT_HANDLER_IMAGE_DIGEST_ERROR");
    };
    let trust_policy = AgentImageTrustPolicy::new(booted.kernel().agent_image_signers());
    let Ok(resource_manager_verified_image) = VerifiedAgentImage::verify_signed(
        resource_manager_record,
        resource_manager_image.bytes(),
        &trust_policy,
    ) else {
        fatal_boot("AGENT_KERNEL_RESOURCE_MANAGER_SIGNATURE_ERROR");
    };
    if resource_manager_verified_image.format() != AgentImageFormat::SignedPackageV3
        || resource_manager_verified_image.signer_id() != Some(RESOURCE_MANAGER_SIGNER_ID)
        || resource_manager_verified_image.trust()
            != AgentImageTrust::Signed(RESOURCE_MANAGER_SIGNER_ID)
    {
        fatal_boot("AGENT_KERNEL_RESOURCE_MANAGER_TRUST_ERROR");
    }
    serial_write_line("AGENT_KERNEL_AGENT_IMAGE_DIGEST_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_SIGNED_PACKAGE_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_TRUSTED_SIGNER_OK");
    serial_write_line("AGENT_KERNEL_VERIFIER_IMAGE_OK");
    let Some(agent_a_memory) = PreparedAgentMemory::prepare(boot_info, agent_a_image) else {
        fatal_boot("AGENT_KERNEL_AGENT_USER_MEMORY_ERROR");
    };
    let Some(agent_b_memory) = PreparedAgentMemory::prepare(boot_info, agent_b_image) else {
        fatal_boot("AGENT_KERNEL_AGENT_USER_MEMORY_ERROR");
    };
    let Some(verifier_memory) = PreparedAgentMemory::prepare(boot_info, verifier_verified_image)
    else {
        fatal_boot("AGENT_KERNEL_VERIFIER_USER_MEMORY_ERROR");
    };
    let Some(fault_memory) = PreparedAgentMemory::prepare(boot_info, fault_verified_image) else {
        fatal_boot("AGENT_KERNEL_FAULT_USER_MEMORY_ERROR");
    };
    let Some(fault_handler_memory) =
        PreparedAgentMemory::prepare(boot_info, fault_handler_verified_image)
    else {
        fatal_boot("AGENT_KERNEL_FAULT_HANDLER_USER_MEMORY_ERROR");
    };
    let Some(resource_manager_memory) =
        PreparedAgentMemory::prepare(boot_info, resource_manager_verified_image)
    else {
        fatal_boot("AGENT_KERNEL_RESOURCE_MANAGER_USER_MEMORY_ERROR");
    };
    validate_agent_memory([
        &agent_a_memory,
        &agent_b_memory,
        &verifier_memory,
        &fault_memory,
        &fault_handler_memory,
        &resource_manager_memory,
    ]);
    let Some(mut runtime_memory_pool) = RuntimeMemoryPool::prepare(boot_info) else {
        fatal_boot("AGENT_KERNEL_RUNTIME_FRAME_POOL_ERROR");
    };
    if !runtime_memory_pool.all_available_and_zero()
        || [
            &agent_a_memory,
            &agent_b_memory,
            &verifier_memory,
            &fault_memory,
            &fault_handler_memory,
            &resource_manager_memory,
        ]
        .iter()
        .any(|memory| !runtime_memory_pool.is_disjoint_from(memory))
    {
        fatal_boot("AGENT_KERNEL_RUNTIME_FRAME_POOL_ERROR");
    }
    serial_write_line("AGENT_KERNEL_RUNTIME_FRAME_POOL_OK");
    serial_write_line("AGENT_KERNEL_AGENT_IMAGE_LOAD_OK");
    let Some((local_apic_base, physical_offset, initial_count)) = smp_bootstrap.bsp_quantum_timer()
    else {
        fatal_boot("AGENT_KERNEL_AGENT_CPU_SETUP_ERROR");
    };
    let Some(cpu_runtime) = AgentCpuRuntime::install(
        &privilege_boundary,
        agent_a_memory.roots(),
        smp_bootstrap.bsp_index(),
        local_apic_base,
        physical_offset,
        initial_count,
    ) else {
        fatal_boot("AGENT_KERNEL_AGENT_CPU_SETUP_ERROR");
    };
    if uart_interrupt::install_gate().is_none()
        || smp_bootstrap.install_ipi_gate().is_err()
        || exception_runtime::freeze_for_smp().is_none()
    {
        fatal_boot("AGENT_KERNEL_IDT_FREEZE_ERROR");
    }
    let Ok(online_cpu_count) = smp_bootstrap.start_application_processors() else {
        fatal_boot("AGENT_KERNEL_SMP_AP_STARTUP_ERROR");
    };
    if online_cpu_count < 2 {
        fatal_boot("AGENT_KERNEL_SMP_AP_STARTUP_ERROR");
    }
    serial_write_line("AGENT_KERNEL_SMP_AP_ONLINE_OK");
    serial_write_line("AGENT_KERNEL_PER_CPU_PRIVILEGE_OK");
    let Some(agent_a_cpu) = cpu_runtime.prepare(agent_a_memory, agent_a_context) else {
        fatal_boot("AGENT_KERNEL_AGENT_CPU_SETUP_ERROR");
    };
    let Some(agent_b_cpu) = cpu_runtime.prepare(agent_b_memory, agent_b_context) else {
        fatal_boot("AGENT_KERNEL_AGENT_CPU_SETUP_ERROR");
    };
    let Some(verifier_cpu) = cpu_runtime.prepare(verifier_memory, verifier_context) else {
        fatal_boot("AGENT_KERNEL_VERIFIER_CPU_SETUP_ERROR");
    };
    let Some(fault_cpu) = cpu_runtime.prepare(fault_memory, fault_context) else {
        fatal_boot("AGENT_KERNEL_FAULT_CPU_SETUP_ERROR");
    };
    let Some(fault_handler_cpu) = cpu_runtime.prepare(fault_handler_memory, fault_handler_context)
    else {
        fatal_boot("AGENT_KERNEL_FAULT_HANDLER_CPU_SETUP_ERROR");
    };
    let Some(resource_manager_cpu) =
        cpu_runtime.prepare(resource_manager_memory, resource_manager_context)
    else {
        fatal_boot("AGENT_KERNEL_RESOURCE_MANAGER_CPU_SETUP_ERROR");
    };
    let mut native_runtime = NativeAgentRuntime::new();
    let mut address_space_frame_pool = NativeAddressSpaceFramePool::new();
    for cpu in [
        agent_a_cpu,
        agent_b_cpu,
        verifier_cpu,
        fault_cpu,
        fault_handler_cpu,
        resource_manager_cpu,
    ] {
        if native_runtime.register_prepared(cpu).is_some() {
            fatal_boot("AGENT_KERNEL_NATIVE_RUNTIME_STORE_ERROR");
        }
    }
    if native_runtime.len() != 6 {
        fatal_boot("AGENT_KERNEL_NATIVE_RUNTIME_STORE_ERROR");
    }
    let runtime_plan = runtime_loop::RuntimeLoopPlan::new(
        queued_timer_flow,
        verifier_flow,
        [worker_a, worker_b],
        [agent_a_context, agent_b_context],
        verifier_image,
        verifier_context,
        fault_flow,
        fault_image,
        fault_context,
        fault_handler_flow,
        fault_handler_image,
        resource_manager_flow,
        resource_manager_image,
    );
    if runtime_loop::run(
        &mut booted,
        &mut native_runtime,
        &mut runtime_memory_pool,
        &mut address_space_frame_pool,
        &mut smp_bootstrap,
        runtime_plan,
    )
    .is_none()
        || !address_space_frame_pool.seal_inventory()
        || !address_space_frame_pool.all_reclaimed_and_zero()
        || address_space_frame_pool.inventory_frame_count()
            != Some(INITIAL_ADDRESS_SPACE_FRAME_INVENTORY)
    {
        fatal_boot("AGENT_KERNEL_NATIVE_RUNTIME_LOOP_ERROR");
    }
    serial_write_line("AGENT_KERNEL_NATIVE_RIGHT_SIZED_CODE_FRAMES_OK");
    let signers = booted.kernel().agent_image_signers();
    let resource_manager_signer = signers
        .iter()
        .find(|signer| signer.signer_id == RESOURCE_MANAGER_SIGNER_ID);
    let reuse_worker_signer = signers
        .iter()
        .find(|signer| signer.signer_id == reuse_worker_image.signer_id());
    if booted.kernel().agent_image_signer_policy_generation() != 2
        || signers.len() != 2
        || !matches!(
            resource_manager_signer,
            Some(signer)
                if signer.status == AgentImageSignerStatus::Revoked
                    && signer.generation == 2
                    && signer.resource == report.bootstrap_resource
        )
        || !matches!(
            reuse_worker_signer,
            Some(signer)
                if signer.status == AgentImageSignerStatus::Active
                    && signer.generation == 2
                    && signer.resource == report.bootstrap_resource
                    && signer.public_key == reuse_worker_image.public_key()
        )
    {
        fatal_boot("AGENT_KERNEL_NATIVE_SIGNER_ROTATION_ERROR");
    }
    serial_write_line("AGENT_KERNEL_NATIVE_SIGNER_ROTATION_OK");
    let trust_policy = AgentImageTrustPolicy::new(signers);
    if !matches!(
        VerifiedAgentImage::verify_signed(
            resource_manager_record,
            resource_manager_image.bytes(),
            &trust_policy,
        ),
        Err(AgentImageLoadError::SignerRevoked)
    ) {
        fatal_boot("AGENT_KERNEL_NATIVE_REVOKED_SIGNER_REJECTED_ERROR");
    }
    serial_write_line("AGENT_KERNEL_NATIVE_REVOKED_SIGNER_REJECTED_OK");
    if verify_initial_task_prefix(&mut booted).is_none() {
        fatal_boot("AGENT_KERNEL_TASK_PREFIX_VERIFICATION_ERROR");
    }
    serial_write_line("AGENT_KERNEL_TASK_PREFIX_VERIFIED_OK");
    let Some(event_archive) = address_space_reuse::run(
        &mut booted,
        &mut native_runtime,
        &mut runtime_memory_pool,
        &mut address_space_frame_pool,
        &mut smp_bootstrap,
        &cpu_runtime,
        reuse_worker_image,
        admission_supervisor_image,
    ) else {
        fatal_boot("AGENT_KERNEL_NATIVE_ADDRESS_SPACE_REUSE_ERROR");
    };
    serial_write_line("AGENT_KERNEL_AGENT_CALL_ABI_OK");
    serial_write_line("AGENT_KERNEL_AGENT_CALL_RETURN_OK");
    serial_write_line("AGENT_KERNEL_AGENT_CALL_AUTHORITY_OK");
    serial_write_line("AGENT_KERNEL_AGENT_CALL_COMPLETE_OK");
    serial_write_line("AGENT_KERNEL_AGENT_CR3_SWITCH_OK");
    serial_write_line("AGENT_KERNEL_MULTI_AGENT_CONTEXT_SWITCH_OK");
    serial_write_line("AGENT_KERNEL_HETEROGENEOUS_AGENT_EXECUTION_OK");
    complete_driver_flow(&mut booted, &mut smp_bootstrap, driver_setup);
    if !event_archive.proves_terminal_replay(&booted) {
        fatal_boot("AGENT_KERNEL_NATIVE_EVENT_ARCHIVE_REPLAY_ERROR");
    }
    serial_write_line("AGENT_KERNEL_NATIVE_EVENT_ARCHIVE_REPLAY_OK");
    event_trace::write(event_archive.events());
    event_trace::write(booted.kernel().events());
    serial_write_line("AGENT_KERNEL_SMP_HANDOFF_READY");
    serial_write_line("SUPERVISOR_HANDOFF_READY");
    exit_qemu(0x10);
    halt_forever()
}

fn verify_initial_task_prefix(booted: &mut X86BootedKernel) -> Option<()> {
    let report = *booted.report();
    let event_start = booted.kernel().events().len();
    for raw in 1..=6 {
        let task = TaskId::new(raw);
        match booted.kernel().task(task).ok()?.status {
            TaskStatus::Verified => {}
            TaskStatus::Completed => {
                booted
                    .kernel_mut()
                    .sys_verify_task(report.bootstrap_agent, report.bootstrap_capability, task)
                    .ok()?;
            }
            _ => return None,
        }
    }

    let kernel = booted.kernel();
    let events = kernel.events().get(event_start..)?;
    (events.len() == 10
        && (1..=6).all(|raw| {
            kernel
                .task(TaskId::new(raw))
                .is_ok_and(|task| task.status == TaskStatus::Verified)
        })
        && (0..events.len() / 2).all(|index| {
            let task = TaskId::new(index as u64 + 2);
            let verified = &events[index * 2];
            let fulfilled = &events[index * 2 + 1];
            verified.kind == EventKind::TaskVerified
                && verified.task == Some(task)
                && fulfilled.kind == EventKind::IntentFulfilled
                && fulfilled.task == Some(task)
        }))
    .then_some(())
}

fn validate_agent_memory(memories: [&PreparedAgentMemory; 6]) {
    serial_write_line("AGENT_KERNEL_AGENT_USER_MEMORY_OK");
    let kernel_cr3 = memories[0].roots().kernel_cr3();
    if memories.iter().any(|memory| {
        !memory.kernel_address_space_active() || memory.roots().kernel_cr3() != kernel_cr3
    }) {
        fatal_boot("AGENT_KERNEL_AGENT_ADDRESS_SPACE_ERROR");
    }
    serial_write_line("AGENT_KERNEL_AGENT_ADDRESS_SPACE_OK");
    for (index, first) in memories.iter().enumerate() {
        if memories[index + 1..]
            .iter()
            .any(|second| !first.is_disjoint_from(second))
        {
            fatal_boot("AGENT_KERNEL_MULTI_AGENT_MEMORY_ERROR");
        }
    }
    let expected_code_pages = [1, 1, 1, 1, 1, 5];
    let expected_rodata_pages = [0, 0, 0, 0, 0, 1];
    if memories
        .iter()
        .zip(expected_code_pages.into_iter().zip(expected_rodata_pages))
        .any(|(memory, (code_pages, rodata_pages))| {
            memory.identity().code_page_count() != code_pages
                || memory.identity().rodata_page_count() != rodata_pages
                || memory.identity().owned_frame_count() != code_pages + rodata_pages + 11
        })
    {
        fatal_boot("AGENT_KERNEL_AGENT_CODE_FRAME_PROFILE_ERROR");
    }
    if memories.iter().any(|memory| !memory.signal_is_clear()) {
        fatal_boot("AGENT_KERNEL_MULTI_AGENT_MEMORY_ERROR");
    }
    serial_write_line("AGENT_KERNEL_VERIFIER_MEMORY_OK");
    serial_write_line("AGENT_KERNEL_FAULT_HANDLER_MEMORY_OK");
    serial_write_line("AGENT_KERNEL_RESOURCE_MANAGER_MEMORY_OK");
    serial_write_line("AGENT_KERNEL_MULTI_AGENT_MEMORY_OK");
}

fn complete_driver_flow(
    booted: &mut X86BootedKernel,
    smp_bootstrap: &mut SmpBootstrap,
    driver_setup: PortDriverSetup,
) {
    let Some(uart_signal) = uart_interrupt::wait_for_uart_thre(smp_bootstrap) else {
        fatal_boot("AGENT_KERNEL_UART_IRQ_ERROR");
    };
    serial_write_line("AGENT_KERNEL_UART_IRQ_OK");
    let Some(mut port_flow) =
        driver_setup.dispatch_interrupt(booted, uart_signal.iir, uart_signal.line_status, b'O')
    else {
        fatal_boot("AGENT_KERNEL_PORT_DRIVER_INTERRUPT_ERROR");
    };
    serial_write_str("AGENT_KERNEL_PORT_IO_BACKEND_");
    while !serial_transmit_empty() {}
    if !port_flow.execute_and_record(booted) {
        fatal_boot("ERROR");
    }
    serial_write_line("K");
    serial_write_line("AGENT_KERNEL_PORT_COMMAND_FLOW_OK");
    serial_write_line("AGENT_KERNEL_DRIVER_INVOCATION_FLOW_OK");
}
