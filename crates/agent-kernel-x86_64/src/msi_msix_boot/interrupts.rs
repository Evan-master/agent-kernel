//! Native IDT ingress for the V28 EDU MSI and Virtio RNG MSI-X vectors.
//!
//! Each assembly gate records one vector without touching device state. The
//! boot orchestrator reclaims IF, acknowledges the owning device, and issues
//! Local APIC EOI before another message interrupt can be accepted.

use core::{
    arch::{asm, global_asm},
    hint::spin_loop,
    sync::atomic::{AtomicU8, Ordering},
};

use crate::exception_runtime;

use super::{EDU_MSI_VECTOR, RNG_MSIX_VECTOR};

const INTERRUPT_WAIT_SPINS: usize = 100_000_000;

#[no_mangle]
#[used]
static AGENT_KERNEL_EDU_MSI_COUNT: AtomicU8 = AtomicU8::new(0);

#[no_mangle]
#[used]
static AGENT_KERNEL_RNG_MSIX_COUNT: AtomicU8 = AtomicU8::new(0);

global_asm!(
    r#"
    .section .text.agent_kernel_msi_msix,"ax",@progbits

    .global agent_kernel_edu_msi_stub
    .type agent_kernel_edu_msi_stub,@function
agent_kernel_edu_msi_stub:
    inc byte ptr [rip + {edu_count}]
    iretq
    .size agent_kernel_edu_msi_stub, . - agent_kernel_edu_msi_stub

    .global agent_kernel_rng_msix_stub
    .type agent_kernel_rng_msix_stub,@function
agent_kernel_rng_msix_stub:
    inc byte ptr [rip + {rng_count}]
    iretq
    .size agent_kernel_rng_msix_stub, . - agent_kernel_rng_msix_stub
"#,
    edu_count = sym AGENT_KERNEL_EDU_MSI_COUNT,
    rng_count = sym AGENT_KERNEL_RNG_MSIX_COUNT,
);

unsafe extern "C" {
    fn agent_kernel_edu_msi_stub();
    fn agent_kernel_rng_msix_stub();
}

pub(super) fn install_gates() -> Option<()> {
    // SAFETY: the BSP owns the mutable IDT with IF clear during boot.
    unsafe {
        exception_runtime::install_irq_gate(EDU_MSI_VECTOR, agent_kernel_edu_msi_stub)?;
        exception_runtime::install_irq_gate(RNG_MSIX_VECTOR, agent_kernel_rng_msix_stub)?;
    }
    Some(())
}

pub(super) fn reset() {
    AGENT_KERNEL_EDU_MSI_COUNT.store(0, Ordering::Release);
    AGENT_KERNEL_RNG_MSIX_COUNT.store(0, Ordering::Release);
}

pub(super) fn wait_for_counts(expected_edu: u8, expected_rng: u8) -> bool {
    // SAFETY: both gates are installed and every source is configured before
    // the proof opens IF for this bounded interval.
    unsafe {
        asm!("sti", options(nomem, nostack));
    }
    let mut matched = false;
    for _ in 0..INTERRUPT_WAIT_SPINS {
        let counts = current_counts();
        if counts == (expected_edu, expected_rng) {
            matched = true;
            break;
        }
        if counts.0 > expected_edu || counts.1 > expected_rng {
            break;
        }
        spin_loop();
    }
    // SAFETY: the BSP reclaims interrupt ownership before inspecting devices
    // or acknowledging the Local APIC.
    unsafe {
        asm!("cli", options(nomem, nostack));
    }
    matched && current_counts() == (expected_edu, expected_rng)
}

pub(super) fn current_counts() -> (u8, u8) {
    (
        AGENT_KERNEL_EDU_MSI_COUNT.load(Ordering::Acquire),
        AGENT_KERNEL_RNG_MSIX_COUNT.load(Ordering::Acquire),
    )
}
