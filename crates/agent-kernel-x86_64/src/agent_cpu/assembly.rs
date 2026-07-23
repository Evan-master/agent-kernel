//! Ring-3 entry, resume, PIT, Agent-call, and contained-exception assembly.
//!
//! This child module owns only register and privilege-frame mechanics. It never
//! calls Rust while a user frame is active; the parent validates all recorded
//! evidence after control returns to CPL0.

use core::arch::global_asm;

use agent_kernel_x86_64::{
    context::{
        PRIVILEGE_ERROR_CODE_OFFSET, PRIVILEGE_ERROR_CODE_RIP_OFFSET,
        PRIVILEGE_INTERRUPT_RIP_OFFSET,
    },
    native_runtime::{GENERAL_PROTECTION_VECTOR, INVALID_OPCODE_VECTOR, PAGE_FAULT_VECTOR},
    per_cpu::{
        PER_CPU_CALL_COUNT_OFFSET, PER_CPU_CALL_CR3_OFFSET, PER_CPU_CALL_RIP_OFFSET,
        PER_CPU_CALL_RSP_OFFSET, PER_CPU_CALL_SEEN_OFFSET, PER_CPU_FAULT_ADDRESS_OFFSET,
        PER_CPU_FAULT_COUNT_OFFSET, PER_CPU_FAULT_CR3_OFFSET, PER_CPU_FAULT_ERROR_CODE_OFFSET,
        PER_CPU_FAULT_RIP_OFFSET, PER_CPU_FAULT_RSP_OFFSET, PER_CPU_FAULT_SEEN_OFFSET,
        PER_CPU_FAULT_VECTOR_OFFSET, PER_CPU_HOST_RSP_OFFSET, PER_CPU_INTERRUPT_CR3_OFFSET,
        PER_CPU_INTERRUPT_RIP_OFFSET, PER_CPU_INTERRUPT_RSP_OFFSET, PER_CPU_IRQ_COUNT_OFFSET,
        PER_CPU_IRQ_SEEN_OFFSET, PER_CPU_KERNEL_CR3_OFFSET, PER_CPU_PREEMPTED_OFFSET,
    },
};

use crate::pic;

const EXCEPTION_ORIGIN_CS_OFFSET_AFTER_RAX: usize = 16;
const ERROR_CODE_ORIGIN_CS_OFFSET_AFTER_RAX: usize = 24;

