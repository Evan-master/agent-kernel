/* Auditable source for the native Network Driver Capsule. */
.intel_syntax noprefix
.section .text
.global network_driver_entry
network_driver_entry:
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
    mov esi, 0xd81ce030
    int 0x90
.global network_driver_describe_return
network_driver_describe_return:
    cmp r10, 1
    jne .halt

    mov ecx, 57
    xor edx, edx
    xor r10d, r10d
    xor r11d, r11d
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
.global network_driver_inspect_return
network_driver_inspect_return:
    test r10, r10
    je .halt
    test r11, r11
    je .halt
    test r12, r12
    je .halt
    test rbp, rbp
    jne .halt
    cmp r13, 4
    jne .halt

    cmp r14, 0x3001
    je .neighbor
    cmp r14, 0x3002
    je .udp
    jmp .halt

.neighbor:
    test r15, r15
    jne .halt
    jmp .remember_event

.udp:
    cmp r15, 13
    jne .halt

.remember_event:
    push r15
    push r14
    push r10
    movabs rax, 0x4c4c4143544e4741

    mov ecx, 58
    xor edx, edx
    xor r11d, r11d
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
.global network_driver_acknowledge_return
network_driver_acknowledge_return:
    cmp r10, qword ptr [rsp]
    jne .halt

    mov ecx, 59
    xor edx, edx
    mov r10, qword ptr [rsp]
    mov r12, qword ptr [rsp + 8]
    mov r13, qword ptr [rsp + 16]
    cmp r12, 0x3001
    je .configure
    mov r11d, 1
    jmp .submit

.configure:
    mov r11d, 2

.submit:
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
.global network_driver_submit_return
network_driver_submit_return:
    test r10, r10
    je .halt
    test r11, r11
    jne .halt
    cmp r12, qword ptr [rsp + 16]
    jne .halt
    test r13, r13
    jne .halt
    test r14, r14
    jne .halt
    test r15, r15
    jne .halt
    test rbp, rbp
    jne .halt
    add rsp, 24

    mov ecx, 60
    xor edx, edx
    xor r10d, r10d
    xor r11d, r11d
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
.global network_driver_completion_return
network_driver_completion_return:

.halt:
    jmp .halt
