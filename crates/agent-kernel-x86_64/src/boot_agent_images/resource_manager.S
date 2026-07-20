/* Auditable source for the immutable kind-4 Resource Manager Capsule. */
.intel_syntax noprefix
.section .text
.global resource_manager_entry
resource_manager_entry:
    movabs rax, 0x0000400000004000
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
    mov r10d, 12
    mov r11d, 1
    mov r12d, 1
    mov r13d, 51
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
    test rcx, rcx
    jne .halt
    cmp rdx, 10
    jne .halt
    cmp r10, 3
    jne .halt
    cmp r11, 13
    jne .halt

    mov ecx, 12
    xor edx, edx
    mov r10d, 13
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
    cmp r10, 14
    jne .halt
    test r11, r11
    jne .halt

    mov ecx, 13
    xor edx, edx
    mov r10d, 13
    mov r11d, 14
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
    cmp r10, 14
    jne .halt
    cmp r11, 13
    jne .halt

    mov ecx, 14
    xor edx, edx
    mov r10d, 12
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
    mov r10d, 12
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
    mov r10d, 12
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
    cmp r11, 15
    jne .halt
    cmp r12, 2
    jne .halt

    mov ecx, 17
    xor edx, edx
    mov r10d, 12
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

    mov ecx, 7
    xor edx, edx
    mov r10d, 9
    mov r11d, 1
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
    test rcx, rcx
    jne .halt
    cmp rdx, 7
    jne .halt
    cmp r10, 3
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

    mov ecx, 18
    xor edx, edx
    mov r10d, 12
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
    mov r10d, 12
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
    mov r10d, 12
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

    mov ecx, 35
    xor edx, edx
    mov r10d, 12
    mov r11d, 3
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
    test rcx, rcx
    jne .halt
    cmp rdx, 35
    jne .halt
    cmp r10, 3
    jne .halt
    cmp r11, 9
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

    mov ecx, 36
    xor edx, edx
    mov r10d, 12
    mov r11d, 9
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
    test rcx, rcx
    jne .halt
    cmp rdx, 36
    jne .halt
    cmp r10, 9
    jne .halt
    cmp r11, 1
    jne .halt
    cmp r12, 9
    jne .halt
    test r13, r13
    jne .halt
    test r14, r14
    jne .halt
    test r15, r15
    jne .halt
    test rbp, rbp
    jne .halt

    mov ecx, 37
    xor edx, edx
    mov r10d, 12
    mov r11d, 9
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
    test rcx, rcx
    jne .halt
    cmp rdx, 37
    jne .halt
    cmp r10, 9
    jne .halt
    cmp r11, 1
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

    mov ecx, 17
    xor edx, edx
    mov r10d, 12
    mov r11d, 1
    mov r12d, 15
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
    test rcx, rcx
    jne .halt
    cmp rdx, 17
    jne .halt
    cmp r10, 15
    jne .halt
    cmp r11, 1
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

    mov ecx, 10
    xor edx, edx
    mov r10d, 12
    mov r11d, 1
    mov r12d, 2
    mov r13d, 19
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
    test rcx, rcx
    jne .halt
    cmp rdx, 10
    jne .halt
    cmp r10, 4
    jne .halt
    cmp r11, 16
    jne .halt

    mov ecx, 21
    xor edx, edx
    mov r10d, 16
    mov r11d, 4
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
    test rcx, rcx
    jne .halt
    cmp rdx, 21
    jne .halt
    cmp r10, 2
    jne .halt
    movabs r15, 0x000040000000b000
    cmp r11, r15
    jne .halt
    cmp r12, 4096
    jne .halt
    cmp r13, 1
    jne .halt

    movabs r15, 0x4d454d5041474531
    mov qword ptr [r11], r15

    mov ecx, 22
    xor edx, edx
    mov r10d, 16
    mov r11d, 2
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
    test rcx, rcx
    jne .halt
    cmp rdx, 22
    jne .halt
    cmp r10, 2
    jne .halt
    movabs r15, 0x4d454d5041474531
    cmp r11, r15
    jne .halt
    cmp r12, 1
    jne .halt

    mov ecx, 23
    xor edx, edx
    mov r10d, 16
    mov r11d, 2
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
    test rcx, rcx
    jne .halt
    cmp rdx, 23
    jne .halt
    cmp r10, 2
    jne .halt
    cmp r11, 4
    jne .halt
    cmp r12, 1
    jne .halt

    mov ecx, 44
    xor edx, edx
    mov r10d, 12
    mov r11d, 1
    mov r12d, 0x4e530001
    mov r13d, 30
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
    test rcx, rcx
    jne .halt
    cmp rdx, 44
    jne .halt
    cmp r10, 1
    jne .halt
    cmp r11, 8
    jne .halt
    cmp r12, 1
    jne .halt
    cmp r13, 12
    jne .halt
    cmp r14, 0x4e530001
    jne .halt
    cmp r15, 30
    jne .halt
    cmp rbp, 1
    jne .halt

    mov ecx, 44
    xor edx, edx
    mov r10d, 13
    mov r11d, 3
    mov r12d, 0x4e530002
    mov r13d, 21
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
    test rcx, rcx
    jne .halt
    cmp rdx, 44
    jne .halt
    cmp r10, 2
    jne .halt
    cmp r11, 8
    jne .halt
    cmp r12, 3
    jne .halt
    cmp r13, 13
    jne .halt
    cmp r14, 0x4e530002
    jne .halt
    cmp r15, 21
    jne .halt
    cmp rbp, 1
    jne .halt

    mov ecx, 50
    xor edx, edx
    mov r10d, 1
    mov r11d, 2
    mov r12d, 12
    mov r13d, 0x4e530001
    mov r14d, 13
    mov r15d, 0x4e530002
    xor ebp, ebp
    int 0x90
    test rcx, rcx
    jne .halt
    cmp rdx, 50
    jne .halt
    cmp r10, 2
    jne .halt
    cmp r11, 8
    jne .halt
    cmp r12, 3
    jne .halt
    cmp r13, 13
    jne .halt
    cmp r14, 0x4e530002
    jne .halt
    cmp r15, 21
    jne .halt
    cmp rbp, 1
    jne .halt

    mov ecx, 10
    xor edx, edx
    mov r10d, 12
    mov r11d, 1
    mov r12d, 2
    mov r13d, 19
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
    test rcx, rcx
    jne .halt
    cmp rdx, 10
    jne .halt
    cmp r10, 5
    jne .halt
    cmp r11, 17
    jne .halt

    mov ecx, 24
    xor edx, edx
    mov r10d, 17
    mov r11d, 5
    mov r12d, 3
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
    test rcx, rcx
    jne .halt
    cmp rdx, 24
    jne .halt
    cmp r10, 3
    jne .halt
    movabs r15, 0x000040000000c000
    cmp r11, r15
    jne .halt
    cmp r12, 12288
    jne .halt
    cmp r13, 3
    jne .halt
    cmp r14, 1
    jne .halt

    movabs r15, 0x524547494f4e3031
    mov qword ptr [r11], r15
    movabs r15, 0x524547494f4e3033
    mov qword ptr [r11 + 8192], r15

    mov ecx, 25
    xor edx, edx
    mov r10d, 17
    mov r11d, 3
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
    test rcx, rcx
    jne .halt
    cmp rdx, 25
    jne .halt
    cmp r10, 3
    jne .halt
    movabs r15, 0x524547494f4e3031
    cmp r11, r15
    jne .halt
    movabs r15, 0x524547494f4e3033
    cmp r12, r15
    jne .halt
    cmp r13, 3
    jne .halt
    cmp r14, 1
    jne .halt

    mov ecx, 10
    xor edx, edx
    mov r10d, 12
    mov r11d, 1
    mov r12d, 2
    mov r13d, 19
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
    test rcx, rcx
    jne .halt
    cmp rdx, 10
    jne .halt
    cmp r10, 6
    jne .halt
    cmp r11, 18
    jne .halt

    mov ecx, 24
    xor edx, edx
    mov r10d, 18
    mov r11d, 6
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
    cmp r10, 4
    jne .halt
    movabs r15, 0x000040000000f000
    cmp r11, r15
    jne .halt
    cmp r12, 8192
    jne .halt
    cmp r13, 2
    jne .halt
    cmp r14, 2
    jne .halt

    movabs r15, 0x524547494f4e4231
    mov qword ptr [r11], r15
    movabs r15, 0x524547494f4e4232
    mov qword ptr [r11 + 4096], r15

    mov ecx, 26
    xor edx, edx
    mov r10d, 17
    mov r11d, 3
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
    test rcx, rcx
    jne .halt
    cmp rdx, 26
    jne .halt
    cmp r10, 3
    jne .halt
    cmp r11, 5
    jne .halt
    cmp r12, 3
    jne .halt
    cmp r13, 1
    jne .halt

    mov ecx, 25
    xor edx, edx
    mov r10d, 18
    mov r11d, 4
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
    test rcx, rcx
    jne .halt
    cmp rdx, 25
    jne .halt
    cmp r10, 4
    jne .halt
    movabs r15, 0x524547494f4e4231
    cmp r11, r15
    jne .halt
    movabs r15, 0x524547494f4e4232
    cmp r12, r15
    jne .halt
    cmp r13, 2
    jne .halt
    cmp r14, 2
    jne .halt

    mov ecx, 10
    xor edx, edx
    mov r10d, 12
    mov r11d, 1
    mov r12d, 2
    mov r13d, 19
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
    test rcx, rcx
    jne .halt
    cmp rdx, 10
    jne .halt
    cmp r10, 7
    jne .halt
    cmp r11, 19
    jne .halt

    mov ecx, 24
    xor edx, edx
    mov r10d, 19
    mov r11d, 7
    mov r12d, 3
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
    test rcx, rcx
    jne .halt
    cmp rdx, 24
    jne .halt
    cmp r10, 5
    jne .halt
    movabs r15, 0x000040000000c000
    cmp r11, r15
    jne .halt
    cmp r12, 12288
    jne .halt
    cmp r13, 3
    jne .halt
    cmp r14, 3
    jne .halt

    movabs r15, 0x524547494f4e4331
    mov qword ptr [r11], r15
    movabs r15, 0x524547494f4e4333
    mov qword ptr [r11 + 8192], r15

    mov ecx, 25
    xor edx, edx
    mov r10d, 19
    mov r11d, 5
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
    test rcx, rcx
    jne .halt
    cmp rdx, 25
    jne .halt
    cmp r10, 5
    jne .halt
    movabs r15, 0x524547494f4e4331
    cmp r11, r15
    jne .halt
    movabs r15, 0x524547494f4e4333
    cmp r12, r15
    jne .halt
    cmp r13, 3
    jne .halt
    cmp r14, 3
    jne .halt

    mov ecx, 26
    xor edx, edx
    mov r10d, 18
    mov r11d, 4
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
    test rcx, rcx
    jne .halt
    cmp rdx, 26
    jne .halt
    cmp r10, 4
    jne .halt
    cmp r11, 6
    jne .halt
    cmp r12, 2
    jne .halt
    cmp r13, 2
    jne .halt

    mov ecx, 10
    xor edx, edx
    mov r10d, 12
    mov r11d, 1
    mov r12d, 1
    mov r13d, 51
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90

    mov ecx, 10
    xor edx, edx
    mov r10d, 12
    mov r11d, 1
    mov r12d, 1
    mov r13d, 51
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90

    mov ecx, 48
    xor edx, edx
    mov r10d, 13
    mov r11d, 2
    mov r12d, 1
    mov r13d, 70
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90

    mov ecx, 44
    xor edx, edx
    mov r10d, 20
    mov r11d, 8
    mov r12d, 0x4e530003
    mov r13d, 78
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90

    mov ecx, 44
    xor edx, edx
    mov r10d, 21
    mov r11d, 9
    mov r12d, 0x4e530004
    mov r13d, 65
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90

    movabs r15, 0x0000400000014000
    movabs r14, 0x313548544150534e
    mov qword ptr [r15], r14
    mov qword ptr [r15 + 8], 1
    mov qword ptr [r15 + 16], 1
    mov qword ptr [r15 + 24], 1
    mov qword ptr [r15 + 32], 4
    mov qword ptr [r15 + 40], 112
    mov qword ptr [r15 + 48], 12
    mov qword ptr [r15 + 56], 0x4e530001
    mov qword ptr [r15 + 64], 13
    mov qword ptr [r15 + 72], 0x4e530002
    mov qword ptr [r15 + 80], 20
    mov qword ptr [r15 + 88], 0x4e530003
    mov qword ptr [r15 + 96], 21
    mov qword ptr [r15 + 104], 0x4e530004

    mov ecx, 51
    xor edx, edx
    mov r10d, 1
    mov r11d, 1
    mov r12d, 112
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
    test rcx, rcx
    jne .memory_path_fail
    cmp rdx, 51
    jne .memory_path_fail
    cmp r10, 4
    jne .memory_path_fail
    cmp r11, 8
    jne .memory_path_fail
    cmp r12, 9
    jne .memory_path_fail
    cmp r13, 21
    jne .memory_path_fail
    cmp r14, 0x4e530004
    jne .memory_path_fail
    cmp r15, 65
    jne .memory_path_fail
    cmp rbp, 1
    jne .memory_path_fail
    jmp .memory_path_done
.memory_path_fail:
    jmp .halt
.memory_path_done:

    movabs r15, 0x0000400000014000
    movabs r14, 0x3147534d544e4741
    mov qword ptr [r15], r14
    mov dword ptr [r15 + 32], 160
    mov dword ptr [r15 + 112], 1
    mov dword ptr [r15 + 120], 4
    mov dword ptr [r15 + 128], 1
    mov dword ptr [r15 + 136], 26

    mov ecx, 52
    xor edx, edx
    mov r10d, 1
    xor r11d, r11d
    xor r12d, r12d
    xor r13d, r13d
    xor r14d, r14d
    xor r15d, r15d
    xor ebp, ebp
    int 0x90
    test rcx, rcx
    jne .typed_rebind_fail
    cmp rdx, 52
    jne .typed_rebind_fail
    cmp r10, 4
    jne .typed_rebind_fail
    cmp r15, 26
    jne .typed_rebind_fail
    cmp rbp, 2
    jne .typed_rebind_fail
    jmp .typed_rebind_done
.typed_rebind_fail:
    jmp .halt
.typed_rebind_done:
    .fill 64, 1, 0x90

    mov ecx, 4
    xor edx, edx
    mov r10d, 0xc008
    movabs r11, 0x000900070005000f
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
