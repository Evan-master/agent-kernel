/*
 * Native TPM provider shim for the ring-3 State Signer.
 *
 * The provider has no TPM device authority. It asks the kernel to sign the
 * retained durable manifest through Agent Call 56, then validates the reply.
 */
.intel_syntax noprefix

.equ AGENT_CALL_MAGIC, 0x4c4c4143544e4741
.equ AGENT_CALL_ABI, 1
.equ AGENT_CALL_SIGN_DURABLE, 56
.equ AGENT_CALL_DATA, 0x0000400000030000
.equ MANIFEST_OFFSET, 32
.equ SIGNATURE_OFFSET, 317
.equ ECDSA_P256_SHA256, 2

.extern state_signer_config_nonce
.extern state_signer_config_call_data_generation
.extern state_signer_config_policy_generation
.extern state_signer_config_signature_algorithm

.section .text.state_signer_tpm_provider,"ax",@progbits
.global state_signer_provider_sign
.type state_signer_provider_sign, @function
state_signer_provider_sign:
    movabs rax, AGENT_CALL_DATA + MANIFEST_OFFSET
    cmp rdi, rax
    jne .rejected
    movabs rax, AGENT_CALL_DATA + SIGNATURE_OFFSET
    cmp rsi, rax
    jne .rejected
    cmp rdx, qword ptr [rip + state_signer_config_policy_generation]
    jne .rejected
    cmp qword ptr [rip + state_signer_config_signature_algorithm], ECDSA_P256_SHA256
    jne .rejected

    push rbp
    push rbx
    push r12
    push r13
    push r14
    push r15
    push rcx
    push r8
    push r9

    mov rsi, qword ptr [rsp + 16]
    mov rdi, qword ptr [rsp + 8]
    mov r8, qword ptr [rsp]
    mov r9, qword ptr [rip + state_signer_config_nonce]
    movabs rax, AGENT_CALL_MAGIC
    mov ebx, AGENT_CALL_ABI
    mov ecx, AGENT_CALL_SIGN_DURABLE
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
    jne .call_rejected
    cmp rdx, AGENT_CALL_SIGN_DURABLE
    jne .call_rejected
    cmp rsi, qword ptr [rsp + 16]
    jne .call_rejected
    cmp rdi, qword ptr [rsp + 8]
    jne .call_rejected
    cmp r8, qword ptr [rsp]
    jne .call_rejected
    cmp r9, qword ptr [rip + state_signer_config_nonce]
    jne .call_rejected
    cmp r10, qword ptr [rip + state_signer_config_call_data_generation]
    jne .call_rejected
    cmp r11, qword ptr [rip + state_signer_config_policy_generation]
    jne .call_rejected
    cmp r12, ECDSA_P256_SHA256
    jne .call_rejected
    xor eax, eax
    jmp .restore

.call_rejected:
    mov eax, 1
.restore:
    add rsp, 24
    pop r15
    pop r14
    pop r13
    pop r12
    pop rbx
    pop rbp
    ret

.rejected:
    mov eax, 1
    ret

.size state_signer_provider_sign, . - state_signer_provider_sign
.section .note.GNU-stack,"",@progbits
