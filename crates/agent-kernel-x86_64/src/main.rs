#![no_std]
#![no_main]

//! x86_64 bootloader entry for Agent Kernel.
//!
//! This crate owns the architecture-specific QEMU boot entry. It runs the
//! bounded COM1 interrupt-to-command flow, prints the deterministic boot
//! handoff event sequence, and exits QEMU through isa-debug-exit when the
//! handoff succeeds.

use core::{arch::asm, panic::PanicInfo};

use agent_kernel_boot::{BootConfig, BootedKernel};
use agent_kernel_core::EventKind;
use bootloader_api::{entry_point, BootInfo, BootloaderConfig};

mod exception_runtime;
mod port_driver_flow;
mod uart_interrupt;

use port_driver_flow::PortDriverSetup;

const KERNEL_STACK_SIZE: u64 = 256 * 1024;

static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.kernel_stack_size = KERNEL_STACK_SIZE;
    config
};

entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

pub(crate) type X86BootedKernel = BootedKernel<2, 1, 2, 32, 1, 1, 0, 0, 0, 0, 1, 1, 1, 1>;

fn kernel_main(_boot_info: &'static mut BootInfo) -> ! {
    serial_init();
    serial_write_line("AGENT_KERNEL_QEMU_BOOT_OK");
    if exception_runtime::install_and_probe().is_none() {
        serial_write_line("AGENT_KERNEL_EXCEPTION_BASELINE_ERROR");
        exit_qemu(0x11);
        halt_forever();
    }
    serial_write_line("AGENT_KERNEL_EXCEPTION_BASELINE_OK");

    match X86BootedKernel::boot(BootConfig::default()) {
        Ok(mut booted) => {
            let Some(setup) = PortDriverSetup::prepare(&mut booted, COM1) else {
                serial_write_line("AGENT_KERNEL_PORT_DRIVER_SETUP_ERROR");
                exit_qemu(0x11);
                halt_forever();
            };
            let Some(signal) = uart_interrupt::wait_for_uart_thre() else {
                serial_write_line("AGENT_KERNEL_UART_IRQ_ERROR");
                exit_qemu(0x11);
                halt_forever();
            };
            serial_write_line("AGENT_KERNEL_UART_IRQ_OK");
            let Some(mut flow) =
                setup.dispatch_interrupt(&mut booted, signal.iir, signal.line_status, b'O')
            else {
                serial_write_line("AGENT_KERNEL_PORT_DRIVER_INTERRUPT_ERROR");
                exit_qemu(0x11);
                halt_forever();
            };
            serial_write_str("AGENT_KERNEL_PORT_IO_BACKEND_");
            while !serial_transmit_empty() {}
            if !flow.execute_and_record(&mut booted) {
                serial_write_line("ERROR");
                exit_qemu(0x11);
                halt_forever();
            }
            serial_write_line("K");
            serial_write_line("AGENT_KERNEL_PORT_COMMAND_FLOW_OK");
            serial_write_line("AGENT_KERNEL_DRIVER_INVOCATION_FLOW_OK");
            for event in booted.kernel().events() {
                serial_write_str("event[");
                serial_write_u64(event.sequence);
                serial_write_str("] ");
                match event.kind {
                    EventKind::AgentRegistered => {
                        serial_write_line("agent_registered");
                    }
                    EventKind::AgentImageRegistered => {
                        serial_write_line("agent_image_registered");
                    }
                    EventKind::AgentImageVerified => {
                        serial_write_line("agent_image_verified");
                    }
                    EventKind::AgentImageRetired => {
                        serial_write_line("agent_image_retired");
                    }
                    EventKind::AgentLaunched => {
                        serial_write_line("agent_launched");
                    }
                    EventKind::AgentSuspended => {
                        serial_write_line("agent_suspended");
                    }
                    EventKind::AgentResumed => {
                        serial_write_line("agent_resumed");
                    }
                    EventKind::AgentRetired => {
                        serial_write_line("agent_retired");
                    }
                    EventKind::DriverEndpointRegistered => {
                        serial_write_line("driver_endpoint_registered");
                    }
                    EventKind::DriverBound => {
                        serial_write_line("driver_bound");
                    }
                    EventKind::DeviceEventRaised => {
                        serial_write_line("device_event_raised");
                    }
                    EventKind::DeviceEventDelivered => {
                        serial_write_line("device_event_delivered");
                    }
                    EventKind::DeviceEventAcknowledged => {
                        serial_write_line("device_event_acknowledged");
                    }
                    EventKind::DriverInvocationQueued => {
                        serial_write_line("driver_invocation_queued");
                    }
                    EventKind::DriverInvocationDispatched => {
                        serial_write_line("driver_invocation_dispatched");
                    }
                    EventKind::DriverInvocationTicked => {
                        serial_write_line("driver_invocation_ticked");
                    }
                    EventKind::DriverInvocationQuantumExpired => {
                        serial_write_line("driver_invocation_quantum_expired");
                    }
                    EventKind::DriverInvocationCompleted => {
                        serial_write_line("driver_invocation_completed");
                    }
                    EventKind::DriverCommandSubmitted => {
                        serial_write_line("driver_command_submitted");
                    }
                    EventKind::DriverCommandDispatched => {
                        serial_write_line("driver_command_dispatched");
                    }
                    EventKind::DriverCommandCompleted => {
                        serial_write_line("driver_command_completed");
                    }
                    EventKind::DriverCommandFailed => {
                        serial_write_line("driver_command_failed");
                    }
                    EventKind::ResourceCreated => {
                        serial_write_line("resource_created");
                    }
                    EventKind::ResourceRetired => {
                        serial_write_line("resource_retired");
                    }
                    EventKind::CapabilityGranted => {
                        serial_write_line("capability_granted");
                    }
                    EventKind::CapabilityDerived => {
                        serial_write_line("capability_derived");
                    }
                    EventKind::CapabilityRevoked => {
                        serial_write_line("capability_revoked");
                    }
                    EventKind::IntentDeclared => {
                        serial_write_line("intent_declared");
                    }
                    EventKind::IntentBound => {
                        serial_write_line("intent_bound");
                    }
                    EventKind::IntentFulfilled => {
                        serial_write_line("intent_fulfilled");
                    }
                    EventKind::IntentCancelled => {
                        serial_write_line("intent_cancelled");
                    }
                    EventKind::Observation => {
                        serial_write_line("observation");
                    }
                    EventKind::ActionExecuted => {
                        serial_write_line("action");
                    }
                    EventKind::VerificationRequested => {
                        serial_write_line("verification");
                    }
                    EventKind::CheckpointCreated => {
                        serial_write_line("checkpoint");
                    }
                    EventKind::RollbackRequested => {
                        serial_write_line("rollback");
                    }
                    EventKind::DelegationRequested => {
                        serial_write_line("delegation");
                    }
                    EventKind::TaskCreated => {
                        serial_write_line("task_created");
                    }
                    EventKind::TaskAccepted => {
                        serial_write_line("task_accepted");
                    }
                    EventKind::TaskCompleted => {
                        serial_write_line("task_completed");
                    }
                    EventKind::TaskVerified => {
                        serial_write_line("task_verified");
                    }
                    EventKind::TaskCancelled => {
                        serial_write_line("task_cancelled");
                    }
                    EventKind::TaskQueued => {
                        serial_write_line("task_queued");
                    }
                    EventKind::TaskDispatched => {
                        serial_write_line("task_dispatched");
                    }
                    EventKind::TaskYielded => {
                        serial_write_line("task_yielded");
                    }
                    EventKind::TaskTicked => {
                        serial_write_line("task_ticked");
                    }
                    EventKind::TaskQuantumExpired => {
                        serial_write_line("task_quantum_expired");
                    }
                    EventKind::TaskWaiting => {
                        serial_write_line("task_waiting");
                    }
                    EventKind::TaskWoken => {
                        serial_write_line("task_woken");
                    }
                    EventKind::TaskFaulted => {
                        serial_write_line("task_faulted");
                    }
                    EventKind::TaskFaultRecovered => {
                        serial_write_line("task_fault_recovered");
                    }
                    EventKind::SignalEmitted => {
                        serial_write_line("signal_emitted");
                    }
                    EventKind::FaultHandlerInstalled => {
                        serial_write_line("fault_handler_installed");
                    }
                    EventKind::FaultRouted => {
                        serial_write_line("fault_routed");
                    }
                    EventKind::FaultPolicyInstalled => {
                        serial_write_line("fault_policy_installed");
                    }
                    EventKind::FaultPolicyApplied => {
                        serial_write_line("fault_policy_applied");
                    }
                    EventKind::MessageSent => {
                        serial_write_line("message_sent");
                    }
                    EventKind::MessageReceived => {
                        serial_write_line("message_received");
                    }
                    EventKind::MessageAcknowledged => {
                        serial_write_line("message_acknowledged");
                    }
                    EventKind::MemoryCellCreated => {
                        serial_write_line("memory_cell_created");
                    }
                    EventKind::MemoryCellRecalled => {
                        serial_write_line("memory_cell_recalled");
                    }
                    EventKind::MemoryCellRemembered => {
                        serial_write_line("memory_cell_remembered");
                    }
                    EventKind::NamespaceEntryBound => {
                        serial_write_line("namespace_entry_bound");
                    }
                    EventKind::NamespaceEntryResolved => {
                        serial_write_line("namespace_entry_resolved");
                    }
                    EventKind::NamespaceEntryRebound => {
                        serial_write_line("namespace_entry_rebound");
                    }
                }
            }
            serial_write_line("SUPERVISOR_HANDOFF_READY");
            exit_qemu(0x10);
        }
        Err(_) => {
            serial_write_line("AGENT_KERNEL_BOOT_ERROR");
            exit_qemu(0x11);
        }
    }

    halt_forever()
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    serial_init();
    serial_write_line("AGENT_KERNEL_PANIC");
    exit_qemu(0x11);
    halt_forever()
}

