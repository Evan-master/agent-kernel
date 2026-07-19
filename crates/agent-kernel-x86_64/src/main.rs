#![no_std]
#![no_main]

//! x86_64 bootloader entry for Agent Kernel.
//!
//! This crate owns the architecture-specific QEMU boot entry. It proves
//! persistent exceptions, CPL3 Agent preemption, and the bounded COM1
//! interrupt-to-command flow before publishing the deterministic handoff.

use core::{arch::asm, panic::PanicInfo};

use agent_kernel_boot::BootedKernel;
use bootloader_api::{entry_point, BootInfo};

mod admission_supervisor_flow;
mod agent_boot_flow;
mod agent_cpu;
mod agent_memory;
mod boot_agent_images;
mod boot_config;
mod event_trace;
mod exception_runtime;
mod fault_handler_flow;
mod fault_task_flow;
mod native_address_space_service;
mod native_agent_executor;
mod native_agent_runtime;
mod native_runtime_admission_broker;
mod pic;
mod pit_timer;
mod port_driver_flow;
mod privilege_runtime;
mod resource_manager_flow;
mod reuse_worker_flow;
mod timer_task_flow;
mod uart_interrupt;
mod verifier_task_flow;

use boot_config::BOOTLOADER_CONFIG;
use privilege_runtime::PrivilegeBoundary;

entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

pub(crate) const X86_TASK_CAPACITY: usize = 12;
pub(crate) const X86_INTENT_CAPACITY: usize = 12;
pub(crate) const X86_CAPABILITY_CAPACITY: usize = 26;
pub(crate) const X86_RUNTIME_ADMISSION_CAPACITY: usize = 16;
pub(crate) const X86_WAITER_CAPACITY: usize = 3;
pub(crate) const X86_FAULT_CAPACITY: usize = 4;
pub(crate) type X86BootedKernel = BootedKernel<
    14,
    7,
    X86_CAPABILITY_CAPACITY,
    378,
    1,
    1,
    0,
    X86_INTENT_CAPACITY,
    X86_TASK_CAPACITY,
    2,
    1,
    1,
    1,
    1,
    4,
    X86_WAITER_CAPACITY,
    X86_FAULT_CAPACITY,
    1,
    1,
    5,
    X86_RUNTIME_ADMISSION_CAPACITY,
>;

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    serial_init();
    serial_write_line("AGENT_KERNEL_QEMU_BOOT_OK");
    let Some(privilege_boundary) = PrivilegeBoundary::install() else {
        fatal_boot("AGENT_KERNEL_GDT_TSS_ERROR");
    };
    serial_write_line("AGENT_KERNEL_GDT_TSS_OK");
    if exception_runtime::install_and_probe().is_none() {
        fatal_boot("AGENT_KERNEL_EXCEPTION_BASELINE_ERROR");
    }
    serial_write_line("AGENT_KERNEL_EXCEPTION_BASELINE_OK");
    agent_boot_flow::run(boot_info, privilege_boundary)
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
