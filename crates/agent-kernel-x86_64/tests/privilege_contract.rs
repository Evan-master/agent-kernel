use core::mem::{offset_of, size_of};

use agent_kernel_x86_64::{
    context::{
        PrivilegeErrorCodeStackFrame, PrivilegeInterruptStackFrame, PRIVILEGE_ERROR_CODE_CS_OFFSET,
        PRIVILEGE_ERROR_CODE_OFFSET, PRIVILEGE_ERROR_CODE_RIP_OFFSET,
        PRIVILEGE_ERROR_CODE_STACK_FRAME_BYTES, PRIVILEGE_INTERRUPT_CS_OFFSET,
    },
    interrupt::{IdtEntry, AGENT_CALL_VECTOR, IDT_USER_INTERRUPT_GATE_OPTIONS},
    privilege::{
        gdt_entries, tss_descriptor, GdtPointer, TaskStateSegment64, GDT_ENTRY_COUNT,
        KERNEL_CODE_DESCRIPTOR, KERNEL_CODE_SELECTOR, KERNEL_DATA_DESCRIPTOR, KERNEL_DATA_SELECTOR,
        TSS_SELECTOR, USER_CODE_DESCRIPTOR, USER_CODE_SELECTOR, USER_DATA_DESCRIPTOR,
        USER_DATA_SELECTOR,
    },
};

#[test]
fn agent_call_gate_is_present_and_callable_from_ring_three() {
    assert_eq!(AGENT_CALL_VECTOR, 0x90);
    let gate = IdtEntry::user_interrupt_gate(0x1122_3344_5566_7788, KERNEL_CODE_SELECTOR);
    assert_eq!(gate.handler_address(), 0x1122_3344_5566_7788);
    assert_eq!(gate.selector(), KERNEL_CODE_SELECTOR);
    assert_eq!(gate.options(), IDT_USER_INTERRUPT_GATE_OPTIONS);
    assert_eq!(gate.options(), 0xee00);
}

#[test]
fn flat_segments_encode_ring_zero_and_ring_three_authority() {
    assert_eq!(KERNEL_CODE_DESCRIPTOR, 0x00af_9a00_0000_ffff);
    assert_eq!(KERNEL_DATA_DESCRIPTOR, 0x00cf_9200_0000_ffff);
    assert_eq!(USER_DATA_DESCRIPTOR, 0x00cf_f200_0000_ffff);
    assert_eq!(USER_CODE_DESCRIPTOR, 0x00af_fa00_0000_ffff);
    assert_eq!(KERNEL_CODE_SELECTOR, 0x08);
    assert_eq!(KERNEL_DATA_SELECTOR, 0x10);
    assert_eq!(USER_DATA_SELECTOR, 0x1b);
    assert_eq!(USER_CODE_SELECTOR, 0x23);
    assert_eq!(TSS_SELECTOR, 0x28);
}

#[test]
fn long_mode_tss_layout_and_descriptor_preserve_rsp0() {
    let rsp0 = 0x1234_5678_9abc_def0;
    let tss = TaskStateSegment64::new(rsp0);
    assert_eq!(size_of::<TaskStateSegment64>(), 104);
    assert_eq!(offset_of!(TaskStateSegment64, rsp0), 4);
    assert_eq!(offset_of!(TaskStateSegment64, iomap_base), 102);
    assert_eq!(tss.rsp0(), rsp0);
    assert_eq!(tss.iomap_base(), 104);

    let base = 0x1122_3344_5566_7788;
    let (low, high) = tss_descriptor(base);
    assert_eq!((low >> 40) & 0xff, 0x89);
    assert_eq!(low & 0xffff, 103);
    let decoded_base =
        ((low >> 16) & 0x00ff_ffff) | (((low >> 56) & 0xff) << 24) | ((high & 0xffff_ffff) << 32);
    assert_eq!(decoded_base, base);
}

