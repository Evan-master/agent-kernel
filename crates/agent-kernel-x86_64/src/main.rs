#![no_std]
#![no_main]

//! x86_64 bootloader entry for Agent Kernel.
//!
//! This crate owns the architecture-specific QEMU boot entry. It proves
//! persistent exceptions, physical PIT-driven task preemption, and the bounded
//! COM1 interrupt-to-command flow before publishing the deterministic handoff.

use core::{arch::asm, panic::PanicInfo};

use agent_kernel_boot::{BootConfig, BootedKernel};
use bootloader_api::{entry_point, BootInfo, BootloaderConfig};

mod event_trace;
mod exception_runtime;
mod pic;
mod pit_timer;
mod port_driver_flow;
mod timer_task_flow;
mod uart_interrupt;

use port_driver_flow::PortDriverSetup;
use timer_task_flow::TimerTaskFlow;

const KERNEL_STACK_SIZE: u64 = 256 * 1024;

static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.kernel_stack_size = KERNEL_STACK_SIZE;
    config
};

entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

pub(crate) type X86BootedKernel = BootedKernel<3, 1, 3, 48, 1, 1, 0, 1, 1, 1, 1, 1, 1, 1>;

fn kernel_main(_boot_info: &'static mut BootInfo) -> ! {
    serial_init();
    serial_write_line("AGENT_KERNEL_QEMU_BOOT_OK");
    if exception_runtime::install_and_probe().is_none() {
        fatal_boot("AGENT_KERNEL_EXCEPTION_BASELINE_ERROR");
    }
    serial_write_line("AGENT_KERNEL_EXCEPTION_BASELINE_OK");

    match X86BootedKernel::boot(BootConfig::default()) {
        Ok(mut booted) => {
            let Some(driver_setup) = PortDriverSetup::prepare(&mut booted, COM1) else {
                fatal_boot("AGENT_KERNEL_PORT_DRIVER_SETUP_ERROR");
            };
            let Some(timer_flow) = TimerTaskFlow::prepare(&mut booted) else {
                fatal_boot("AGENT_KERNEL_TIMER_TASK_SETUP_ERROR");
            };
            let Some(timer_signal) = pit_timer::wait_for_tick() else {
                fatal_boot("AGENT_KERNEL_PIT_IRQ_ERROR");
            };
            serial_write_line("AGENT_KERNEL_PIT_IRQ_OK");
            if !timer_flow.apply_tick(&mut booted, timer_signal) {
                fatal_boot("AGENT_KERNEL_TIMER_PREEMPTION_ERROR");
            }
            serial_write_line("AGENT_KERNEL_TIMER_PREEMPTION_OK");

            let Some(uart_signal) = uart_interrupt::wait_for_uart_thre() else {
                fatal_boot("AGENT_KERNEL_UART_IRQ_ERROR");
            };
            serial_write_line("AGENT_KERNEL_UART_IRQ_OK");
            let Some(mut port_flow) = driver_setup.dispatch_interrupt(
                &mut booted,
                uart_signal.iir,
                uart_signal.line_status,
                b'O',
            ) else {
                fatal_boot("AGENT_KERNEL_PORT_DRIVER_INTERRUPT_ERROR");
            };
            serial_write_str("AGENT_KERNEL_PORT_IO_BACKEND_");
            while !serial_transmit_empty() {}
            if !port_flow.execute_and_record(&mut booted) {
                fatal_boot("ERROR");
            }
            serial_write_line("K");
            serial_write_line("AGENT_KERNEL_PORT_COMMAND_FLOW_OK");
            serial_write_line("AGENT_KERNEL_DRIVER_INVOCATION_FLOW_OK");
            event_trace::write(booted.kernel().events());
            serial_write_line("SUPERVISOR_HANDOFF_READY");
            exit_qemu(0x10);
        }
        Err(_) => {
            fatal_boot("AGENT_KERNEL_BOOT_ERROR");
        }
    }

    halt_forever()
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    serial_init();
    fatal_boot("AGENT_KERNEL_PANIC")
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

fn fatal_boot(message: &str) -> ! {
    serial_write_line(message);
    exit_qemu(0x11);
    halt_forever()
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
