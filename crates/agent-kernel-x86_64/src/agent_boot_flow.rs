//! End-to-end dual-Agent execution and Driver handoff proof.
//!
//! This architecture boot adapter prepares private memory and CPU contexts,
//! binds physical evidence to semantic task transitions, then completes the
//! existing UART Driver flow. All failures terminate through explicit markers.

mod mailbox;
mod verifier;

use agent_kernel_boot::BootConfig;
use agent_kernel_x86_64::agent_image::{AgentImageCapsule, VerifiedAgentImage};
use bootloader_api::BootInfo;

use crate::{
    agent_cpu::AgentCpuRuntime, agent_memory::PreparedAgentMemory, boot_agent_images, event_trace,
    exit_qemu, fatal_boot, halt_forever, native_agent_runtime::NativeAgentRuntime,
    port_driver_flow::PortDriverSetup, privilege_runtime::PrivilegeBoundary, serial_transmit_empty,
    serial_write_line, serial_write_str, timer_task_flow::TimerTaskFlow, uart_interrupt,
    verifier_task_flow::VerifierTaskFlow, X86BootedKernel, COM1,
};

pub(super) fn run(boot_info: &'static mut BootInfo, privilege_boundary: PrivilegeBoundary) -> ! {
    let worker_a = boot_agent_images::worker_a();
    let worker_b = boot_agent_images::worker_b();
    let verifier_image = boot_agent_images::verifier();
    let Ok(mut booted) = X86BootedKernel::boot(BootConfig::default()) else {
        fatal_boot("AGENT_KERNEL_BOOT_ERROR");
    };
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
    if AgentImageCapsule::parse(worker_a.bytes()).is_err()
        || AgentImageCapsule::parse(worker_b.bytes()).is_err()
        || AgentImageCapsule::parse(verifier_image.bytes()).is_err()
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
    serial_write_line("AGENT_KERNEL_AGENT_IMAGE_DIGEST_OK");
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
    validate_agent_memory(&agent_a_memory, &agent_b_memory, &verifier_memory);
    serial_write_line("AGENT_KERNEL_AGENT_IMAGE_LOAD_OK");
    let Some(cpu_runtime) = AgentCpuRuntime::install(&privilege_boundary, agent_a_memory.roots())
    else {
        fatal_boot("AGENT_KERNEL_AGENT_CPU_SETUP_ERROR");
    };
    let Some(agent_a_cpu) = cpu_runtime.prepare(agent_a_memory, agent_a_context) else {
        fatal_boot("AGENT_KERNEL_AGENT_CPU_SETUP_ERROR");
    };
    let Some(agent_b_cpu) = cpu_runtime.prepare(agent_b_memory, agent_b_context) else {
        fatal_boot("AGENT_KERNEL_AGENT_CPU_SETUP_ERROR");
    };
    let Some(verifier_cpu) = cpu_runtime.prepare(verifier_memory, verifier_context) else {
        fatal_boot("AGENT_KERNEL_VERIFIER_CPU_SETUP_ERROR");
    };
    let mut native_runtime = NativeAgentRuntime::new();
    for cpu in [agent_a_cpu, agent_b_cpu, verifier_cpu] {
        if native_runtime.register(cpu).is_some() {
            fatal_boot("AGENT_KERNEL_NATIVE_RUNTIME_STORE_ERROR");
        }
    }
    if native_runtime.len() != 3 {
        fatal_boot("AGENT_KERNEL_NATIVE_RUNTIME_STORE_ERROR");
    }
    let Some((timer_flow, dispatched_b)) = queued_timer_flow.dispatch_second(&mut booted) else {
        fatal_boot("AGENT_KERNEL_TIMER_TASK_SETUP_ERROR");
    };
    let Some(agent_b_cpu) = native_runtime.take_dispatched(dispatched_b) else {
        fatal_boot("AGENT_KERNEL_NATIVE_RUNTIME_STORE_ERROR");
    };
    if native_runtime.len() != 2 {
        fatal_boot("AGENT_KERNEL_NATIVE_RUNTIME_STORE_ERROR");
    }
    let Some(preempted_b) = agent_b_cpu.run_until_preempted() else {
        fatal_boot("AGENT_KERNEL_AGENT_CPU_PREEMPTION_ERROR");
    };
    serial_write_line("AGENT_KERNEL_PIT_IRQ_OK");
    serial_write_line("AGENT_KERNEL_AGENT_CPU_PREEMPTION_OK");
    serial_write_line("AGENT_KERNEL_AGENT_RING3_PREEMPTION_OK");
    let Some((first_running_flow, dispatched_a)) =
        timer_flow.expire_second_and_dispatch_first(&mut booted, &preempted_b)
    else {
        fatal_boot("AGENT_KERNEL_TIMER_PREEMPTION_ERROR");
    };
    let Some(agent_a_cpu) = native_runtime.take_dispatched(dispatched_a) else {
        fatal_boot("AGENT_KERNEL_NATIVE_RUNTIME_STORE_ERROR");
    };
    if native_runtime.len() != 1 {
        fatal_boot("AGENT_KERNEL_NATIVE_RUNTIME_STORE_ERROR");
    }
    let Some(preempted_a) = agent_a_cpu.run_until_preempted() else {
        fatal_boot("AGENT_KERNEL_AGENT_CPU_PREEMPTION_ERROR");
    };
    serial_write_line("AGENT_KERNEL_AGENT_A_PREEMPTION_OK");
    let Some(second_resumed_flow) =
        first_running_flow.expire_first_and_dispatch_second(&mut booted, &preempted_a)
    else {
        fatal_boot("AGENT_KERNEL_TIMER_PREEMPTION_ERROR");
    };
    serial_write_line("AGENT_KERNEL_TIMER_PREEMPTION_OK");
    serial_write_line("AGENT_KERNEL_KERNEL_SELECTED_DISPATCH_OK");
    let completed_workers = mailbox::run(
        &mut booted,
        second_resumed_flow,
        preempted_a,
        preempted_b,
        worker_a,
        worker_b,
    );
    let Some(_completed_verifier) = verifier::run(
        &mut booted,
        verifier_flow,
        &mut native_runtime,
        verifier_image,
        completed_workers,
    ) else {
        fatal_boot("AGENT_KERNEL_NATIVE_VERIFIER_ERROR");
    };
    serial_write_line("AGENT_KERNEL_AGENT_CALL_ABI_OK");
    serial_write_line("AGENT_KERNEL_AGENT_CALL_RETURN_OK");
    serial_write_line("AGENT_KERNEL_AGENT_CALL_AUTHORITY_OK");
    serial_write_line("AGENT_KERNEL_AGENT_CALL_COMPLETE_OK");
    serial_write_line("AGENT_KERNEL_AGENT_CR3_SWITCH_OK");
    serial_write_line("AGENT_KERNEL_MULTI_AGENT_CONTEXT_SWITCH_OK");
    serial_write_line("AGENT_KERNEL_HETEROGENEOUS_AGENT_EXECUTION_OK");
    complete_driver_flow(&mut booted, driver_setup);
    event_trace::write(booted.kernel().events());
    serial_write_line("SUPERVISOR_HANDOFF_READY");
    exit_qemu(0x10);
    halt_forever()
}

