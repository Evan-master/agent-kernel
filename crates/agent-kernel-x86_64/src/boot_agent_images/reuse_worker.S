/* Auditable source for the immutable address-space Reuse Worker Capsule. */
.intel_syntax noprefix
.section .text
.global reuse_worker_entry
reuse_worker_entry:
    push rbx
    pop rbx
    movabs rax, 0x0000400000001000
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

    mov ecx, 4
    xor edx, edx
    mov r10d, 0x0e07
    mov r11d, 0xe77c0007
    int 0x90
.global reuse_worker_result_return
reuse_worker_result_return:

    mov ecx, 3
    xor edx, edx
    int 0x90
.global reuse_worker_completion_return
reuse_worker_completion_return:

.halt:
    jmp .halt
