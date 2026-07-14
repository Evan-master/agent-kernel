//! x86_64 stack-switch, IRQ preemption, and interrupt-resume assembly.
//!
//! This child module owns only register-save mechanics. Frame offsets come from
//! the portable architecture contract; all semantic validation remains in the
//! parent runtime after IF is clear.

use core::arch::global_asm;

use agent_kernel_x86_64::context::INTERRUPT_RIP_OFFSET;

use crate::pic;

global_asm!(
    r#"
    .section .text.agent_kernel_cpu_context,"ax",@progbits

    .global agent_kernel_context_switch
    .type agent_kernel_context_switch,@function
agent_kernel_context_switch:
    push rbp
    push rbx
    push r12
    push r13
    push r14
    push r15
    mov qword ptr [rdi], rsp
    mov rsp, qword ptr [rsi]
    pop r15
    pop r14
    pop r13
    pop r12
    pop rbx
    pop rbp
    ret
    .size agent_kernel_context_switch, . - agent_kernel_context_switch

    .global agent_kernel_resume_interrupted_agent
    .type agent_kernel_resume_interrupted_agent,@function
agent_kernel_resume_interrupted_agent:
    lea rax, [rip + .Lagent_kernel_resume_host]
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
.Lagent_kernel_resume_host:
    cli
    ret
    .size agent_kernel_resume_interrupted_agent, . - agent_kernel_resume_interrupted_agent

    .global agent_kernel_agent_timer_irq_stub
    .type agent_kernel_agent_timer_irq_stub,@function
agent_kernel_agent_timer_irq_stub:
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

    mov rsp, qword ptr [rip + {host_context}]
    pop r15
    pop r14
    pop r13
    pop r12
    pop rbx
    pop rbp
    ret
    .size agent_kernel_agent_timer_irq_stub, . - agent_kernel_agent_timer_irq_stub
"#,
    interrupt_rsp = sym super::storage::AGENT_KERNEL_AGENT_INTERRUPT_RSP,
    interrupt_rip = sym super::storage::AGENT_KERNEL_AGENT_INTERRUPT_RIP,
    irq_count = sym super::storage::AGENT_KERNEL_AGENT_IRQ_COUNT,
    irq_seen = sym super::storage::AGENT_KERNEL_AGENT_IRQ_SEEN,
    preempted = sym super::storage::AGENT_KERNEL_AGENT_PREEMPTED,
    host_context = sym super::storage::AGENT_KERNEL_HOST_CONTEXT_RSP,
    rip_offset = const INTERRUPT_RIP_OFFSET,
    pic_master_data = const pic::PIC_MASTER_DATA,
    pic_master_command = const pic::PIC_MASTER_COMMAND,
    pic_eoi = const pic::PIC_EOI,
);

unsafe extern "C" {
    fn agent_kernel_context_switch(outgoing_rsp: *mut u64, incoming_rsp: *const u64);
    fn agent_kernel_resume_interrupted_agent(host_rsp: *mut u64, interrupt_rsp: u64);
    pub(super) fn agent_kernel_agent_timer_irq_stub();
}

pub(super) unsafe fn context_switch(outgoing_rsp: *mut u64, incoming_rsp: *const u64) {
    unsafe {
        agent_kernel_context_switch(outgoing_rsp, incoming_rsp);
    }
}

pub(super) unsafe fn resume_interrupted_agent(host_rsp: *mut u64, interrupt_rsp: u64) {
    unsafe {
        agent_kernel_resume_interrupted_agent(host_rsp, interrupt_rsp);
    }
}
