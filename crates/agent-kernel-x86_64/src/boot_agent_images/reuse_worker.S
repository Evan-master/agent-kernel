/* Auditable source for the immutable address-space Reuse Worker Capsule. */
.intel_syntax noprefix
.section .text
.global reuse_worker_entry
reuse_worker_entry:
    push rbx
    pop rbx
    movabs rax, 0x0000400000020000
.wait_for_release:
    cmp byte ptr [rax], 0
    je .wait_for_release

    movabs rax, 0x4c4c4143544e4741
    mov ebx, 1
    mov ecx, 1
    xor edx, edx
    mov esi, 0xe77ce007
    int 0x90
.global reuse_worker_describe_return
reuse_worker_describe_return:

    mov ecx, 28
    xor edx, edx
    xor r10d, r10d
    xor r11d, r11d
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
.global reuse_worker_discovery_return
reuse_worker_discovery_return:
    test rcx, rcx
    jne .halt
    cmp rdx, 28
    jne .halt
    test r10, r10
    je .halt
    test r11, r11
    jne .halt
    test r12, r12
    jne .halt
    test r13, r13
    jne .halt
    test r14, r14
    jne .halt
    test r15, r15
    jne .halt
    test rbp, rbp
    jne .halt
    push r10

    mov ecx, 4
    xor edx, edx
    mov r10d, 0x0e07
    mov r11d, 0xe77c0007
    int 0x90
.global reuse_worker_result_return
reuse_worker_result_return:

    pop r10
    mov ecx, 7
    xor edx, edx
    mov r11d, 1
    mov r12, rdi
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
.global reuse_worker_notification_return
reuse_worker_notification_return:

    mov ecx, 3
    xor edx, edx
    xor r10d, r10d
    xor r11d, r11d
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
.global reuse_worker_completion_return
reuse_worker_completion_return:

.halt:
    jmp .halt

.section .rodata
.global reuse_worker_rodata
reuse_worker_rodata:
    .asciz "AGENT_KERNEL_ROTATED_SIGNER"
