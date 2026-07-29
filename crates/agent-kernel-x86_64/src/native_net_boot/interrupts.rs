//! Native IDT ingress for the V29 Virtio network MSI-X vectors.
//!
//! Each gate records one queue identity. The orchestrator reclaims IF and
//! issues Local APIC EOI before the lower-priority vector can be delivered.

use core::{
    arch::{asm, global_asm},
    hint::spin_loop,
    sync::atomic::{AtomicU8, Ordering},
};

use crate::exception_runtime;

use super::{NET_RX_MSIX_VECTOR, NET_TX_MSIX_VECTOR};

const INTERRUPT_WAIT_SPINS: usize = 100_000_000;

#[no_mangle]
#[used]
static AGENT_KERNEL_NET_RX_MSIX_COUNT: AtomicU8 = AtomicU8::new(0);

#[no_mangle]
#[used]
static AGENT_KERNEL_NET_TX_MSIX_COUNT: AtomicU8 = AtomicU8::new(0);

global_asm!(
    r#"
    .section .text.agent_kernel_native_net,"ax",@progbits

    .global agent_kernel_net_rx_msix_stub
    .type agent_kernel_net_rx_msix_stub,@function
agent_kernel_net_rx_msix_stub:
    inc byte ptr [rip + {rx_count}]
    iretq
    .size agent_kernel_net_rx_msix_stub, . - agent_kernel_net_rx_msix_stub

    .global agent_kernel_net_tx_msix_stub
    .type agent_kernel_net_tx_msix_stub,@function
agent_kernel_net_tx_msix_stub:
    inc byte ptr [rip + {tx_count}]
    iretq
    .size agent_kernel_net_tx_msix_stub, . - agent_kernel_net_tx_msix_stub
"#,
    rx_count = sym AGENT_KERNEL_NET_RX_MSIX_COUNT,
    tx_count = sym AGENT_KERNEL_NET_TX_MSIX_COUNT,
);

unsafe extern "C" {
    fn agent_kernel_net_rx_msix_stub();
    fn agent_kernel_net_tx_msix_stub();
}

pub(super) fn install_gates() -> Option<()> {
    // SAFETY: the BSP owns the mutable IDT with IF clear during boot.
    unsafe {
        exception_runtime::install_irq_gate(NET_RX_MSIX_VECTOR, agent_kernel_net_rx_msix_stub)?;
        exception_runtime::install_irq_gate(NET_TX_MSIX_VECTOR, agent_kernel_net_tx_msix_stub)?;
    }
    Some(())
}

pub(super) fn reset() {
    AGENT_KERNEL_NET_RX_MSIX_COUNT.store(0, Ordering::Release);
    AGENT_KERNEL_NET_TX_MSIX_COUNT.store(0, Ordering::Release);
}

pub(super) fn wait_for_counts(expected_rx: u8, expected_tx: u8) -> bool {
    // SAFETY: both gates and sources are configured before this bounded wait.
    unsafe {
        asm!("sti", options(nomem, nostack));
    }
    let mut matched = false;
    for _ in 0..INTERRUPT_WAIT_SPINS {
        let counts = current_counts();
        if counts == (expected_rx, expected_tx) {
            matched = true;
            break;
        }
        if counts.0 > expected_rx || counts.1 > expected_tx {
            break;
        }
        spin_loop();
    }
    // SAFETY: the BSP reclaims interrupt ownership before device inspection.
    unsafe {
        asm!("cli", options(nomem, nostack));
    }
    matched && current_counts() == (expected_rx, expected_tx)
}

pub(super) fn observe_no_interrupts() -> bool {
    // SAFETY: the proof owns both installed gates and all enabled sources.
    unsafe {
        asm!("sti", options(nomem, nostack));
    }
    let mut quiet = true;
    for _ in 0..INTERRUPT_WAIT_SPINS {
        if current_counts() != (0, 0) {
            quiet = false;
            break;
        }
        spin_loop();
    }
    // SAFETY: the BSP reclaims interrupt ownership before fault inspection.
    unsafe {
        asm!("cli", options(nomem, nostack));
    }
    quiet && current_counts() == (0, 0)
}

pub(super) fn current_counts() -> (u8, u8) {
    (
        AGENT_KERNEL_NET_RX_MSIX_COUNT.load(Ordering::Acquire),
        AGENT_KERNEL_NET_TX_MSIX_COUNT.load(Ordering::Acquire),
    )
}
