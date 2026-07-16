//! Fixed x86_64 CPU context frame layouts.
//!
//! This architecture-library module defines the exact `repr(C)` memory layouts
//! consumed by bare-metal context-switch, privilege-boundary, and exception
//! assembly. It performs no privileged operation and allocates no storage,
//! allowing host tests to lock register offsets and bootstrap stack alignment.

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
pub const PRIVILEGE_INTERRUPT_CS_OFFSET: usize =
    core::mem::offset_of!(PrivilegeInterruptStackFrame, cs);
pub const PRIVILEGE_INTERRUPT_STACK_FRAME_BYTES: usize =
    core::mem::size_of::<PrivilegeInterruptStackFrame>();

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PrivilegeErrorCodeStackFrame {
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
    pub error_code: u64,
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub user_rsp: u64,
    pub user_ss: u64,
}

pub const PRIVILEGE_ERROR_CODE_OFFSET: usize =
    core::mem::offset_of!(PrivilegeErrorCodeStackFrame, error_code);
pub const PRIVILEGE_ERROR_CODE_RIP_OFFSET: usize =
    core::mem::offset_of!(PrivilegeErrorCodeStackFrame, rip);
pub const PRIVILEGE_ERROR_CODE_CS_OFFSET: usize =
    core::mem::offset_of!(PrivilegeErrorCodeStackFrame, cs);
pub const PRIVILEGE_ERROR_CODE_STACK_FRAME_BYTES: usize =
    core::mem::size_of::<PrivilegeErrorCodeStackFrame>();

impl PrivilegeErrorCodeStackFrame {
    pub const fn error_code(&self) -> u64 {
        self.error_code
    }

    pub const fn without_error_code(self) -> PrivilegeInterruptStackFrame {
        PrivilegeInterruptStackFrame {
            r15: self.r15,
            r14: self.r14,
            r13: self.r13,
            r12: self.r12,
            r11: self.r11,
            r10: self.r10,
            r9: self.r9,
            r8: self.r8,
            rbp: self.rbp,
            rdi: self.rdi,
            rsi: self.rsi,
            rdx: self.rdx,
            rcx: self.rcx,
            rbx: self.rbx,
            rax: self.rax,
            rip: self.rip,
            cs: self.cs,
            rflags: self.rflags,
            user_rsp: self.user_rsp,
            user_ss: self.user_ss,
        }
    }
}

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

    pub fn frame_mut(&mut self) -> &mut PrivilegeInterruptStackFrame {
        &mut self.frame
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
