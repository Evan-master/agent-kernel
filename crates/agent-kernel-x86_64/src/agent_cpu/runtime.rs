//! Type-state runtime for one fixed x86_64 Agent CPU context.
//!
//! Each transition validates assembly evidence before exposing the next token
//! to the semantic task adapter.

use core::{arch::asm, sync::atomic::Ordering};

use agent_kernel_x86_64::context::{InterruptStackFrame, INTERRUPT_STACK_FRAME_BYTES};

use super::{assembly, storage};
use crate::pit_timer;

pub(crate) struct PreparedAgentCpu {
    stack: storage::StackBounds,
}

pub(crate) struct PreemptedAgentCpu {
    stack: storage::StackBounds,
    frame_rsp: u64,
    frame_rip: u64,
    ticks: u8,
}

pub(crate) struct YieldedAgentCpu {
    yields: u8,
}

impl PreparedAgentCpu {
    pub(crate) fn prepare() -> Option<Self> {
        let entry_rip = agent_task_entry as *const () as usize as u64;
        Some(Self {
            stack: storage::initialize(entry_rip)?,
        })
    }

    pub(crate) fn run_until_preempted(self) -> Option<PreemptedAgentCpu> {
        // SAFETY: both context slots and the bootstrap frame are exclusively
        // owned by this type-state transition, with IF clear on entry.
        unsafe {
            assembly::context_switch(
                storage::AGENT_KERNEL_HOST_CONTEXT_RSP.pointer(),
                storage::AGENT_KERNEL_AGENT_CONTEXT_RSP.pointer(),
            );
        }
        pit_timer::disarm();

        let frame_rsp = storage::AGENT_KERNEL_AGENT_INTERRUPT_RSP.load(Ordering::Acquire);
        let frame_rip = storage::AGENT_KERNEL_AGENT_INTERRUPT_RIP.load(Ordering::Acquire);
        let frame_end = usize::try_from(frame_rsp)
            .ok()?
            .checked_add(INTERRUPT_STACK_FRAME_BYTES)?;
        if storage::AGENT_KERNEL_AGENT_ARM_FAILED.load(Ordering::Acquire) != 0
            || storage::AGENT_KERNEL_AGENT_ENTRY_COUNT.load(Ordering::Acquire) != 1
            || storage::AGENT_KERNEL_AGENT_IRQ_COUNT.load(Ordering::Acquire) != 1
            || storage::AGENT_KERNEL_AGENT_IRQ_SEEN.load(Ordering::Acquire) != 1
            || storage::AGENT_KERNEL_AGENT_PREEMPTED.load(Ordering::Acquire) != 1
            || storage::AGENT_KERNEL_HOST_CONTEXT_RSP.load() == 0
            || frame_rsp < self.stack.start as u64
            || frame_end > self.stack.end
            || !storage::stack_canary_valid(self.stack)
            || !storage::interrupts_are_clear()
        {
            return None;
        }

        // SAFETY: the complete frame range was checked against the fixed stack,
        // and the Agent cannot mutate it while the kernel owns execution.
        let frame = unsafe { (frame_rsp as *const InterruptStackFrame).read_volatile() };
        let entry_rsp = storage::AGENT_KERNEL_AGENT_ENTRY_RSP.load(Ordering::Acquire);
        if frame.rip != frame_rip
            || frame.rip == 0
            || frame.cs & 0x3 != 0
            || frame.rflags & storage::RFLAGS_INTERRUPT_ENABLE == 0
            || entry_rsp < self.stack.start as u64
            || entry_rsp >= self.stack.end as u64
        {
            return None;
        }

        Some(PreemptedAgentCpu {
            stack: self.stack,
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

    pub(crate) fn resume_until_yield(self) -> Option<YieldedAgentCpu> {
        if storage::AGENT_KERNEL_AGENT_INTERRUPT_RSP.load(Ordering::Acquire) != self.frame_rsp
            || storage::AGENT_KERNEL_AGENT_INTERRUPT_RIP.load(Ordering::Acquire) != self.frame_rip
            || storage::AGENT_KERNEL_AGENT_PREEMPTED.load(Ordering::Acquire) != 1
        {
            return None;
        }

        // SAFETY: frame_rsp names the validated complete interrupt frame. The
        // resume assembly saves a fresh kernel continuation before iretq.
        unsafe {
            assembly::resume_interrupted_agent(
                storage::AGENT_KERNEL_HOST_CONTEXT_RSP.pointer(),
                self.frame_rsp,
            );
        }
        pit_timer::disarm();

        let yielded_rsp = storage::AGENT_KERNEL_AGENT_CONTEXT_RSP.load();
        if storage::AGENT_KERNEL_AGENT_RESUMED.load(Ordering::Acquire) != 1
            || storage::AGENT_KERNEL_AGENT_YIELDED.load(Ordering::Acquire) != 1
            || storage::AGENT_KERNEL_AGENT_IRQ_COUNT.load(Ordering::Acquire) != 1
            || yielded_rsp < self.stack.start as u64
            || yielded_rsp >= self.stack.end as u64
            || !storage::stack_canary_valid(self.stack)
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

extern "C" fn agent_task_entry() -> ! {
    let rsp: u64;
    // SAFETY: reading the active stack pointer has no side effect.
    unsafe {
        asm!("mov {}, rsp", out(reg) rsp, options(nomem, nostack, preserves_flags));
    }
    storage::AGENT_KERNEL_AGENT_ENTRY_RSP.store(rsp, Ordering::Release);
    storage::AGENT_KERNEL_AGENT_ENTRY_COUNT.fetch_add(1, Ordering::AcqRel);

    if pit_timer::arm(assembly::agent_kernel_agent_timer_irq_stub).is_none() {
        storage::AGENT_KERNEL_AGENT_ARM_FAILED.store(1, Ordering::Release);
        // SAFETY: the initial switch already populated the host context slot.
        unsafe {
            assembly::context_switch(
                storage::AGENT_KERNEL_AGENT_CONTEXT_RSP.pointer(),
                storage::AGENT_KERNEL_HOST_CONTEXT_RSP.pointer(),
            );
        }
        stop_agent();
    }

    // SAFETY: IRQ0 is installed and the current RSP is inside the Agent stack.
    unsafe {
        asm!("sti", options(nomem, nostack));
    }
    while storage::AGENT_KERNEL_AGENT_PREEMPTED.load(Ordering::Acquire) == 0 {
        core::hint::spin_loop();
    }

    storage::AGENT_KERNEL_AGENT_RESUMED.store(1, Ordering::Release);
    storage::AGENT_KERNEL_AGENT_YIELDED.store(1, Ordering::Release);
    // SAFETY: the resume path saved a fresh host continuation before iretq.
    unsafe {
        assembly::context_switch(
            storage::AGENT_KERNEL_AGENT_CONTEXT_RSP.pointer(),
            storage::AGENT_KERNEL_HOST_CONTEXT_RSP.pointer(),
        );
    }
    stop_agent()
}

fn stop_agent() -> ! {
    loop {
        // SAFETY: this is an unreachable terminal path for a context that must
        // not continue without an explicit future resume transition.
        unsafe {
            asm!("cli", "hlt", options(nomem, nostack));
        }
    }
}