fn validate_agent_memory(
    first: &PreparedAgentMemory,
    second: &PreparedAgentMemory,
    verifier: &PreparedAgentMemory,
) {
    serial_write_line("AGENT_KERNEL_AGENT_USER_MEMORY_OK");
    if !first.kernel_address_space_active()
        || !second.kernel_address_space_active()
        || !verifier.kernel_address_space_active()
        || first.roots().kernel_cr3() != second.roots().kernel_cr3()
        || first.roots().kernel_cr3() != verifier.roots().kernel_cr3()
    {
        fatal_boot("AGENT_KERNEL_AGENT_ADDRESS_SPACE_ERROR");
    }
    serial_write_line("AGENT_KERNEL_AGENT_ADDRESS_SPACE_OK");
    if !first.is_disjoint_from(second)
        || !first.is_disjoint_from(verifier)
        || !second.is_disjoint_from(verifier)
        || !first.signal_is_clear()
        || !second.signal_is_clear()
        || !verifier.signal_is_clear()
    {
        fatal_boot("AGENT_KERNEL_MULTI_AGENT_MEMORY_ERROR");
    }
    serial_write_line("AGENT_KERNEL_VERIFIER_MEMORY_OK");
    serial_write_line("AGENT_KERNEL_MULTI_AGENT_MEMORY_OK");
}

fn complete_driver_flow(booted: &mut X86BootedKernel, driver_setup: PortDriverSetup) {
    let Some(uart_signal) = uart_interrupt::wait_for_uart_thre() else {
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
