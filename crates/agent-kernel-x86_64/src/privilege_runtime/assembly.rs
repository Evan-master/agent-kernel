//! Assembly loader for the permanent GDT and long-mode TSS.
//!
//! This child module performs the one-time privileged table load and segment
//! reload. The parent owns descriptor storage and validates the resulting
//! selectors.

use core::arch::global_asm;

use agent_kernel_x86_64::privilege::GdtPointer;

global_asm!(
    r#"
    .section .text.agent_kernel_privilege,"ax",@progbits
    .global agent_kernel_load_privilege_tables
    .type agent_kernel_load_privilege_tables,@function
agent_kernel_load_privilege_tables:
    lgdt [rdi]
    push rsi
    lea rax, [rip + .Lagent_kernel_segments_loaded]
    push rax
    retfq
.Lagent_kernel_segments_loaded:
    mov ax, dx
    mov ds, ax
    mov es, ax
    mov ss, ax
    xor eax, eax
    mov fs, ax
    mov gs, ax
    mov ax, cx
    ltr ax
    ret
    .size agent_kernel_load_privilege_tables, . - agent_kernel_load_privilege_tables
"#,
);

unsafe extern "C" {
    fn agent_kernel_load_privilege_tables(
        pointer: *const GdtPointer,
        kernel_code: u64,
        kernel_data: u64,
        tss: u64,
    );
}

pub(super) unsafe fn load_tables(
    pointer: *const GdtPointer,
    kernel_code: u16,
    kernel_data: u16,
    tss: u16,
) {
    unsafe {
        agent_kernel_load_privilege_tables(
            pointer,
            u64::from(kernel_code),
            u64::from(kernel_data),
            u64::from(tss),
        );
    }
}
