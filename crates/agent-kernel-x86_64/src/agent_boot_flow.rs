//! End-to-end dual-Agent execution and Driver handoff proof.
//!
//! This architecture boot adapter prepares private memory and CPU contexts,
//! binds physical evidence to semantic task transitions, then completes the
//! existing UART Driver flow. All failures terminate through explicit markers.

use agent_kernel_boot::BootConfig;
use bootloader_api::BootInfo;

use crate::{
    agent_cpu::AgentCpuRuntime, agent_memory::PreparedAgentMemory, event_trace, exit_qemu,
    fatal_boot, halt_forever, port_driver_flow::PortDriverSetup,
    privilege_runtime::PrivilegeBoundary, serial_transmit_empty, serial_write_line,
    serial_write_str, timer_task_flow::TimerTaskFlow, uart_interrupt, X86BootedKernel, COM1,
};

pub(super) fn run(boot_info: &'static mut BootInfo, privilege_boundary: PrivilegeBoundary) -> ! {
    let Some(agent_a_memory) = PreparedAgentMemory::prepare(boot_info) else {
        fatal_boot("AGENT_KERNEL_AGENT_USER_MEMORY_ERROR");
    };
    let Some(agent_b_memory) = PreparedAgentMemory::prepare(boot_info) else {
        fatal_boot("AGENT_KERNEL_AGENT_USER_MEMORY_ERROR");
    };
    serial_write_line("AGENT_KERNEL_AGENT_USER_MEMORY_OK");
    if !agent_a_memory.kernel_address_space_active()
        || !agent_b_memory.kernel_address_space_active()
        || agent_a_memory.roots().kernel_cr3() != agent_b_memory.roots().kernel_cr3()
    {
        fatal_boot("AGENT_KERNEL_AGENT_ADDRESS_SPACE_ERROR");
    }
    serial_write_line("AGENT_KERNEL_AGENT_ADDRESS_SPACE_OK");
    if !agent_a_memory.is_disjoint_from(&agent_b_memory)
        || !agent_a_memory.signal_is_clear()
        || !agent_b_memory.signal_is_clear()
    {
        fatal_boot("AGENT_KERNEL_MULTI_AGENT_MEMORY_ERROR");
    }
    serial_write_line("AGENT_KERNEL_MULTI_AGENT_MEMORY_OK");
    let Some(cpu_runtime) = AgentCpuRuntime::install(&privilege_boundary, agent_a_memory.roots())
    else {
        fatal_boot("AGENT_KERNEL_AGENT_CPU_SETUP_ERROR");
    };
    let Some(agent_a_cpu) = cpu_runtime.prepare(agent_a_memory) else {
        fatal_boot("AGENT_KERNEL_AGENT_CPU_SETUP_ERROR");
    };
    let Some(agent_b_cpu) = cpu_runtime.prepare(agent_b_memory) else {
        fatal_boot("AGENT_KERNEL_AGENT_CPU_SETUP_ERROR");
    };
    let Ok(mut booted) = X86BootedKernel::boot(BootConfig::default()) else {
        fatal_boot("AGENT_KERNEL_BOOT_ERROR");
    };
    let Some(driver_setup) = PortDriverSetup::prepare(&mut booted, COM1) else {
        fatal_boot("AGENT_KERNEL_PORT_DRIVER_SETUP_ERROR");
    };
    let Some(timer_flow) = TimerTaskFlow::prepare(&mut booted) else {
        fatal_boot("AGENT_KERNEL_TIMER_TASK_SETUP_ERROR");
    };
    let Some(preempted_a) = agent_a_cpu.run_until_preempted() else {
        fatal_boot("AGENT_KERNEL_AGENT_CPU_PREEMPTION_ERROR");
    };
    serial_write_line("AGENT_KERNEL_PIT_IRQ_OK");
    serial_write_line("AGENT_KERNEL_AGENT_CPU_PREEMPTION_OK");
    serial_write_line("AGENT_KERNEL_AGENT_RING3_PREEMPTION_OK");
    let Some(second_running_flow) =
        timer_flow.expire_first_and_dispatch_second(&mut booted, &preempted_a)
    else {
        fatal_boot("AGENT_KERNEL_TIMER_PREEMPTION_ERROR");
    };
    let Some(preempted_b) = agent_b_cpu.run_until_preempted() else {
        fatal_boot("AGENT_KERNEL_AGENT_CPU_PREEMPTION_ERROR");
    };
    serial_write_line("AGENT_KERNEL_AGENT_B_PREEMPTION_OK");
    let Some(first_resumed_flow) =
        second_running_flow.expire_second_and_dispatch_first(&mut booted, &preempted_b)
    else {
        fatal_boot("AGENT_KERNEL_TIMER_PREEMPTION_ERROR");
    };
    serial_write_line("AGENT_KERNEL_TIMER_PREEMPTION_OK");
    if !preempted_a.signal_is_clear() || !preempted_b.signal_is_clear() {
        fatal_boot("AGENT_KERNEL_MULTI_AGENT_ISOLATION_ERROR");
    }
    let Some(yielded_a) = preempted_a.resume_until_yield() else {
        fatal_boot("AGENT_KERNEL_AGENT_CPU_RESUME_ERROR");
    };
    if yielded_a.address_space_switch_count() != 2 || !preempted_b.signal_is_clear() {
        fatal_boot("AGENT_KERNEL_AGENT_CR3_SWITCH_ERROR");
    }
    serial_write_line("AGENT_KERNEL_MULTI_AGENT_ISOLATION_OK");
    let Some(second_resumed_flow) =
        first_resumed_flow.yield_first_and_dispatch_second(&mut booted, yielded_a)
    else {
        fatal_boot("AGENT_KERNEL_AGENT_CPU_YIELD_ERROR");
    };
    let Some(yielded_b) = preempted_b.resume_until_yield() else {
        fatal_boot("AGENT_KERNEL_AGENT_CPU_RESUME_ERROR");
    };
    if yielded_b.address_space_switch_count() != 2
        || !second_resumed_flow.record_second_yield(&mut booted, yielded_b)
    {
        fatal_boot("AGENT_KERNEL_AGENT_CPU_YIELD_ERROR");
    }
    serial_write_line("AGENT_KERNEL_AGENT_CPU_RESUME_OK");
    serial_write_line("AGENT_KERNEL_AGENT_CALL_YIELD_OK");
    serial_write_line("AGENT_KERNEL_AGENT_CR3_SWITCH_OK");
    serial_write_line("AGENT_KERNEL_MULTI_AGENT_CONTEXT_SWITCH_OK");
    complete_driver_flow(&mut booted, driver_setup);
    event_trace::write(booted.kernel().events());
    serial_write_line("SUPERVISOR_HANDOFF_READY");
    exit_qemu(0x10);
    halt_forever()
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
