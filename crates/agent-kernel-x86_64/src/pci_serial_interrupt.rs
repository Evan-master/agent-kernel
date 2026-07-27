//! One-shot PCI serial INTx ingress through I/O APIC IRQ 11.
//!
//! The interrupt stub performs only trusted BAR register access and publishes
//! fixed-width evidence. Core mutation and Driver scheduling run later on the
//! BSP after IF is clear.

use core::{
    arch::{asm, global_asm},
    sync::atomic::{AtomicU16, AtomicU8, Ordering},
};

use agent_kernel_x86_64::interrupt::PCI_INTX_IRQ_VECTOR;

use crate::{exception_runtime, outb, smp_boot::SmpBootstrap};

const UART_IER_OFFSET: u16 = 1;
const UART_IIR_OFFSET: u16 = 2;
const UART_LSR_OFFSET: u16 = 5;
const UART_IIR_NO_INTERRUPT: u8 = 0x01;
const UART_IIR_ID_MASK: u8 = 0x06;
const UART_IIR_THRE: u8 = 0x02;
const UART_LSR_THRE: u8 = 0x20;
const IRQ_WAIT_SPINS: usize = 1_000_000;

#[no_mangle]
#[used]
static AGENT_KERNEL_PCI_SERIAL_IRQ_BASE: AtomicU16 = AtomicU16::new(0);

#[no_mangle]
#[used]
static AGENT_KERNEL_PCI_SERIAL_IRQ_SEEN: AtomicU8 = AtomicU8::new(0);

#[no_mangle]
#[used]
static AGENT_KERNEL_PCI_SERIAL_IRQ_COUNT: AtomicU8 = AtomicU8::new(0);

#[no_mangle]
#[used]
static AGENT_KERNEL_PCI_SERIAL_IRQ_IIR: AtomicU8 = AtomicU8::new(0);

#[no_mangle]
#[used]
static AGENT_KERNEL_PCI_SERIAL_IRQ_LSR: AtomicU8 = AtomicU8::new(0);

global_asm!(
    r#"
    .section .text.agent_kernel_pci_serial_irq,"ax",@progbits
    .global agent_kernel_pci_serial_irq_stub
    .type agent_kernel_pci_serial_irq_stub,@function
agent_kernel_pci_serial_irq_stub:
    push rax
    push rdx

    mov dx, word ptr [rip + {irq_base}]
    add dx, {uart_iir_offset}
    in al, dx
    mov byte ptr [rip + {irq_iir}], al

    mov dx, word ptr [rip + {irq_base}]
    add dx, {uart_lsr_offset}
    in al, dx
    mov byte ptr [rip + {irq_lsr}], al

    mov dx, word ptr [rip + {irq_base}]
    add dx, {uart_ier_offset}
    xor eax, eax
    out dx, al

    inc byte ptr [rip + {irq_count}]
    mov byte ptr [rip + {irq_seen}], 1

    pop rdx
    pop rax
    iretq
    .size agent_kernel_pci_serial_irq_stub, . - agent_kernel_pci_serial_irq_stub
"#,
    uart_ier_offset = const UART_IER_OFFSET,
    uart_iir_offset = const UART_IIR_OFFSET,
    uart_lsr_offset = const UART_LSR_OFFSET,
    irq_base = sym AGENT_KERNEL_PCI_SERIAL_IRQ_BASE,
    irq_seen = sym AGENT_KERNEL_PCI_SERIAL_IRQ_SEEN,
    irq_count = sym AGENT_KERNEL_PCI_SERIAL_IRQ_COUNT,
    irq_iir = sym AGENT_KERNEL_PCI_SERIAL_IRQ_IIR,
    irq_lsr = sym AGENT_KERNEL_PCI_SERIAL_IRQ_LSR,
);

unsafe extern "C" {
    fn agent_kernel_pci_serial_irq_stub();
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) struct PciSerialInterruptSignal {
    pub(crate) iir: u8,
    pub(crate) line_status: u8,
}

pub(crate) fn install_gate() -> Option<()> {
    // SAFETY: BSP setup owns IF and freezes the IDT only after this write.
    unsafe {
        exception_runtime::install_irq_gate(PCI_INTX_IRQ_VECTOR, agent_kernel_pci_serial_irq_stub)
    }
}

pub(crate) fn configure(base: u16) -> Option<()> {
    if base == 0 || base > u16::MAX - 7 {
        return None;
    }
    AGENT_KERNEL_PCI_SERIAL_IRQ_BASE
        .compare_exchange(0, base, Ordering::AcqRel, Ordering::Acquire)
        .ok()?;
    reset_mailbox();
    Some(())
}

pub(crate) fn wait_for_thre(smp_bootstrap: &mut SmpBootstrap) -> Option<PciSerialInterruptSignal> {
    let base = AGENT_KERNEL_PCI_SERIAL_IRQ_BASE.load(Ordering::Acquire);
    if base == 0 {
        return None;
    }
    // SAFETY: the BSP owns IF and the prepared PCI INTx route.
    unsafe {
        asm!("cli", options(nomem, nostack));
    }
    smp_bootstrap.arm_pci_intx_irq().ok()?;

    // SAFETY: the gate and active-low level route are installed. The Driver
    // backend armed the device source while this route was masked.
    unsafe {
        asm!("sti", options(nomem, nostack));
    }
    for _ in 0..IRQ_WAIT_SPINS {
        if AGENT_KERNEL_PCI_SERIAL_IRQ_SEEN.load(Ordering::Acquire) != 0 {
            break;
        }
        core::hint::spin_loop();
    }

    // SAFETY: this path reclaims IF and disables the trusted device source
    // before masking the level-triggered route and issuing EOI.
    unsafe {
        asm!("cli", options(nomem, nostack));
        outb(base + UART_IER_OFFSET, 0);
    }
    let count = AGENT_KERNEL_PCI_SERIAL_IRQ_COUNT.load(Ordering::Acquire);
    smp_bootstrap.complete_pci_intx_irq(count != 0).ok()?;
    let iir = AGENT_KERNEL_PCI_SERIAL_IRQ_IIR.load(Ordering::Acquire);
    let line_status = AGENT_KERNEL_PCI_SERIAL_IRQ_LSR.load(Ordering::Acquire);
    if count != 1
        || iir & UART_IIR_NO_INTERRUPT != 0
        || iir & UART_IIR_ID_MASK != UART_IIR_THRE
        || line_status & UART_LSR_THRE == 0
    {
        return None;
    }
    Some(PciSerialInterruptSignal { iir, line_status })
}

fn reset_mailbox() {
    AGENT_KERNEL_PCI_SERIAL_IRQ_SEEN.store(0, Ordering::Release);
    AGENT_KERNEL_PCI_SERIAL_IRQ_COUNT.store(0, Ordering::Release);
    AGENT_KERNEL_PCI_SERIAL_IRQ_IIR.store(0, Ordering::Release);
    AGENT_KERNEL_PCI_SERIAL_IRQ_LSR.store(0, Ordering::Release);
}