const COM1: u16 = 0x3f8;
const QEMU_EXIT_PORT: u16 = 0xf4;

fn serial_init() {
    unsafe {
        outb(COM1 + 1, 0x00);
        outb(COM1 + 3, 0x80);
        outb(COM1, 0x03);
        outb(COM1 + 1, 0x00);
        outb(COM1 + 3, 0x03);
        outb(COM1 + 2, 0xc7);
        outb(COM1 + 4, 0x0b);
    }
}

fn serial_write_line(text: &str) {
    serial_write_str(text);
    serial_write_str("\n");
}

fn serial_write_str(text: &str) {
    for byte in text.bytes() {
        serial_write_byte(byte);
    }
}

fn serial_write_u64(mut value: u64) {
    if value == 0 {
        serial_write_byte(b'0');
        return;
    }

    let mut digits = [0u8; 20];
    let mut len = 0;
    while value > 0 {
        digits[len] = b'0' + (value % 10) as u8;
        value /= 10;
        len += 1;
    }
    while len > 0 {
        len -= 1;
        serial_write_byte(digits[len]);
    }
}

fn serial_write_byte(byte: u8) {
    while !serial_transmit_empty() {}
    unsafe {
        outb(COM1, byte);
    }
}

fn serial_transmit_empty() -> bool {
    unsafe { inb(COM1 + 5) & 0x20 != 0 }
}

fn exit_qemu(code: u8) {
    unsafe {
        outb(QEMU_EXIT_PORT, code);
    }
}

fn halt_forever() -> ! {
    loop {
        unsafe {
            asm!("hlt", options(nomem, nostack, preserves_flags));
        }
    }
}

unsafe fn outb(port: u16, value: u8) {
    unsafe {
        asm!("out dx, al", in("dx") port, in("al") value, options(nomem, nostack, preserves_flags));
    }
}

unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    unsafe {
        asm!("in al, dx", in("dx") port, out("al") value, options(nomem, nostack, preserves_flags));
    }
    value
}
