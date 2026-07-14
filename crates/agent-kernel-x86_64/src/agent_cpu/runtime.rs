//! Type-state runtime for one ring-3 x86_64 Agent context.
//!
//! Each transition validates a complete privilege frame on the TSS RSP0 stack
//! before exposing evidence to the semantic task adapter.

use core::sync::atomic::Ordering;

use agent_kernel_x86_64::{
    context::{PrivilegeInterruptStackFrame, PRIVILEGE_INTERRUPT_STACK_FRAME_BYTES},
    interrupt::AGENT_CALL_VECTOR,
    privilege::{USER_CODE_SELECTOR, USER_DATA_SELECTOR},
    user_memory::AGENT_CALL_RETURN_OFFSET,
};

use super::{assembly, storage};
use crate::{
    agent_memory::PreparedUserMemory,
    exception_runtime, pit_timer,
    privilege_runtime::{
        current_privilege_level, stack_canary_valid, PrivilegeBoundary, PrivilegedStackBounds,
    },
};

const RFLAGS_IOPL: u64 = 3 << 12;
const RFLAGS_NESTED_TASK: u64 = 1 << 14;

pub(crate) struct PreparedAgentCpu {
    memory: PreparedUserMemory,
    kernel_stack: PrivilegedStackBounds,
}

pub(crate) struct PreemptedAgentCpu {
    memory: PreparedUserMemory,
    kernel_stack: PrivilegedStackBounds,
    frame_rsp: u64,
    frame_rip: u64,
    ticks: u8,
}

pub(crate) struct YieldedAgentCpu {
    yields: u8,
}

impl PreparedAgentCpu {
    pub(crate) fn prepare(
        privilege: &PrivilegeBoundary,
        memory: PreparedUserMemory,
    ) -> Option<Self> {
        storage::initialize()?;
        let kernel_stack = privilege.stack_bounds();
        if current_privilege_level() != 0 || !stack_canary_valid(kernel_stack) {
            return None;
        }
        // SAFETY: storage initialization left IF clear, and this gate is the
        // single bounded Agent-call ingress for the current IDT.
        unsafe {
            exception_runtime::install_user_interrupt_gate(
                AGENT_CALL_VECTOR,
                assembly::agent_kernel_agent_call_stub,
            )?;
        }
        Some(Self {
            memory,
            kernel_stack,
        })
    }

    pub(crate) fn run_until_preempted(self) -> Option<PreemptedAgentCpu> {
        pit_timer::arm(assembly::agent_kernel_agent_timer_irq_stub)?;
        let layout = self.memory.layout();
        // SAFETY: both user pages and the RSP0 stack were validated, all gates
        // are live, and entry constructs the complete privilege return frame.
        unsafe {
            assembly::enter_user(
                storage::AGENT_KERNEL_HOST_CONTEXT_RSP.pointer(),
                layout.code_start(),
                layout.stack_top(),
                USER_CODE_SELECTOR,
                USER_DATA_SELECTOR,
            );
        }
        pit_timer::disarm();

        let frame_rsp = storage::AGENT_KERNEL_AGENT_INTERRUPT_RSP.load(Ordering::Acquire);
        let frame_rip = storage::AGENT_KERNEL_AGENT_INTERRUPT_RIP.load(Ordering::Acquire);
        let frame = read_validated_frame(frame_rsp, self.kernel_stack)?;
        if storage::AGENT_KERNEL_AGENT_IRQ_COUNT.load(Ordering::Acquire) != 1
            || storage::AGENT_KERNEL_AGENT_IRQ_SEEN.load(Ordering::Acquire) != 1
            || storage::AGENT_KERNEL_AGENT_PREEMPTED.load(Ordering::Acquire) != 1
            || storage::AGENT_KERNEL_HOST_CONTEXT_RSP.load() == 0
            || frame.rip != frame_rip
            || !user_frame_valid(&frame, layout)
            || current_privilege_level() != 0
            || !stack_canary_valid(self.kernel_stack)
            || !storage::interrupts_are_clear()
        {
            return None;
        }

        Some(PreemptedAgentCpu {
            memory: self.memory,
            kernel_stack: self.kernel_stack,
            frame_rsp,
            frame_rip,
            ticks: 1,
        })
    }
}

impl PreemptedAgentCpu {
    pub(crate) const fn tick_count(&self) -> u8 {
        self.ticks
    }

    pub(crate) fn resume_until_yield(mut self) -> Option<YieldedAgentCpu> {
        if storage::AGENT_KERNEL_AGENT_INTERRUPT_RSP.load(Ordering::Acquire) != self.frame_rsp
            || storage::AGENT_KERNEL_AGENT_INTERRUPT_RIP.load(Ordering::Acquire) != self.frame_rip
            || storage::AGENT_KERNEL_AGENT_PREEMPTED.load(Ordering::Acquire) != 1
            || !self.memory.release_for_agent_call()
        {
            return None;
        }

        // SAFETY: frame_rsp names the validated complete privilege frame. The
        // resume assembly saves a fresh kernel continuation before iretq.
        unsafe {
            assembly::resume_interrupted_user(
                storage::AGENT_KERNEL_HOST_CONTEXT_RSP.pointer(),
                self.frame_rsp,
            );
        }
        pit_timer::disarm();

        let call_rsp = storage::AGENT_KERNEL_AGENT_CALL_RSP.load(Ordering::Acquire);
        let call_rip = storage::AGENT_KERNEL_AGENT_CALL_RIP.load(Ordering::Acquire);
        let frame = read_validated_frame(call_rsp, self.kernel_stack)?;
        let layout = self.memory.layout();
        if storage::AGENT_KERNEL_AGENT_CALL_COUNT.load(Ordering::Acquire) != 1
            || storage::AGENT_KERNEL_AGENT_CALL_SEEN.load(Ordering::Acquire) != 1
            || storage::AGENT_KERNEL_AGENT_YIELDED.load(Ordering::Acquire) != 1
            || storage::AGENT_KERNEL_AGENT_IRQ_COUNT.load(Ordering::Acquire) != 1
            || frame.rip != call_rip
            || frame.rip != layout.code_start() + AGENT_CALL_RETURN_OFFSET
            || frame.rax != layout.signal_start()
            || !user_frame_valid(&frame, layout)
            || current_privilege_level() != 0
            || !stack_canary_valid(self.kernel_stack)
            || !storage::interrupts_are_clear()
        {
            return None;
        }

        Some(YieldedAgentCpu { yields: 1 })
    }
}

impl YieldedAgentCpu {
    pub(crate) const fn yield_count(&self) -> u8 {
        self.yields
    }
}

fn read_validated_frame(
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

fn user_frame_valid(
    frame: &PrivilegeInterruptStackFrame,
    layout: agent_kernel_x86_64::user_memory::UserMemoryLayout,
) -> bool {
    frame.cs == u64::from(USER_CODE_SELECTOR)
        && frame.user_ss == u64::from(USER_DATA_SELECTOR)
        && layout.contains_code(frame.rip)
        && layout.contains_stack_pointer(frame.user_rsp)
        && frame.rflags & storage::RFLAGS_INTERRUPT_ENABLE != 0
        && frame.rflags & (RFLAGS_IOPL | RFLAGS_NESTED_TASK) == 0
}
