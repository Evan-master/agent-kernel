//! Fixed x86_64 CPU context frame layouts.
//!
//! This architecture-library module defines the exact `repr(C)` memory layouts
//! consumed by bare-metal context-switch and same-ring interrupt assembly. It
//! performs no privileged operation and allocates no storage, allowing host
//! tests to lock register offsets and bootstrap stack alignment.

pub const CONTEXT_STACK_ALIGNMENT: usize = 16;
pub const CONTEXT_BOOTSTRAP_BYTES: usize = 64;

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct CalleeSavedFrame {
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub rbx: u64,
    pub rbp: u64,
    pub return_rip: u64,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct InterruptStackFrame {
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rbp: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub rdx: u64,
    pub rcx: u64,
    pub rbx: u64,
    pub rax: u64,
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
}

pub const INTERRUPT_RIP_OFFSET: usize = core::mem::offset_of!(InterruptStackFrame, rip);
pub const INTERRUPT_STACK_FRAME_BYTES: usize = core::mem::size_of::<InterruptStackFrame>();

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PrivilegeInterruptStackFrame {
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rbp: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub rdx: u64,
    pub rcx: u64,
    pub rbx: u64,
    pub rax: u64,
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub user_rsp: u64,
    pub user_ss: u64,
}

pub const PRIVILEGE_INTERRUPT_RIP_OFFSET: usize =
    core::mem::offset_of!(PrivilegeInterruptStackFrame, rip);
pub const PRIVILEGE_INTERRUPT_STACK_FRAME_BYTES: usize =
    core::mem::size_of::<PrivilegeInterruptStackFrame>();

#[repr(transparent)]
#[derive(Debug, PartialEq, Eq)]
pub struct SavedAgentFrame {
    frame: PrivilegeInterruptStackFrame,
}

pub const SAVED_AGENT_FRAME_BYTES: usize = core::mem::size_of::<SavedAgentFrame>();

impl SavedAgentFrame {
    pub const fn new(frame: PrivilegeInterruptStackFrame) -> Self {
        Self { frame }
    }

    pub const fn frame(&self) -> &PrivilegeInterruptStackFrame {
        &self.frame
    }

    pub fn as_mut_ptr(&mut self) -> *mut PrivilegeInterruptStackFrame {
        &mut self.frame
    }
}

pub const fn bootstrap_stack_pointer(stack_start: usize, stack_len: usize) -> Option<usize> {
    if !stack_start.is_multiple_of(CONTEXT_STACK_ALIGNMENT)
        || !stack_len.is_multiple_of(CONTEXT_STACK_ALIGNMENT)
        || stack_len < CONTEXT_BOOTSTRAP_BYTES
    {
        return None;
    }

    let Some(stack_end) = stack_start.checked_add(stack_len) else {
        return None;
    };
    stack_end.checked_sub(CONTEXT_BOOTSTRAP_BYTES)
}
