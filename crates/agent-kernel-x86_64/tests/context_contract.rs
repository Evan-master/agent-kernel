use core::mem::{align_of, offset_of, size_of};

use agent_kernel_x86_64::context::{
    bootstrap_stack_pointer, CalleeSavedFrame, InterruptStackFrame, CONTEXT_BOOTSTRAP_BYTES,
    CONTEXT_STACK_ALIGNMENT, INTERRUPT_RIP_OFFSET, INTERRUPT_STACK_FRAME_BYTES,
};

#[test]
fn callee_saved_frame_matches_context_switch_pop_order() {
    assert_eq!(align_of::<CalleeSavedFrame>(), 8);
    assert_eq!(size_of::<CalleeSavedFrame>(), 56);
    assert_eq!(offset_of!(CalleeSavedFrame, r15), 0);
    assert_eq!(offset_of!(CalleeSavedFrame, r14), 8);
    assert_eq!(offset_of!(CalleeSavedFrame, r13), 16);
    assert_eq!(offset_of!(CalleeSavedFrame, r12), 24);
    assert_eq!(offset_of!(CalleeSavedFrame, rbx), 32);
    assert_eq!(offset_of!(CalleeSavedFrame, rbp), 40);
    assert_eq!(offset_of!(CalleeSavedFrame, return_rip), 48);
}

#[test]
fn same_ring_interrupt_frame_matches_irq_push_order() {
    assert_eq!(align_of::<InterruptStackFrame>(), 8);
    assert_eq!(size_of::<InterruptStackFrame>(), 144);
    assert_eq!(offset_of!(InterruptStackFrame, r15), 0);
    assert_eq!(offset_of!(InterruptStackFrame, r8), 56);
    assert_eq!(offset_of!(InterruptStackFrame, rbp), 64);
    assert_eq!(offset_of!(InterruptStackFrame, rax), 112);
    assert_eq!(offset_of!(InterruptStackFrame, rip), 120);
    assert_eq!(offset_of!(InterruptStackFrame, cs), 128);
    assert_eq!(offset_of!(InterruptStackFrame, rflags), 136);
    assert_eq!(INTERRUPT_RIP_OFFSET, 120);
    assert_eq!(INTERRUPT_STACK_FRAME_BYTES, 144);
}

#[test]
fn bootstrap_stack_reserves_abi_aligned_entry_frame() {
    assert_eq!(CONTEXT_STACK_ALIGNMENT, 16);
    assert_eq!(CONTEXT_BOOTSTRAP_BYTES, 64);
    let rsp = bootstrap_stack_pointer(0x1000, 0x8000).unwrap();

    assert_eq!(rsp, 0x8fc0);
    assert_eq!(rsp % CONTEXT_STACK_ALIGNMENT, 0);
    assert_eq!((rsp + size_of::<CalleeSavedFrame>()) % 16, 8);
    assert_eq!(bootstrap_stack_pointer(0x1001, 0x8000), None);
    assert_eq!(bootstrap_stack_pointer(0x1000, 63), None);
    assert_eq!(bootstrap_stack_pointer(usize::MAX - 15, 32), None);
}
