/* Auditable source for the immutable kind-4 Resource Manager Capsule. */
.intel_syntax noprefix
.section .text
.global resource_manager_entry
resource_manager_entry:
    movabs rax, 0x0000400000001000
.wait_for_quantum:
    cmp byte ptr [rax + 1], 1
    jb .wait_for_quantum

    movabs rax, 0x4c4c4143544e4741
    mov ebx, 1
    mov ecx, 1
    xor edx, edx
    mov esi, 0xf66ce006
    int 0x90

    mov ecx, 10
    xor edx, edx
    mov r10d, 10
    mov r11d, 1
    mov r12d, 3
    mov r13d, 19
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
    test rcx, rcx
    jne .halt
    cmp rdx, 10
    jne .halt
    cmp r10, 2
    jne .halt
    cmp r11, 11
    jne .halt

    mov ecx, 11
    xor edx, edx
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
    test rcx, rcx
    jne .halt
    cmp rdx, 11
    jne .halt
    cmp r10, 2
    jne .halt
    cmp r11, 11
    jne .halt

    mov ecx, 4
    xor edx, edx
    mov r10d, 0xc001
    movabs r11, 0x000000020000000b
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90

    mov ecx, 3
    xor edx, edx
    int 0x90
.halt:
    jmp .halt
