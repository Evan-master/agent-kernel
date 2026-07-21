/* Auditable source for the immutable Runtime Admission Supervisor Capsule. */
.intel_syntax noprefix
.section .text
.global admission_supervisor_entry
admission_supervisor_entry:
    movabs rax, 0x0000400000020000
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
    mov r10d, 25
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
    mov r10d, 25
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
    cmp r10, 4
    jne .halt
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

    push r10
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

    pop r10
    mov ecx, 34
    xor edx, edx
    int 0x90
.global admission_supervisor_first_message_retirement_return
admission_supervisor_first_message_retirement_return:
    test rcx, rcx
    jne .halt
    cmp rdx, 34
    jne .halt
    cmp r10, 4
    jne .halt
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

    mov ecx, 8
    xor edx, edx
    xor r10d, r10d
    int 0x90
.global admission_supervisor_second_receive_return
admission_supervisor_second_receive_return:
    test rcx, rcx
    jne .halt
    cmp rdx, 8
    jne .halt
    cmp r10, 5
    jne .halt
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

    push r10
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

    pop r10
    mov ecx, 34
    xor edx, edx
    int 0x90
.global admission_supervisor_second_message_retirement_return
admission_supervisor_second_message_retirement_return:
    test rcx, rcx
    jne .halt
    cmp rdx, 34
    jne .halt
    cmp r10, 5
    jne .halt
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

    mov ecx, 38
    xor edx, edx
    mov r10d, 25
    mov r11d, 3
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
.global admission_supervisor_first_waiter_compaction_return
admission_supervisor_first_waiter_compaction_return:
    test rcx, rcx
    jne .halt
    cmp rdx, 38
    jne .halt
    cmp r10, 1
    jne .halt
    cmp r11, 3
    jne .halt
    cmp r12, 3
    jne .halt
    test r13, r13
    jne .halt
    test r14, r14
    jne .halt
    test r15, r15
    jne .halt
    test rbp, rbp
    jne .halt

    mov ecx, 27
    xor edx, edx
    mov r10d, 25
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
    mov r10d, 25
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
    cmp r10, 6
    jne .halt
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

    push r10
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

    pop r10
    mov ecx, 34
    xor edx, edx
    int 0x90
.global admission_supervisor_third_message_retirement_return
admission_supervisor_third_message_retirement_return:
    test rcx, rcx
    jne .halt
    cmp rdx, 34
    jne .halt
    cmp r10, 6
    jne .halt
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

    mov ecx, 8
    xor edx, edx
    xor r10d, r10d
    int 0x90
.global admission_supervisor_fourth_receive_return
admission_supervisor_fourth_receive_return:
    test rcx, rcx
    jne .halt
    cmp rdx, 8
    jne .halt
    cmp r10, 7
    jne .halt
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

    push r10
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

    pop r10
    mov ecx, 34
    xor edx, edx
    int 0x90
