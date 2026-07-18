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
    mov r13d, 51
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

    mov ecx, 12
    xor edx, edx
    mov r10d, 11
    mov r11d, 2
    mov r12d, 1
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
    test rcx, rcx
    jne .halt
    cmp rdx, 12
    jne .halt
    cmp r10, 12
    jne .halt
    test r11, r11
    jne .halt

    mov ecx, 13
    xor edx, edx
    mov r10d, 11
    mov r11d, 12
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
    test rcx, rcx
    jne .halt
    cmp rdx, 13
    jne .halt
    cmp r10, 12
    jne .halt
    cmp r11, 11
    jne .halt

    mov ecx, 11
    xor edx, edx
    mov r10d, 2
    mov r11d, 11
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

    mov ecx, 14
    xor edx, edx
    mov r10d, 10
    mov r11d, 1
    mov r12d, 2
    mov r13d, 1
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
    test rcx, rcx
    jne .halt
    cmp rdx, 14
    jne .halt
    cmp r10, 7
    jne .halt
    test r11, r11
    jne .halt

    mov ecx, 15
    xor edx, edx
    mov r10d, 10
    mov r11d, 7
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
    test rcx, rcx
    jne .halt
    cmp rdx, 15
    jne .halt
    cmp r10, 7
    jne .halt
    test r11, r11
    jne .halt

    mov ecx, 16
    xor edx, edx
    mov r10d, 10
    mov r11d, 7
    mov r12d, 2
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
    test rcx, rcx
    jne .halt
    cmp rdx, 16
    jne .halt
    cmp r10, 7
    jne .halt
    cmp r11, 13
    jne .halt
    cmp r12, 2
    jne .halt

    mov ecx, 17
    xor edx, edx
    mov r10d, 10
    mov r11d, 1
    mov r12d, 9
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
    test rcx, rcx
    jne .halt
    cmp rdx, 17
    jne .halt
    cmp r10, 9
    jne .halt
    cmp r11, 1
    jne .halt
    cmp r12, 1
    jne .halt

    mov ecx, 18
    xor edx, edx
    mov r10d, 10
    mov r11d, 9
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
    test rcx, rcx
    jne .halt
    cmp rdx, 18
    jne .halt
    cmp r10, 9
    jne .halt
    cmp r11, 1
    jne .halt
    cmp r12, 2
    jne .halt

    mov ecx, 19
    xor edx, edx
    mov r10d, 10
    mov r11d, 9
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
    test rcx, rcx
    jne .halt
    cmp rdx, 19
    jne .halt
    cmp r10, 9
    jne .halt
    cmp r11, 1
    jne .halt
    cmp r12, 1
    jne .halt

    mov ecx, 20
    xor edx, edx
    mov r10d, 10
    mov r11d, 9
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
    test rcx, rcx
    jne .halt
    cmp rdx, 20
    jne .halt
    cmp r10, 9
    jne .halt
    cmp r11, 1
    jne .halt
    cmp r12, 3
    jne .halt

    mov ecx, 4
    xor edx, edx
    mov r10d, 0xc004
    movabs r11, 0x000900070007000d
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
