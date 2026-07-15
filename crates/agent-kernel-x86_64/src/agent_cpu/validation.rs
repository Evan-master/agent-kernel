//! Read-only validation of suspended CPL3 Agent frames.
//!
//! This child module checks RSP0 bounds, privilege selectors, Agent virtual
//! addresses, return flags, and initial register sanitization. It never mutates
//! CPU or semantic task state.

use agent_kernel_x86_64::{
    context::{PrivilegeInterruptStackFrame, PRIVILEGE_INTERRUPT_STACK_FRAME_BYTES},
    privilege::{USER_CODE_SELECTOR, USER_DATA_SELECTOR},
    user_memory::UserMemoryLayout,
};

use super::storage;
use crate::privilege_runtime::PrivilegedStackBounds;

const RFLAGS_IOPL: u64 = 3 << 12;
const RFLAGS_NESTED_TASK: u64 = 1 << 14;

pub(super) fn read_frame(
    frame_rsp: u64,
    stack: PrivilegedStackBounds,
) -> Option<PrivilegeInterruptStackFrame> {
    let frame_start = usize::try_from(frame_rsp).ok()?;
    let frame_end = frame_start.checked_add(PRIVILEGE_INTERRUPT_STACK_FRAME_BYTES)?;
    if frame_start < stack.start || frame_end > stack.end {
        return None;
    }
    // SAFETY: the complete range lies in the kernel-owned RSP0 stack while CPL3
    // is suspended and cannot modify it.
    Some(unsafe { (frame_rsp as *const PrivilegeInterruptStackFrame).read_volatile() })
}

pub(super) fn user_frame_valid(
    frame: &PrivilegeInterruptStackFrame,
    layout: UserMemoryLayout,
) -> bool {
    frame.cs == u64::from(USER_CODE_SELECTOR)
        && frame.user_ss == u64::from(USER_DATA_SELECTOR)
        && layout.contains_code(frame.rip)
        && layout.contains_stack_pointer(frame.user_rsp)
        && frame.rflags & storage::RFLAGS_INTERRUPT_ENABLE != 0
        && frame.rflags & (RFLAGS_IOPL | RFLAGS_NESTED_TASK) == 0
}

pub(super) fn initial_registers_sanitized(
    frame: &PrivilegeInterruptStackFrame,
    layout: UserMemoryLayout,
) -> bool {
    (frame.rax == 0 || frame.rax == layout.signal_start())
        && frame.rbx == 0
        && frame.rcx == 0
        && frame.rdx == 0
        && frame.rsi == 0
        && frame.rdi == 0
        && frame.rbp == 0
        && frame.r8 == 0
        && frame.r9 == 0
        && frame.r10 == 0
        && frame.r11 == 0
        && frame.r12 == 0
        && frame.r13 == 0
        && frame.r14 == 0
        && frame.r15 == 0
}
