/* Auditable source for the native PCI serial Driver Capsule. */
.intel_syntax noprefix
.section .text
.global pci_serial_driver_entry
pci_serial_driver_entry:
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
    mov esi, 0xd81ce024
    int 0x90
.global pci_serial_driver_describe_return
pci_serial_driver_describe_return:
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
.global pci_serial_driver_inspect_return
pci_serial_driver_inspect_return:
    test r10, r10
    je .halt
    test r11, r11
    je .halt
    test r12, r12
    je .halt
    test rbp, rbp
    jne .halt

    cmp r13, 4
    je .state_changed
    cmp r13, 1
    je .interrupt
    jmp .halt

.state_changed:
    cmp r14, 0x1b36
    jne .halt
    cmp r15, 2
    jne .halt
    movabs rax, 0x0000400000020002
    cmp byte ptr [rax], 0
    je .fault_generation_zero
    cmp byte ptr [rax], 1
    jne .halt
    jmp .remember_event

.fault_generation_zero:
.global pci_serial_driver_fault_site
pci_serial_driver_fault_site:
    ud2

.interrupt:
    cmp r14, 2
    jne .halt
    test r15, 0x20
    je .halt

.remember_event:
    movabs rax, 0x4c4c4143544e4741
    push r13
    push r10

    mov ecx, 58
    xor edx, edx
    xor r11d, r11d
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
.global pci_serial_driver_acknowledge_return
pci_serial_driver_acknowledge_return:
    cmp r10, qword ptr [rsp]
    jne .halt

    mov ecx, 59
    xor edx, edx
    cmp qword ptr [rsp + 8], 4
    je .configure

.write:
    mov r11d, 1
    xor r12d, r12d
    mov r13d, 0x50
    jmp .submit

.configure:
    mov r11d, 2
    mov r12d, 1
    xor r13d, r13d

.submit:
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
.global pci_serial_driver_submit_return
pci_serial_driver_submit_return:
    test r10, r10
    je .halt
    test r11, r11
    jne .halt
    cmp qword ptr [rsp + 8], 4
    je .configure_result
    cmp r12, 0x50
    jne .halt
    jmp .result_checked

.configure_result:
    test r12, r12
    jne .halt

.result_checked:
    test r13, r13
    jne .halt
    test r14, r14
    jne .halt
    test r15, r15
    jne .halt
    test rbp, rbp
    jne .halt
    add rsp, 16

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
.global pci_serial_driver_completion_return
pci_serial_driver_completion_return:

.halt:
    jmp .halt
