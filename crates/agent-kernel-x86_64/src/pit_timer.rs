//! One-shot x86_64 PIT IRQ0 ingress for scheduler tick delivery.
//!
//! This architecture-binary module programs channel 0, installs its persistent
//! IDT gate, and captures one interrupt in a fixed atomic mailbox. The assembly
//! top half owns no Agent Kernel authority; normal Rust validates the signal
//! after IF is clear and hands it to the scheduler bottom half.

use core::{
    arch::{asm, global_asm},
    sync::atomic::{AtomicU8, Ordering},
};

use agent_kernel_x86_64::interrupt::{
    PIT_CHANNEL0_COMMAND, PIT_CHANNEL0_DATA_PORT, PIT_COMMAND_PORT, PIT_DIVISOR, PIT_IRQ_LINE,
    PIT_IRQ_VECTOR,
};

use crate::{exception_runtime, outb, pic};

const IRQ_WAIT_SPINS: usize = 20_000_000;

#[no_mangle]
#[used]
static AGENT_KERNEL_PIT_IRQ_SEEN: AtomicU8 = AtomicU8::new(0);

#[no_mangle]
#[used]
static AGENT_KERNEL_PIT_IRQ_COUNT: AtomicU8 = AtomicU8::new(0);

global_asm!(
    r#"
    .section .text.agent_kernel_pit_irq,"ax",@progbits
    .global agent_kernel_pit_irq_stub
    .type agent_kernel_pit_irq_stub,@function
agent_kernel_pit_irq_stub:
    push rax
    push rdx

    inc byte ptr [rip + {irq_count}]

    mov dx, {pic_master_data}
    mov al, 0xff
    out dx, al

    mov dx, {pic_master_command}
    mov al, {pic_eoi}
    out dx, al

    mov byte ptr [rip + {irq_seen}], 1
    pop rdx
    pop rax
    iretq
    .size agent_kernel_pit_irq_stub, . - agent_kernel_pit_irq_stub
"#,
    pic_master_data = const pic::PIC_MASTER_DATA,
    pic_master_command = const pic::PIC_MASTER_COMMAND,
    pic_eoi = const pic::PIC_EOI,
    irq_seen = sym AGENT_KERNEL_PIT_IRQ_SEEN,
    irq_count = sym AGENT_KERNEL_PIT_IRQ_COUNT,
);

unsafe extern "C" {
    fn agent_kernel_pit_irq_stub();
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct PitTimerSignal {
    ticks: u8,
}

impl PitTimerSignal {
    pub(super) const fn count(self) -> u8 {
        self.ticks
    }
}

pub(super) fn wait_for_tick() -> Option<PitTimerSignal> {
    // SAFETY: the ring-0 single-core boot path owns IF and both PICs here.
    unsafe {
        asm!("cli", options(nomem, nostack));
    }
    reset_mailbox();

    // SAFETY: the gate and source are configured while IF is clear. Channel 0
    // is programmed before the remapped PIC exposes IRQ0.
    unsafe {
        exception_runtime::install_irq_gate(PIT_IRQ_VECTOR, agent_kernel_pit_irq_stub)?;
        pic::mask_all();
        program_channel_zero();
        pic::initialize_for_irq(PIT_IRQ_LINE)?;
        asm!("sti", options(nomem, nostack));
    }

    for _ in 0..IRQ_WAIT_SPINS {
        if AGENT_KERNEL_PIT_IRQ_SEEN.load(Ordering::Acquire) != 0 {
            break;
        }
        core::hint::spin_loop();
    }

    // SAFETY: normal context reclaims IF before final controller masking and
    // mailbox validation.
    unsafe {
        asm!("cli", options(nomem, nostack));
        pic::mask_all();
    }

    let seen = AGENT_KERNEL_PIT_IRQ_SEEN.load(Ordering::Acquire);
    let ticks = AGENT_KERNEL_PIT_IRQ_COUNT.load(Ordering::Acquire);
    if seen != 1 || ticks != 1 {
        return None;
    }

    Some(PitTimerSignal { ticks })
}

fn reset_mailbox() {
    AGENT_KERNEL_PIT_IRQ_SEEN.store(0, Ordering::Release);
    AGENT_KERNEL_PIT_IRQ_COUNT.store(0, Ordering::Release);
}

unsafe fn program_channel_zero() {
    let [low, high] = PIT_DIVISOR.to_le_bytes();
    unsafe {
        outb(PIT_COMMAND_PORT, PIT_CHANNEL0_COMMAND);
        outb(PIT_CHANNEL0_DATA_PORT, low);
        outb(PIT_CHANNEL0_DATA_PORT, high);
    }
}
