//! Minimal Local APIC interrupt entries installed before AP startup.

use core::arch::global_asm;

global_asm!(
    r#"
    .section .text.agent_kernel_apic_interrupts,"ax",@progbits

    .global agent_kernel_apic_spurious_stub
    .type agent_kernel_apic_spurious_stub,@function
agent_kernel_apic_spurious_stub:
    iretq
    .size agent_kernel_apic_spurious_stub, . - agent_kernel_apic_spurious_stub
"#,
);

unsafe extern "C" {
    fn agent_kernel_apic_spurious_stub();
}

pub(super) const fn spurious_handler() -> unsafe extern "C" fn() {
    agent_kernel_apic_spurious_stub
}
