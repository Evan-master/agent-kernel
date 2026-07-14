//! Ring-3 entry, resume, PIT, and Agent-call assembly.
//!
//! This child module owns only register and privilege-frame mechanics. It never
//! calls Rust while a user frame is active; the parent validates all recorded
//! evidence after control returns to CPL0.

use core::arch::global_asm;

use agent_kernel_x86_64::context::PRIVILEGE_INTERRUPT_RIP_OFFSET;

use crate::pic;

global_asm!(
    r#"
    .section .text.agent_kernel_cpu_context,"ax",@progbits

    .macro agent_kernel_push_integer_frame
    push rax
    push rbx
    push rcx
    push rdx
    push rsi
    push rdi
    push rbp
    push r8
    push r9
    push r10
    push r11
    push r12
    push r13
    push r14
    push r15
    .endm

    .macro agent_kernel_restore_host
    mov rsp, qword ptr [rip + {host_context}]
    pop r15
    pop r14
    pop r13
    pop r12
    pop rbx
    pop rbp
    ret
    .endm

    .global agent_kernel_enter_user
    .type agent_kernel_enter_user,@function
agent_kernel_enter_user:
    lea rax, [rip + .Lagent_kernel_user_host]
    push rax
    push rbp
    push rbx
    push r12
    push r13
    push r14
    push r15
    mov qword ptr [rdi], rsp

    push r8
    push rdx
    pushfq
    pop rax
    and rax, -28929
    or rax, 0x202
    push rax
    push rcx
    push rsi
    iretq
.Lagent_kernel_user_host:
    cli
    ret
    .size agent_kernel_enter_user, . - agent_kernel_enter_user

    .global agent_kernel_resume_interrupted_user
    .type agent_kernel_resume_interrupted_user,@function
agent_kernel_resume_interrupted_user:
    lea rax, [rip + .Lagent_kernel_user_host]
    push rax
    push rbp
    push rbx
    push r12
    push r13
    push r14
    push r15
    mov qword ptr [rdi], rsp
    mov rsp, rsi
    pop r15
    pop r14
    pop r13
    pop r12
    pop r11
    pop r10
    pop r9
    pop r8
    pop rbp
    pop rdi
    pop rsi
    pop rdx
    pop rcx
    pop rbx
    pop rax
    iretq
    .size agent_kernel_resume_interrupted_user, . - agent_kernel_resume_interrupted_user

    .global agent_kernel_agent_timer_irq_stub
    .type agent_kernel_agent_timer_irq_stub,@function
agent_kernel_agent_timer_irq_stub:
    agent_kernel_push_integer_frame
    mov qword ptr [rip + {interrupt_rsp}], rsp
    mov rax, qword ptr [rsp + {rip_offset}]
    mov qword ptr [rip + {interrupt_rip}], rax
    inc byte ptr [rip + {irq_count}]

    mov dx, {pic_master_data}
    mov al, 0xff
    out dx, al
    mov dx, {pic_master_command}
    mov al, {pic_eoi}
    out dx, al

    mov byte ptr [rip + {irq_seen}], 1
    mov byte ptr [rip + {preempted}], 1
    agent_kernel_restore_host
    .size agent_kernel_agent_timer_irq_stub, . - agent_kernel_agent_timer_irq_stub

    .global agent_kernel_agent_call_stub
    .type agent_kernel_agent_call_stub,@function
agent_kernel_agent_call_stub:
    agent_kernel_push_integer_frame
    mov qword ptr [rip + {call_rsp}], rsp
    mov rax, qword ptr [rsp + {rip_offset}]
    mov qword ptr [rip + {call_rip}], rax
    inc byte ptr [rip + {call_count}]
    mov byte ptr [rip + {call_seen}], 1
    mov byte ptr [rip + {yielded}], 1
    agent_kernel_restore_host
    .size agent_kernel_agent_call_stub, . - agent_kernel_agent_call_stub
"#,
    interrupt_rsp = sym super::storage::AGENT_KERNEL_AGENT_INTERRUPT_RSP,
    interrupt_rip = sym super::storage::AGENT_KERNEL_AGENT_INTERRUPT_RIP,
    irq_count = sym super::storage::AGENT_KERNEL_AGENT_IRQ_COUNT,
    irq_seen = sym super::storage::AGENT_KERNEL_AGENT_IRQ_SEEN,
    preempted = sym super::storage::AGENT_KERNEL_AGENT_PREEMPTED,
    call_rsp = sym super::storage::AGENT_KERNEL_AGENT_CALL_RSP,
    call_rip = sym super::storage::AGENT_KERNEL_AGENT_CALL_RIP,
    call_count = sym super::storage::AGENT_KERNEL_AGENT_CALL_COUNT,
    call_seen = sym super::storage::AGENT_KERNEL_AGENT_CALL_SEEN,
    yielded = sym super::storage::AGENT_KERNEL_AGENT_YIELDED,
    host_context = sym super::storage::AGENT_KERNEL_HOST_CONTEXT_RSP,
    rip_offset = const PRIVILEGE_INTERRUPT_RIP_OFFSET,
    pic_master_data = const pic::PIC_MASTER_DATA,
    pic_master_command = const pic::PIC_MASTER_COMMAND,
    pic_eoi = const pic::PIC_EOI,
);

unsafe extern "C" {
    fn agent_kernel_enter_user(
        host_rsp: *mut u64,
        user_rip: u64,
        user_rsp: u64,
        user_cs: u64,
        user_ss: u64,
    );
    fn agent_kernel_resume_interrupted_user(host_rsp: *mut u64, interrupt_rsp: u64);
    pub(super) fn agent_kernel_agent_timer_irq_stub();
    pub(super) fn agent_kernel_agent_call_stub();
}

pub(super) unsafe fn enter_user(
    host_rsp: *mut u64,
    user_rip: u64,
    user_rsp: u64,
    user_cs: u16,
    user_ss: u16,
) {
    unsafe {
        agent_kernel_enter_user(
            host_rsp,
            user_rip,
            user_rsp,
            u64::from(user_cs),
            u64::from(user_ss),
        );
    }
}

pub(super) unsafe fn resume_interrupted_user(host_rsp: *mut u64, interrupt_rsp: u64) {
    unsafe {
        agent_kernel_resume_interrupted_user(host_rsp, interrupt_rsp);
    }
}
