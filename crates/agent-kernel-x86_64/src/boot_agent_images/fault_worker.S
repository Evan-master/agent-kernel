/* Auditable source for the immutable kind-1 Fault Worker Capsule. */
.intel_syntax noprefix
.section .text
.global fault_worker_entry
fault_worker_entry:
    movabs rax, 0x0000400000004000
.wait_for_quantum:
    cmp byte ptr [rax + 1], 1
    jb .wait_for_quantum

    cmp byte ptr [rax + 2], 0
    je .live_memory_fault
    cmp byte ptr [rax + 2], 1
    je .general_protection_fault
    cmp byte ptr [rax + 2], 2
    je .write_protection_fault
    cmp byte ptr [rax + 2], 3
    je .lazy_page_fault
    jmp .halt

.live_memory_fault:
    xor edi, edi
    movabs rax, 0x4c4c4143544e4741
    mov ebx, 1
    mov ecx, 1
    xor edx, edx
    mov esi, 0xd44ce004
    int 0x90

    mov ecx, 24
    xor edx, edx
    mov r10d, 8
    mov r11d, 2
    mov r12d, 2
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
    test rcx, rcx
    jne .halt
    cmp rdx, 24
    jne .halt
    cmp r10, 1
    jne .halt
    movabs r15, 0x000040000000c000
    cmp r11, r15
    jne .halt
    cmp r12, 8192
    jne .halt
    cmp r13, 2
    jne .halt
    cmp r14, 1
    jne .halt

    movabs r15, 0x4641554c544d3031
    mov qword ptr [r11], r15
    movabs r15, 0x4641554c544d3032
    mov qword ptr [r11 + 4096], r15
    ud2

.general_protection_fault:
    cli
    jmp .halt

.write_protection_fault:
    mov byte ptr [rax], 1
    jmp .halt

.lazy_page_fault:
    movabs rdi, 0x000040000000a000
    mov byte ptr [rdi], 0x5a
    cmp byte ptr [rdi], 0x5a
    jne .halt
    jmp .complete

.complete:
    xor edi, edi
    movabs rax, 0x4c4c4143544e4741
    mov ebx, 1
    mov ecx, 1
    xor edx, edx
    mov esi, 0xd44ce004
    int 0x90
    mov ecx, 3
    xor edx, edx
    int 0x90

.halt:
    jmp .halt