.global admission_supervisor_fourth_message_retirement_return
admission_supervisor_fourth_message_retirement_return:
    test rcx, rcx
    jne .halt
    cmp rdx, 34
    jne .halt
    cmp r10, 7
    jne .halt
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

    mov ecx, 38
    xor edx, edx
    mov r10d, 25
    mov r11d, 4
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
.global admission_supervisor_second_waiter_compaction_return
admission_supervisor_second_waiter_compaction_return:
    test rcx, rcx
    jne .halt
    cmp rdx, 38
    jne .halt
    cmp r10, 4
    jne .halt
    cmp r11, 4
    jne .halt
    cmp r12, 1
    jne .halt
    test r13, r13
    jne .halt
    test r14, r14
    jne .halt
    test r15, r15
    jne .halt
    test rbp, rbp
    jne .halt

    mov ecx, 29
    xor edx, edx
    mov r10d, 25
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

    mov ecx, 30
    xor edx, edx
    mov r10d, 25
    mov r11d, 6
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
.global admission_supervisor_task_compaction_return
admission_supervisor_task_compaction_return:
    test rcx, rcx
    jne .halt
    cmp rdx, 30
    jne .halt
    cmp r10, 1
    jne .halt
    cmp r11, 6
    jne .halt
    cmp r12, 6
    jne .halt
    test r13, r13
    jne .halt
    test r14, r14
    jne .halt
    test r15, r15
    jne .halt
    test rbp, rbp
    jne .halt

    mov ecx, 39
    xor edx, edx
    mov r10d, 25
    mov r11d, 4
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
.global admission_supervisor_fault_compaction_return
admission_supervisor_fault_compaction_return:
    test rcx, rcx
    jne .halt
    cmp rdx, 39
    jne .halt
    cmp r10, 1
    jne .halt
    cmp r11, 4
    jne .halt
    cmp r12, 4
    jne .halt
    test r13, r13
    jne .halt
    test r14, r14
    jne .halt
    test r15, r15
    jne .halt
    test rbp, rbp
    jne .halt

    mov ecx, 31
    xor edx, edx
    mov r10d, 25
    mov r11d, 6
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
.global admission_supervisor_intent_compaction_return
admission_supervisor_intent_compaction_return:
    test rcx, rcx
    jne .halt
    cmp rdx, 31
    jne .halt
    cmp r10, 1
    jne .halt
    cmp r11, 6
    jne .halt
    cmp r12, 6
    jne .halt
    test r13, r13
    jne .halt
    test r14, r14
    jne .halt
    test r15, r15
    jne .halt
    test rbp, rbp
    jne .halt

    mov ecx, 33
    xor edx, edx
    mov r10d, 25
    mov r11d, 10
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
.global admission_supervisor_first_agent_entry_retirement_return
admission_supervisor_first_agent_entry_retirement_return:
    test rcx, rcx
    jne .halt
    cmp rdx, 33
    jne .halt
    cmp r10, 10
    jne .halt
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

    mov ecx, 33
    xor edx, edx
    mov r10d, 25
    mov r11d, 11
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
.global admission_supervisor_second_agent_entry_retirement_return
admission_supervisor_second_agent_entry_retirement_return:
    test rcx, rcx
    jne .halt
    cmp rdx, 33
    jne .halt
    cmp r10, 11
    jne .halt
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

    mov ecx, 12
    xor edx, edx
    mov r10d, 25
    mov r11d, 12
    mov r12d, 16
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
.global admission_supervisor_capability_derive_return
admission_supervisor_capability_derive_return:
    test rcx, rcx
    jne .halt
    cmp rdx, 12
    jne .halt
    cmp r10, 28
    jne .halt
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

    mov ecx, 13
    xor edx, edx
    mov r10d, 25
    mov r11d, 28
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
.global admission_supervisor_capability_revoke_return
admission_supervisor_capability_revoke_return:
    test rcx, rcx
    jne .halt
    cmp rdx, 13
    jne .halt
    cmp r10, 28
    jne .halt
    cmp r11, 25
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

    mov ecx, 32
    xor edx, edx
    mov r10d, 25
    mov r11d, 28
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
.global admission_supervisor_first_transient_capability_compaction_return
admission_supervisor_first_transient_capability_compaction_return:
    test rcx, rcx
    jne .halt
    cmp rdx, 32
    jne .halt
    cmp r10, 28
    jne .halt
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

    mov ecx, 12
    xor edx, edx
    mov r10d, 25
    mov r11d, 12
    mov r12d, 16
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
.global admission_supervisor_second_transient_capability_derive_return
admission_supervisor_second_transient_capability_derive_return:
    test rcx, rcx
    jne .halt
    cmp rdx, 12
    jne .halt
    cmp r10, 29
    jne .halt
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

    mov ecx, 13
    xor edx, edx
    mov r10d, 25
    mov r11d, 29
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
.global admission_supervisor_second_transient_capability_revoke_return
admission_supervisor_second_transient_capability_revoke_return:
    test rcx, rcx
    jne .halt
    cmp rdx, 13
    jne .halt
    cmp r10, 29
    jne .halt
    cmp r11, 25
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

    mov ecx, 32
    xor edx, edx
    mov r10d, 25
    mov r11d, 29
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
.global admission_supervisor_third_capability_compaction_return
admission_supervisor_third_capability_compaction_return:
    test rcx, rcx
    jne .halt
    cmp rdx, 32
    jne .halt
    cmp r10, 29
    jne .halt
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

    mov ecx, 40
    xor edx, edx
    mov r10d, 25
    mov r11d, 64
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
.global admission_supervisor_event_archive_return
admission_supervisor_event_archive_return:
    test rcx, rcx
    jne .halt
    cmp rdx, 40
    jne .halt
    cmp r10, 1
    jne .halt
    cmp r11, 64
    jne .halt
    cmp r12, 64
    jne .halt
    movabs r12, 0xec72ca2f18dfdf64
    cmp r13, r12
    jne .halt
    movabs r12, 0xe1e332af9e880408
    cmp r14, r12
    jne .halt
    movabs r12, 0xa6c5477a0f27d6ae
    cmp r15, r12
    jne .halt
    movabs r12, 0x50c4bbbdff067ca1
    cmp rbp, r12
    jne .halt

    mov ecx, 42
    xor edx, edx
    mov r10d, 25
    mov r11d, 16
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
.global admission_supervisor_memory_capability_early_cleanup_return
admission_supervisor_memory_capability_early_cleanup_return:
    test rcx, rcx
    jne .halt
    cmp rdx, 42
    jne .halt
    cmp r10, 16
    jne .halt
    cmp r11, 4
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

    mov ecx, 10
    xor edx, edx
    mov r10d, 25
    mov r11d, 1
    mov r12d, 3
    mov r13d, 1
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
.global admission_supervisor_resource_reuse_return
admission_supervisor_resource_reuse_return:
    test rcx, rcx
    jne .halt
    cmp rdx, 10
    jne .halt
    cmp r10, 10
    jne .halt
    cmp r11, 30
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

    mov ecx, 12
    xor edx, edx
    mov r10d, 25
    mov r11d, 12
    mov r12d, 32
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
.global admission_supervisor_first_capability_reuse_return
admission_supervisor_first_capability_reuse_return:
    test rcx, rcx
    jne .halt
    cmp rdx, 12
    jne .halt
    cmp r10, 31
    jne .halt
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

    mov ecx, 12
    xor edx, edx
    mov r10d, 25
    mov r11d, 12
    mov r12d, 16
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
.global admission_supervisor_second_capability_reuse_return
admission_supervisor_second_capability_reuse_return:
    test rcx, rcx
    jne .halt
    cmp rdx, 12
    jne .halt
    cmp r10, 32
    jne .halt

    mov cl, 43
    xor edx, edx
    mov r10d, 25
    mov r11d, 2
    int 0x90
