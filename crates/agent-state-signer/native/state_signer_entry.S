/* Native ring-3 State Signer entry with an external provider symbol. */
.intel_syntax noprefix

.equ AGENT_CALL_MAGIC, 0x4c4c4143544e4741
.equ AGENT_CALL_ABI, 1
.equ AGENT_CALL_DESCRIBE, 1
.equ AGENT_CALL_COMPLETE, 3
.equ AGENT_CALL_SUBMIT_RESULT, 4
.equ AGENT_CALL_PREPARE_DURABLE, 54
.equ AGENT_CALL_COMMIT_DURABLE, 55
.equ AGENT_CALL_DATA, 0x0000400000030000
.equ AGENT_SIGNAL_PAGE, 0x0000400000020000

.equ REQUEST_MAGIC, 0x3531515241444b41
.equ REQUEST_BYTES, 384
.equ MANIFEST_BYTES, 285
.equ SIGNATURE_OFFSET, 317
.equ SIGNATURE_BYTES, 64
.equ RESERVED_OFFSET, 381
.equ MANIFEST_ACTOR_OFFSET, 165
.equ MANIFEST_ARCHIVE_AUTHORITY_OFFSET, 173
.equ MANIFEST_ROOT_OFFSET, 181
.equ MANIFEST_STORAGE_OFFSET, 189
.equ MANIFEST_SIGNER_ID_OFFSET, 237
.equ MANIFEST_POLICY_GENERATION_OFFSET, 269
.equ MANIFEST_THROUGH_SEQUENCE_OFFSET, 85

.extern state_signer_config_nonce
.extern state_signer_config_archive_authority
.extern state_signer_config_storage_authority
.extern state_signer_config_root
.extern state_signer_config_storage
.extern state_signer_config_through_sequence
.extern state_signer_config_call_data_generation
.extern state_signer_config_policy_generation
.extern state_signer_config_signer_id
.extern state_signer_provider_sign

.section .text.state_signer_entry,"ax",@progbits
.global state_signer_entry
.type state_signer_entry, @function
state_signer_entry:
    movabs rax, AGENT_SIGNAL_PAGE
.wait_for_release:
    cmp byte ptr [rax], 0
    je .wait_for_release

    movabs rax, AGENT_CALL_MAGIC
    mov ebx, AGENT_CALL_ABI
    mov ecx, AGENT_CALL_DESCRIBE
    xor edx, edx
    mov rsi, qword ptr [rip + state_signer_config_nonce]
    xor edi, edi
    xor r8d, r8d
    xor r9d, r9d
    xor r10d, r10d
    xor r11d, r11d
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
    test rcx, rcx
    jne .fault
    cmp rdx, AGENT_CALL_DESCRIBE
    jne .fault

    movabs rax, AGENT_CALL_MAGIC
    mov ebx, AGENT_CALL_ABI
    mov ecx, AGENT_CALL_PREPARE_DURABLE
    xor edx, edx
    mov r10, qword ptr [rip + state_signer_config_archive_authority]
    mov r11, qword ptr [rip + state_signer_config_storage_authority]
    mov r12, qword ptr [rip + state_signer_config_through_sequence]
    mov r13, qword ptr [rip + state_signer_config_call_data_generation]
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
    test rcx, rcx
    jne .fault
    cmp rdx, AGENT_CALL_PREPARE_DURABLE
    jne .fault
    test r10, r10
    je .fault
    test r11, r11
    je .fault
    cmp r12, qword ptr [rip + state_signer_config_through_sequence]
    jne .fault
    test r13, r13
    je .fault
    cmp r14, MANIFEST_BYTES
    jne .fault
    cmp r15, REQUEST_BYTES
    jne .fault
    cmp rbp, qword ptr [rip + state_signer_config_policy_generation]
    jne .fault

    /* Preserve the authenticated identity while provider code runs. */
    push r8
    push rdi
    push rsi
    push r9

    movabs r12, AGENT_CALL_DATA
    movabs rax, REQUEST_MAGIC
    cmp qword ptr [r12], rax
    jne .fault
    cmp word ptr [r12 + 8], 1
    jne .fault
    cmp word ptr [r12 + 10], 0
    jne .fault
    cmp dword ptr [r12 + 12], REQUEST_BYTES
    jne .fault
    mov rax, qword ptr [rip + state_signer_config_call_data_generation]
    cmp qword ptr [r12 + 16], rax
    jne .fault
    mov rax, qword ptr [rip + state_signer_config_storage_authority]
    cmp qword ptr [r12 + 24], rax
    jne .fault
    mov rax, qword ptr [rsp + 24]
    cmp qword ptr [r12 + MANIFEST_ACTOR_OFFSET], rax
    jne .fault
    mov rax, qword ptr [rip + state_signer_config_archive_authority]
    cmp qword ptr [r12 + MANIFEST_ARCHIVE_AUTHORITY_OFFSET], rax
    jne .fault
    mov rax, qword ptr [rip + state_signer_config_root]
    cmp qword ptr [r12 + MANIFEST_ROOT_OFFSET], rax
    jne .fault
    mov rax, qword ptr [rip + state_signer_config_storage]
    cmp qword ptr [r12 + MANIFEST_STORAGE_OFFSET], rax
    jne .fault
    mov rax, qword ptr [rip + state_signer_config_through_sequence]
    cmp qword ptr [r12 + MANIFEST_THROUGH_SEQUENCE_OFFSET], rax
    jne .fault
    mov rax, qword ptr [rip + state_signer_config_policy_generation]
    cmp qword ptr [r12 + MANIFEST_POLICY_GENERATION_OFFSET], rax
    jne .fault

    lea rsi, [r12 + MANIFEST_SIGNER_ID_OFFSET]
    lea rdi, [rip + state_signer_config_signer_id]
    mov ecx, 4
    cld
    repe cmpsq
    jne .fault

    lea rsi, [r12 + SIGNATURE_OFFSET]
    mov ecx, SIGNATURE_BYTES