#[test]
fn gdt_pointer_covers_exact_permanent_table() {
    let entries = gdt_entries(0x1122_3344_5566_7788);
    assert_eq!(entries.len(), GDT_ENTRY_COUNT);
    assert_eq!(entries[0], 0);
    assert_eq!(entries[1], KERNEL_CODE_DESCRIPTOR);
    assert_eq!(entries[2], KERNEL_DATA_DESCRIPTOR);
    assert_eq!(entries[3], USER_DATA_DESCRIPTOR);
    assert_eq!(entries[4], USER_CODE_DESCRIPTOR);

    let pointer = GdtPointer::for_table(entries.as_ptr() as u64, entries.len()).unwrap();
    assert_eq!(pointer.limit(), 55);
    assert_eq!(pointer.base(), entries.as_ptr() as u64);
}

#[test]
fn privilege_interrupt_frame_matches_hardware_ring_transition() {
    assert_eq!(size_of::<PrivilegeInterruptStackFrame>(), 160);
    assert_eq!(offset_of!(PrivilegeInterruptStackFrame, r15), 0);
    assert_eq!(offset_of!(PrivilegeInterruptStackFrame, rax), 112);
    assert_eq!(offset_of!(PrivilegeInterruptStackFrame, rip), 120);
    assert_eq!(offset_of!(PrivilegeInterruptStackFrame, cs), 128);
    assert_eq!(PRIVILEGE_INTERRUPT_CS_OFFSET, 128);
    assert_eq!(offset_of!(PrivilegeInterruptStackFrame, rflags), 136);
    assert_eq!(offset_of!(PrivilegeInterruptStackFrame, user_rsp), 144);
    assert_eq!(offset_of!(PrivilegeInterruptStackFrame, user_ss), 152);
}

#[test]
fn privilege_error_code_frame_matches_hardware_ring_transition() {
    assert_eq!(size_of::<PrivilegeErrorCodeStackFrame>(), 168);
    assert_eq!(offset_of!(PrivilegeErrorCodeStackFrame, r15), 0);
    assert_eq!(offset_of!(PrivilegeErrorCodeStackFrame, rax), 112);
    assert_eq!(offset_of!(PrivilegeErrorCodeStackFrame, error_code), 120);
    assert_eq!(PRIVILEGE_ERROR_CODE_OFFSET, 120);
    assert_eq!(offset_of!(PrivilegeErrorCodeStackFrame, rip), 128);
    assert_eq!(PRIVILEGE_ERROR_CODE_RIP_OFFSET, 128);
    assert_eq!(offset_of!(PrivilegeErrorCodeStackFrame, cs), 136);
    assert_eq!(PRIVILEGE_ERROR_CODE_CS_OFFSET, 136);
    assert_eq!(offset_of!(PrivilegeErrorCodeStackFrame, rflags), 144);
    assert_eq!(offset_of!(PrivilegeErrorCodeStackFrame, user_rsp), 152);
    assert_eq!(offset_of!(PrivilegeErrorCodeStackFrame, user_ss), 160);
    assert_eq!(PRIVILEGE_ERROR_CODE_STACK_FRAME_BYTES, 168);
}

#[test]
fn privilege_error_code_frame_normalizes_without_resuming_the_error_slot() {
    let frame = PrivilegeErrorCodeStackFrame {
        r15: 15,
        r14: 14,
        r13: 13,
        r12: 12,
        r11: 11,
        r10: 10,
        r9: 9,
        r8: 8,
        rbp: 7,
        rdi: 6,
        rsi: 5,
        rdx: 4,
        rcx: 3,
        rbx: 2,
        rax: 1,
        error_code: 0x1234,
        rip: 0x4000,
        cs: 0x23,
        rflags: 0x202,
        user_rsp: 0x8000,
        user_ss: 0x1b,
    };

    assert_eq!(frame.error_code(), 0x1234);
    assert_eq!(
        frame.without_error_code(),
        PrivilegeInterruptStackFrame {
            r15: 15,
            r14: 14,
            r13: 13,
            r12: 12,
            r11: 11,
            r10: 10,
            r9: 9,
            r8: 8,
            rbp: 7,
            rdi: 6,
            rsi: 5,
            rdx: 4,
            rcx: 3,
            rbx: 2,
            rax: 1,
            rip: 0x4000,
            cs: 0x23,
            rflags: 0x202,
            user_rsp: 0x8000,
            user_ss: 0x1b,
        }
    );
}
