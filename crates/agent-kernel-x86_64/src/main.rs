#![no_std]
#![no_main]

//! x86_64 bootloader entry for Agent Kernel.
//!
//! This crate owns the architecture-specific QEMU boot entry. It prints the
//! deterministic boot handoff event sequence over COM1 serial and exits QEMU
//! through isa-debug-exit when the handoff succeeds.

use core::{arch::asm, panic::PanicInfo};

use agent_kernel_boot::{BootConfig, BootedKernel};
use agent_kernel_core::EventKind;
use bootloader_api::{entry_point, BootInfo};

entry_point!(kernel_main);

fn kernel_main(_boot_info: &'static mut BootInfo) -> ! {
    serial_init();
    serial_write_line("AGENT_KERNEL_QEMU_BOOT_OK");

    match BootedKernel::<8, 8, 16, 4, 4>::boot(BootConfig::default()) {
        Ok(booted) => {
            for event in booted.kernel().events() {
                serial_write_str("event[");
                serial_write_u64(event.sequence);
                serial_write_str("] ");
                match event.kind {
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