global_asm!(
    r#"
    .section .text.agent_kernel_cpu_context,"ax",@progbits
    .macro agent_kernel_push_integer_frame
    push rax
    agent_kernel_push_integer_frame_after_rax
    .endm
    .macro agent_kernel_push_integer_frame_after_rax
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
    .macro agent_kernel_pop_integer_frame
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
    .endm
    .macro agent_kernel_restore_host
    mov rsp, qword ptr gs:[{host_context_offset}]
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
    mov cr3, r9
    xor eax, eax
    xor ebx, ebx
    xor ecx, ecx
    xor edx, edx
    xor esi, esi
    xor edi, edi
    xor ebp, ebp
    xor r8d, r8d
    xor r9d, r9d
    xor r10d, r10d
    xor r11d, r11d
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
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
    mov cr3, rdx
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
    mov r10, cr3
    mov rax, qword ptr gs:[{kernel_cr3_offset}]
    mov cr3, rax
    mov qword ptr gs:[{interrupt_cr3_offset}], r10
    mov qword ptr gs:[{interrupt_rsp_offset}], rsp
    mov rax, qword ptr [rsp + {rip_offset}]
    mov qword ptr gs:[{interrupt_rip_offset}], rax
    inc byte ptr gs:[{irq_count_offset}]
    mov dx, {pic_master_data}
    mov al, 0xff
    out dx, al
    mov dx, {pic_master_command}
    mov al, {pic_eoi}
    out dx, al
    mov byte ptr gs:[{irq_seen_offset}], 1
    mov byte ptr gs:[{preempted_offset}], 1
    agent_kernel_restore_host
    .size agent_kernel_agent_timer_irq_stub, . - agent_kernel_agent_timer_irq_stub
    .global agent_kernel_agent_apic_timer_stub
    .type agent_kernel_agent_apic_timer_stub,@function
agent_kernel_agent_apic_timer_stub:
    push rax
    mov rax, qword ptr [rsp + {origin_cs_offset}]
    and eax, 3
    cmp eax, 3
    jne .Lagent_kernel_apic_timer_kernel
    agent_kernel_push_integer_frame_after_rax
    mov r10, cr3
    mov rax, qword ptr gs:[{kernel_cr3_offset}]
    mov cr3, rax
    mov qword ptr gs:[{interrupt_cr3_offset}], r10
    mov qword ptr gs:[{interrupt_rsp_offset}], rsp
    mov rax, qword ptr [rsp + {rip_offset}]
    mov qword ptr gs:[{interrupt_rip_offset}], rax
    inc byte ptr gs:[{irq_count_offset}]
    mov byte ptr gs:[{irq_seen_offset}], 1
    mov byte ptr gs:[{preempted_offset}], 1
    agent_kernel_restore_host
.Lagent_kernel_apic_timer_kernel:
    agent_kernel_push_integer_frame_after_rax
    mov r12, rsp
    and rsp, -16
    call agent_kernel_local_apic_eoi
    mov rsp, r12
    agent_kernel_pop_integer_frame
    iretq
    .size agent_kernel_agent_apic_timer_stub, . - agent_kernel_agent_apic_timer_stub
    .global agent_kernel_agent_call_stub
    .type agent_kernel_agent_call_stub,@function
agent_kernel_agent_call_stub:
    agent_kernel_push_integer_frame
    mov r10, cr3
    mov rax, qword ptr gs:[{kernel_cr3_offset}]
    mov cr3, rax
    mov qword ptr gs:[{call_cr3_offset}], r10
    mov qword ptr gs:[{call_rsp_offset}], rsp
    mov rax, qword ptr [rsp + {rip_offset}]
    mov qword ptr gs:[{call_rip_offset}], rax
    inc byte ptr gs:[{call_count_offset}]
    mov byte ptr gs:[{call_seen_offset}], 1
    agent_kernel_restore_host
    .size agent_kernel_agent_call_stub, . - agent_kernel_agent_call_stub
    .global agent_kernel_agent_invalid_opcode_stub
    .type agent_kernel_agent_invalid_opcode_stub,@function
agent_kernel_agent_invalid_opcode_stub:
    push rax
    mov rax, qword ptr [rsp + {origin_cs_offset}]
    and eax, 3
    cmp eax, 3
    jne .Lagent_kernel_invalid_opcode_fatal
    agent_kernel_push_integer_frame_after_rax
    mov r10, cr3
    mov rax, qword ptr gs:[{kernel_cr3_offset}]
    mov cr3, rax
    mov qword ptr gs:[{fault_cr3_offset}], r10
    mov qword ptr gs:[{fault_rsp_offset}], rsp
    mov rax, qword ptr [rsp + {rip_offset}]
    mov qword ptr gs:[{fault_rip_offset}], rax
    mov byte ptr gs:[{fault_vector_offset}], {invalid_opcode_vector}
    inc byte ptr gs:[{fault_count_offset}]
    mov byte ptr gs:[{fault_seen_offset}], 1
    agent_kernel_restore_host
.Lagent_kernel_invalid_opcode_fatal:
    pop rax
    jmp agent_kernel_exception_6
    .size agent_kernel_agent_invalid_opcode_stub, . - agent_kernel_agent_invalid_opcode_stub
    .global agent_kernel_agent_general_protection_stub
    .type agent_kernel_agent_general_protection_stub,@function
agent_kernel_agent_general_protection_stub:
    push rax
    mov rax, qword ptr [rsp + {error_origin_cs_offset}]
    and eax, 3
    cmp eax, 3
    jne .Lagent_kernel_general_protection_fatal
    agent_kernel_push_integer_frame_after_rax
    mov r10, cr3
    mov rax, qword ptr gs:[{kernel_cr3_offset}]
    mov cr3, rax
    mov qword ptr gs:[{fault_cr3_offset}], r10
    mov qword ptr gs:[{fault_rsp_offset}], rsp
    mov rax, qword ptr [rsp + {error_code_offset}]
    mov qword ptr gs:[{fault_error_code_offset}], rax
    mov rax, qword ptr [rsp + {error_rip_offset}]
    mov qword ptr gs:[{fault_rip_offset}], rax
    mov byte ptr gs:[{fault_vector_offset}], {general_protection_vector}
    inc byte ptr gs:[{fault_count_offset}]
    mov byte ptr gs:[{fault_seen_offset}], 1
    agent_kernel_restore_host
.Lagent_kernel_general_protection_fatal:
    pop rax
    jmp agent_kernel_exception_13
    .size agent_kernel_agent_general_protection_stub, . - agent_kernel_agent_general_protection_stub
    .global agent_kernel_agent_page_fault_stub
    .type agent_kernel_agent_page_fault_stub,@function
agent_kernel_agent_page_fault_stub:
    push rax
    mov rax, qword ptr [rsp + {error_origin_cs_offset}]
    and eax, 3
    cmp eax, 3
    jne .Lagent_kernel_page_fault_fatal
    agent_kernel_push_integer_frame_after_rax
    mov r11, cr2
    mov r10, cr3
    mov rax, qword ptr gs:[{kernel_cr3_offset}]
    mov cr3, rax
    mov qword ptr gs:[{fault_address_offset}], r11
    mov qword ptr gs:[{fault_cr3_offset}], r10
    mov qword ptr gs:[{fault_rsp_offset}], rsp
    mov rax, qword ptr [rsp + {error_code_offset}]
    mov qword ptr gs:[{fault_error_code_offset}], rax
    mov rax, qword ptr [rsp + {error_rip_offset}]
    mov qword ptr gs:[{fault_rip_offset}], rax
    mov byte ptr gs:[{fault_vector_offset}], {page_fault_vector}
    inc byte ptr gs:[{fault_count_offset}]
    mov byte ptr gs:[{fault_seen_offset}], 1
    agent_kernel_restore_host
.Lagent_kernel_page_fault_fatal:
    pop rax
    jmp agent_kernel_exception_14
    .size agent_kernel_agent_page_fault_stub, . - agent_kernel_agent_page_fault_stub
"#,
    host_context_offset = const PER_CPU_HOST_RSP_OFFSET,
    kernel_cr3_offset = const PER_CPU_KERNEL_CR3_OFFSET,
    interrupt_rsp_offset = const PER_CPU_INTERRUPT_RSP_OFFSET,
    interrupt_rip_offset = const PER_CPU_INTERRUPT_RIP_OFFSET,
    interrupt_cr3_offset = const PER_CPU_INTERRUPT_CR3_OFFSET,
    irq_count_offset = const PER_CPU_IRQ_COUNT_OFFSET,
    irq_seen_offset = const PER_CPU_IRQ_SEEN_OFFSET,
    preempted_offset = const PER_CPU_PREEMPTED_OFFSET,
    call_rsp_offset = const PER_CPU_CALL_RSP_OFFSET,
    call_rip_offset = const PER_CPU_CALL_RIP_OFFSET,
    call_cr3_offset = const PER_CPU_CALL_CR3_OFFSET,
    call_count_offset = const PER_CPU_CALL_COUNT_OFFSET,
    call_seen_offset = const PER_CPU_CALL_SEEN_OFFSET,
    fault_rsp_offset = const PER_CPU_FAULT_RSP_OFFSET,
    fault_rip_offset = const PER_CPU_FAULT_RIP_OFFSET,
    fault_cr3_offset = const PER_CPU_FAULT_CR3_OFFSET,
    fault_error_code_offset = const PER_CPU_FAULT_ERROR_CODE_OFFSET,
    fault_address_offset = const PER_CPU_FAULT_ADDRESS_OFFSET,
    fault_count_offset = const PER_CPU_FAULT_COUNT_OFFSET,
    fault_seen_offset = const PER_CPU_FAULT_SEEN_OFFSET,
    fault_vector_offset = const PER_CPU_FAULT_VECTOR_OFFSET,
    rip_offset = const PRIVILEGE_INTERRUPT_RIP_OFFSET,
    origin_cs_offset = const EXCEPTION_ORIGIN_CS_OFFSET_AFTER_RAX,
    error_origin_cs_offset = const ERROR_CODE_ORIGIN_CS_OFFSET_AFTER_RAX,
    error_code_offset = const PRIVILEGE_ERROR_CODE_OFFSET,
    error_rip_offset = const PRIVILEGE_ERROR_CODE_RIP_OFFSET,
    invalid_opcode_vector = const INVALID_OPCODE_VECTOR,
    general_protection_vector = const GENERAL_PROTECTION_VECTOR,
    page_fault_vector = const PAGE_FAULT_VECTOR,
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
        agent_cr3: u64,
    );
    fn agent_kernel_resume_interrupted_user(host_rsp: *mut u64, interrupt_rsp: u64, agent_cr3: u64);
    pub(super) fn agent_kernel_agent_timer_irq_stub();
    pub(super) fn agent_kernel_agent_apic_timer_stub();
    pub(super) fn agent_kernel_agent_call_stub();
    pub(super) fn agent_kernel_agent_invalid_opcode_stub();
    pub(super) fn agent_kernel_agent_general_protection_stub();
    pub(super) fn agent_kernel_agent_page_fault_stub();
}

pub(super) unsafe fn enter_user(
    host_rsp: *mut u64,
    user_rip: u64,
    user_rsp: u64,
    user_cs: u16,
    user_ss: u16,
    agent_cr3: u64,
) {
    unsafe {
        agent_kernel_enter_user(
            host_rsp,
            user_rip,
            user_rsp,
            u64::from(user_cs),
            u64::from(user_ss),
            agent_cr3,
        );
    }
}

pub(super) unsafe fn resume_interrupted_user(
    host_rsp: *mut u64,
    interrupt_rsp: u64,
    agent_cr3: u64,
) {
    unsafe {
        agent_kernel_resume_interrupted_user(host_rsp, interrupt_rsp, agent_cr3);
    }
}