.check_empty_signature:
    cmp byte ptr [rsi], 0
    jne .fault
    inc rsi
    loop .check_empty_signature
    cmp word ptr [r12 + RESERVED_OFFSET], 0
    jne .fault
    cmp byte ptr [r12 + RESERVED_OFFSET + 2], 0
    jne .fault

    /* Snapshot every byte before allowing the provider to fill the signature. */
    sub rsp, REQUEST_BYTES
    mov r14, rsp
    mov rsi, r12
    mov rdi, r14
    mov ecx, REQUEST_BYTES
    rep movsb

    mov r15, rsp
    and rsp, -16
    lea rdi, [r12 + 32]
    lea rsi, [r12 + SIGNATURE_OFFSET]
    mov rdx, qword ptr [rip + state_signer_config_policy_generation]
    call state_signer_provider_sign
    mov rsp, r15
    test eax, eax
    jne .fault

    cld
    mov rsi, r12
    mov rdi, r14
    mov ecx, SIGNATURE_OFFSET
    repe cmpsb
    jne .fault
    lea rsi, [r12 + RESERVED_OFFSET]
    lea rdi, [r14 + RESERVED_OFFSET]
    mov ecx, REQUEST_BYTES - RESERVED_OFFSET
    repe cmpsb
    jne .fault

    lea rsi, [r12 + SIGNATURE_OFFSET]
    mov ecx, SIGNATURE_BYTES
    xor eax, eax
.collect_signature:
    or al, byte ptr [rsi]
    inc rsi
    loop .collect_signature
    test al, al
    je .fault

    add rsp, REQUEST_BYTES
    pop r9
    pop rsi
    pop rdi
    pop r8

    movabs rax, AGENT_CALL_MAGIC
    mov ebx, AGENT_CALL_ABI
    mov ecx, AGENT_CALL_COMMIT_DURABLE
    xor edx, edx
    mov r10, qword ptr [rip + state_signer_config_call_data_generation]
    xor r11d, r11d
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
    test rcx, rcx
    jne .fault
    cmp rdx, AGENT_CALL_COMMIT_DURABLE
    jne .fault
    test r10, r10
    je .fault
    cmp r11, qword ptr [rip + state_signer_config_through_sequence]
    jne .fault
    test r12, r12
    je .fault

    movabs rax, AGENT_CALL_MAGIC
    mov ebx, AGENT_CALL_ABI
    mov ecx, AGENT_CALL_SUBMIT_RESULT
    xor edx, edx
    mov r10d, 0x0a17
    mov r11, qword ptr [rip + state_signer_config_call_data_generation]
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
    test rcx, rcx
    jne .fault
    cmp rdx, AGENT_CALL_SUBMIT_RESULT
    jne .fault

    movabs rax, AGENT_CALL_MAGIC
    mov ebx, AGENT_CALL_ABI
    mov ecx, AGENT_CALL_COMPLETE
    xor edx, edx
    xor r10d, r10d
    xor r11d, r11d
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90

.fault:
    ud2
    jmp .fault

.size state_signer_entry, . - state_signer_entry
.section .note.GNU-stack,"",@progbits