.global admission_supervisor_memory_cell_record_retirement_return
admission_supervisor_memory_cell_record_retirement_return:
    jmp .memory_cell_retirement_check
.memory_cell_retirement_fail:
    jmp .halt
.memory_cell_retirement_check:
    test rcx, rcx
    jne .memory_cell_retirement_fail
    cmp rdx, 43
    jne .memory_cell_retirement_fail
    cmp r10, 2
    jne .memory_cell_retirement_fail
    cmp r11, 4
    jne .memory_cell_retirement_fail
    cmp r12, 1
    jne .memory_cell_retirement_fail

    mov cl, 42
    xor edx, edx
    mov r10d, 25
    mov r11d, 17
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
.global admission_supervisor_memory_capability_cleanup_return
admission_supervisor_memory_capability_cleanup_return:
    jmp .memory_capability_cleanup_check
.memory_capability_cleanup_fail:
    jmp .halt
.memory_capability_cleanup_check:
    test rcx, rcx
    jne .memory_capability_cleanup_fail
    cmp rdx, 42
    jne .memory_capability_cleanup_fail
    cmp r10, 17
    jne .memory_capability_cleanup_fail
    cmp r11, 5
    jne .memory_capability_cleanup_fail

    mov cl, 32
    xor edx, edx
    mov r10d, 25
    mov r11d, 16
    int 0x90
.global admission_supervisor_memory_capability_compaction_return
admission_supervisor_memory_capability_compaction_return:
    jmp .memory_capability_compaction_check
.memory_capability_compaction_fail:
    jmp .halt
.memory_capability_compaction_check:
    test rcx, rcx
    jne .memory_capability_compaction_fail
    cmp rdx, 32
    jne .memory_capability_compaction_fail
    cmp r10, 16
    jne .memory_capability_compaction_fail

    mov cl, 41
    xor edx, edx
    mov r10d, 25
    mov r11d, 4
    int 0x90
.global admission_supervisor_memory_resource_record_retirement_return
admission_supervisor_memory_resource_record_retirement_return:
    jmp .memory_resource_retirement_check
.memory_resource_retirement_fail:
    jmp .halt
.memory_resource_retirement_check:
    test rcx, rcx
    jne .memory_resource_retirement_fail
    cmp rdx, 41
    jne .memory_resource_retirement_fail
    cmp r10, 4
    jne .memory_resource_retirement_fail
    cmp r11, 2
    jne .memory_resource_retirement_fail

    mov cl, 10
    xor edx, edx
    mov r10d, 25
    mov r11d, 1
    mov r12d, 2
    mov r13d, 19
    int 0x90
.global admission_supervisor_memory_resource_reuse_return
admission_supervisor_memory_resource_reuse_return:
    jmp .memory_resource_reuse_check
.memory_resource_reuse_fail:
    jmp .halt
.memory_resource_reuse_check:
    test rcx, rcx
    jne .memory_resource_reuse_fail
    cmp rdx, 10
    jne .memory_resource_reuse_fail
    cmp r10, 11
    jne .memory_resource_reuse_fail
    cmp r11, 33
    jne .memory_resource_reuse_fail

    mov cl, 21
    xor edx, edx
    mov r10d, 33
    mov r11d, 11
    int 0x90
.global admission_supervisor_memory_cell_reuse_return
admission_supervisor_memory_cell_reuse_return:
    jmp .memory_cell_reuse_check
.memory_cell_reuse_fail:
    jmp .halt
.memory_cell_reuse_check:
    test rcx, rcx
    jne .memory_cell_reuse_fail
    cmp rdx, 21
    jne .memory_cell_reuse_fail
    cmp r10, 6
    jne .memory_cell_reuse_fail
    movabs rax, 0x0000400000027000
    cmp r11, rax
    jne .memory_cell_reuse_fail

    movabs r15, 0x4d454d43454c4c36
    mov qword ptr [r11], r15
    movabs rax, 0x4c4c4143544e4741

    mov cl, 4
    xor edx, edx
    mov r10d, 0x0a08
    mov r11d, 0xa88c0008
    xor r12d, r12d
    xor r13d, r13d
    xor r15d, r15d
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
