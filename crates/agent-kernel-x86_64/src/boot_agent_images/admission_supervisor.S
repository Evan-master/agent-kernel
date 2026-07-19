/* Auditable source for the immutable Runtime Admission Supervisor Capsule. */
.intel_syntax noprefix
.section .text
.global admission_supervisor_entry
admission_supervisor_entry:
    movabs rax, 0x0000400000001000
.wait_for_release:
    cmp byte ptr [rax], 0
    je .wait_for_release

    movabs rax, 0x4c4c4143544e4741
    mov ebx, 1
    mov ecx, 1
    xor edx, edx
    mov esi, 0xa88ce008
    int 0x90
.global admission_supervisor_describe_return
admission_supervisor_describe_return:

    mov ecx, 27
    xor edx, edx
    mov r10d, 23
    mov r11d, 10
    mov r12d, 8
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
.global admission_supervisor_first_request_return
admission_supervisor_first_request_return:
    test rcx, rcx
    jne .halt
    cmp rdx, 27
    jne .halt
    cmp r10, 1
    jne .halt
    cmp r11, 10
    jne .halt
    cmp r12, 8
    jne .halt

    mov ecx, 27
    xor edx, edx
    mov r10d, 23
    mov r11d, 11
    mov r12d, 9
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
.global admission_supervisor_second_request_return
admission_supervisor_second_request_return:
    test rcx, rcx
    jne .halt
    cmp rdx, 27
    jne .halt
    cmp r10, 2
    jne .halt
    cmp r11, 11
    jne .halt
    cmp r12, 9
    jne .halt

    mov ecx, 8
    xor edx, edx
    xor r10d, r10d
    xor r11d, r11d
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
.global admission_supervisor_first_receive_return
admission_supervisor_first_receive_return:
    test rcx, rcx
    jne .halt
    cmp rdx, 8
    jne .halt
    test r10, r10
    je .halt
    cmp r11, 10
    jne .halt
    cmp r12, 1
    jne .halt
    cmp r13, 8
    jne .halt
    test r14, r14
    jne .halt
    test r15, r15
    jne .halt
    test rbp, rbp
    jne .halt

    mov ecx, 9
    xor edx, edx
    xor r11d, r11d
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
.global admission_supervisor_first_acknowledgement_return
admission_supervisor_first_acknowledgement_return:
    test rcx, rcx
    jne .halt
    cmp rdx, 9
    jne .halt
    test r10, r10
    jne .halt

    mov ecx, 8
    xor edx, edx
    int 0x90
.global admission_supervisor_second_receive_return
admission_supervisor_second_receive_return:
    test rcx, rcx
    jne .halt
    cmp rdx, 8
    jne .halt
    test r10, r10
    je .halt
    cmp r11, 11
    jne .halt
    cmp r12, 1
    jne .halt
    cmp r13, 9
    jne .halt
    test r14, r14
    jne .halt
    test r15, r15
    jne .halt
    test rbp, rbp
    jne .halt

    mov ecx, 9
    xor edx, edx
    xor r11d, r11d
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
.global admission_supervisor_second_acknowledgement_return
admission_supervisor_second_acknowledgement_return:
    test rcx, rcx
    jne .halt
    cmp rdx, 9
    jne .halt
    test r10, r10
    jne .halt

    mov ecx, 27
    xor edx, edx
    mov r10d, 23
    mov r11d, 13
    mov r12d, 11
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
.global admission_supervisor_third_request_return
admission_supervisor_third_request_return:
    test rcx, rcx
    jne .halt
    cmp rdx, 27
    jne .halt
    cmp r10, 3
    jne .halt
    cmp r11, 13
    jne .halt
    cmp r12, 11
    jne .halt

    mov ecx, 27
    xor edx, edx
    mov r10d, 23
    mov r11d, 14
    mov r12d, 12
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
.global admission_supervisor_fourth_request_return
admission_supervisor_fourth_request_return:
    test rcx, rcx
    jne .halt
    cmp rdx, 27
    jne .halt
    cmp r10, 4
    jne .halt
    cmp r11, 14
    jne .halt
    cmp r12, 12
    jne .halt

    mov ecx, 8
    xor edx, edx
    xor r10d, r10d
    xor r11d, r11d
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
.global admission_supervisor_third_receive_return
admission_supervisor_third_receive_return:
    test rcx, rcx
    jne .halt
    cmp rdx, 8
    jne .halt
    test r10, r10
    je .halt
    cmp r11, 13
    jne .halt
    cmp r12, 1
    jne .halt
    cmp r13, 11
    jne .halt
    test r14, r14
    jne .halt
    test r15, r15
    jne .halt
    test rbp, rbp
    jne .halt

    mov ecx, 9
    xor edx, edx
    xor r11d, r11d
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
.global admission_supervisor_third_acknowledgement_return
admission_supervisor_third_acknowledgement_return:
    test rcx, rcx
    jne .halt
    cmp rdx, 9
    jne .halt
    test r10, r10
    jne .halt

    mov ecx, 8
    xor edx, edx
    int 0x90
.global admission_supervisor_fourth_receive_return
admission_supervisor_fourth_receive_return:
    test rcx, rcx
    jne .halt
    cmp rdx, 8
    jne .halt
    test r10, r10
    je .halt
    cmp r11, 14
    jne .halt
    cmp r12, 1
    jne .halt
    cmp r13, 12
    jne .halt
    test r14, r14
    jne .halt
    test r15, r15
    jne .halt
    test rbp, rbp
    jne .halt

    mov ecx, 9
    xor edx, edx
    xor r11d, r11d
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
.global admission_supervisor_fourth_acknowledgement_return
admission_supervisor_fourth_acknowledgement_return:
    test rcx, rcx
    jne .halt
    cmp rdx, 9
    jne .halt
    test r10, r10
    jne .halt

    mov ecx, 29
    xor edx, edx
    mov r10d, 23
    mov r11d, 2
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
.global admission_supervisor_compaction_return
admission_supervisor_compaction_return:
    test rcx, rcx
    jne .halt
    cmp rdx, 29
    jne .halt
    cmp r10, 1
    jne .halt
    cmp r11, 2
    jne .halt
    cmp r12, 2
    jne .halt
    test r13, r13
    jne .halt
    test r14, r14
    jne .halt
    test r15, r15
    jne .halt
    test rbp, rbp
    jne .halt

    mov ecx, 4
    xor edx, edx
    mov r10d, 0x0a08
    mov r11d, 0xa88c0008
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
.global admission_supervisor_result_return
admission_supervisor_result_return:

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
.global admission_supervisor_completion_return
admission_supervisor_completion_return:

.halt:
    jmp .halt
