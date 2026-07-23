//! One-shot x86_64 COM1 IRQ4 ingress for the bare-metal boot proof.
//!
//! This architecture-binary module registers IRQ4 in the persistent IDT,
//! remaps and masks the legacy PIC, and arms the 16550 THRE source. Its assembly
//! top half only captures fixed-width hardware state and acknowledges
//! controllers; normal Rust code validates the mailbox after IF is clear again.

use core::{
    arch::{asm, global_asm},
    sync::atomic::{AtomicU8, Ordering},
};

use agent_kernel_x86_64::interrupt::{UART_IRQ_LINE, UART_IRQ_VECTOR};

use crate::{exception_runtime, inb, outb, pic, COM1};

const UART_IER_THRE: u8 = 0x02;
const UART_IIR_NO_INTERRUPT: u8 = 0x01;
const UART_IIR_ID_MASK: u8 = 0x06;
const UART_IIR_THRE: u8 = 0x02;
const UART_LSR_THRE: u8 = 0x20;
const IRQ_WAIT_SPINS: usize = 1_000_000;

#[no_mangle]
#[used]
static AGENT_KERNEL_UART_IRQ_SEEN: AtomicU8 = AtomicU8::new(0);

#[no_mangle]
#[used]
static AGENT_KERNEL_UART_IRQ_COUNT: AtomicU8 = AtomicU8::new(0);

#[no_mangle]
#[used]
static AGENT_KERNEL_UART_IRQ_IIR: AtomicU8 = AtomicU8::new(0);

#[no_mangle]
#[used]
static AGENT_KERNEL_UART_IRQ_LSR: AtomicU8 = AtomicU8::new(0);

global_asm!(
    r#"
    .section .text.agent_kernel_uart_irq,"ax",@progbits
    .global agent_kernel_uart_irq_stub
    .type agent_kernel_uart_irq_stub,@function
agent_kernel_uart_irq_stub:
    push rax
    push rdx

    mov dx, {uart_iir_port}
    in al, dx
    mov byte ptr [rip + {irq_iir}], al

    mov dx, {uart_lsr_port}
    in al, dx
    mov byte ptr [rip + {irq_lsr}], al

    inc byte ptr [rip + {irq_count}]
    mov byte ptr [rip + {irq_seen}], 1

    mov dx, {uart_ier_port}
    xor eax, eax
    out dx, al

    mov dx, {pic_master_command}
    mov al, {pic_eoi}
    out dx, al

    pop rdx
    pop rax
    iretq
    .size agent_kernel_uart_irq_stub, . - agent_kernel_uart_irq_stub
"#,
    uart_iir_port = const COM1 + 2,
    uart_lsr_port = const COM1 + 5,
    uart_ier_port = const COM1 + 1,
    pic_master_command = const pic::PIC_MASTER_COMMAND,
    pic_eoi = const pic::PIC_EOI,
    irq_seen = sym AGENT_KERNEL_UART_IRQ_SEEN,
    irq_count = sym AGENT_KERNEL_UART_IRQ_COUNT,
    irq_iir = sym AGENT_KERNEL_UART_IRQ_IIR,
    irq_lsr = sym AGENT_KERNEL_UART_IRQ_LSR,
);

unsafe extern "C" {
    fn agent_kernel_uart_irq_stub();
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct UartInterruptSignal {
    pub iir: u8,
    pub line_status: u8,
}

pub fn install_gate() -> Option<()> {
    // SAFETY: BSP setup owns IF and freezes the IDT only after this write.
    unsafe { exception_runtime::install_irq_gate(UART_IRQ_VECTOR, agent_kernel_uart_irq_stub) }
}

pub fn wait_for_uart_thre() -> Option<UartInterruptSignal> {
    // SAFETY: the ring-0 single-core boot path owns IF until this proof completes.
    unsafe {
        asm!("cli", options(nomem, nostack));
    }
    reset_mailbox();

    // SAFETY: IF is clear, the IDT gate is installed before STI, and COM1 was
    // initialized with OUT2 asserted by `serial_init`.
    unsafe {
        pic::initialize_for_irq(UART_IRQ_LINE)?;
        let _ = inb(COM1 + 2);
        outb(COM1 + 1, UART_IER_THRE);
        asm!("sti", options(nomem, nostack));
    }

    for _ in 0..IRQ_WAIT_SPINS {
        if AGENT_KERNEL_UART_IRQ_SEEN.load(Ordering::Acquire) != 0 {
            break;
        }
        core::hint::spin_loop();
    }

    // SAFETY: this path reclaims IF before masking both interrupt controllers.
    unsafe {
        asm!("cli", options(nomem, nostack));
        outb(COM1 + 1, 0);
        pic::mask_all();
    }

    let count = AGENT_KERNEL_UART_IRQ_COUNT.load(Ordering::Acquire);
    let iir = AGENT_KERNEL_UART_IRQ_IIR.load(Ordering::Acquire);
    let line_status = AGENT_KERNEL_UART_IRQ_LSR.load(Ordering::Acquire);
    if count != 1
        || iir & UART_IIR_NO_INTERRUPT != 0
        || iir & UART_IIR_ID_MASK != UART_IIR_THRE
        || line_status & UART_LSR_THRE == 0
    {
        return None;
    }

    Some(UartInterruptSignal { iir, line_status })
}

fn reset_mailbox() {
    AGENT_KERNEL_UART_IRQ_SEEN.store(0, Ordering::Release);
    AGENT_KERNEL_UART_IRQ_COUNT.store(0, Ordering::Release);
    AGENT_KERNEL_UART_IRQ_IIR.store(0, Ordering::Release);
    AGENT_KERNEL_UART_IRQ_LSR.store(0, Ordering::Release);
}
