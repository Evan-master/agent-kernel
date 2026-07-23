//! x86_64 PIT IRQ0 source control for Agent CPU preemption.
//!
//! This architecture-binary module only installs a supplied IRQ gate, programs
//! channel 0, and controls the shared PIC mask. The Agent CPU runtime owns the
//! interrupt frame and mailbox; Agent Kernel task state never enters this
//! privileged hardware boundary.

use core::arch::asm;

use agent_kernel_x86_64::interrupt::{
    PIT_CHANNEL0_COMMAND, PIT_CHANNEL0_DATA_PORT, PIT_COMMAND_PORT, PIT_DIVISOR, PIT_IRQ_LINE,
    PIT_IRQ_VECTOR,
};

use crate::{exception_runtime, outb, pic};

pub(super) fn install_gate(handler: unsafe extern "C" fn()) -> Option<()> {
    // SAFETY: BSP setup owns IF and freezes the IDT only after this write.
    unsafe { exception_runtime::install_irq_gate(PIT_IRQ_VECTOR, handler) }
}

pub(super) fn arm() -> Option<()> {
    // SAFETY: this single-core boot proof configures the gate and controllers
    // with IF clear. The caller enables interrupts only after entering the
    // dedicated Agent stack.
    unsafe {
        asm!("cli", options(nomem, nostack));
        pic::mask_all();
        program_channel_zero();
        pic::initialize_for_irq(PIT_IRQ_LINE)?;
    }
    Some(())
}

pub(super) fn disarm() {
    // SAFETY: masking occurs in trusted normal context after IRQ0 has already
    // restored the kernel stack, or on an initialization failure with IF clear.
    unsafe {
        asm!("cli", options(nomem, nostack));
        pic::mask_all();
    }
}

unsafe fn program_channel_zero() {
    let [low, high] = PIT_DIVISOR.to_le_bytes();
    unsafe {
        outb(PIT_COMMAND_PORT, PIT_CHANNEL0_COMMAND);
        outb(PIT_CHANNEL0_DATA_PORT, low);
        outb(PIT_CHANNEL0_DATA_PORT, high);
    }
}
