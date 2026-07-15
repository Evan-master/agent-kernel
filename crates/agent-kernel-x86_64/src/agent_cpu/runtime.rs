//! Type-state runtime for multiple suspended ring-3 Agent contexts.
//!
//! One installed CPU boundary resets evidence for each physical dispatch. Every
//! preempted context owns a copied privilege frame, so the shared TSS RSP0 stack
//! can accept another Agent interrupt before the first context resumes.

use core::sync::atomic::Ordering;

use agent_kernel_x86_64::{
    address_space::AddressSpaceRoots,
    agent_call::{AgentCallContext, AgentCallRequest},
    context::SavedAgentFrame,
    interrupt::AGENT_CALL_VECTOR,
    privilege::{USER_CODE_SELECTOR, USER_DATA_SELECTOR},
};

use super::{assembly, call, storage, validation};
use crate::{
    agent_memory::PreparedAgentMemory,
    exception_runtime, pit_timer,
    privilege_runtime::{
        current_privilege_level, stack_canary_valid, PrivilegeBoundary, PrivilegedStackBounds,
    },
};

#[derive(Copy, Clone)]
pub(crate) struct AgentCpuRuntime {
    kernel_stack: PrivilegedStackBounds,
    kernel_cr3: u64,
}

pub(crate) struct PreparedAgentCpu {
    memory: PreparedAgentMemory,
    runtime: AgentCpuRuntime,
    context: AgentCallContext,
}

pub(crate) struct PreemptedAgentCpu {
    memory: PreparedAgentMemory,
    runtime: AgentCpuRuntime,
    frame: SavedAgentFrame,
    context: AgentCallContext,
    ticks: u8,
}

pub(crate) struct YieldedAgentCpu {
    yields: u8,
    calls: u8,
    address_space_switches: u8,
    describe_return_offset: u32,
    yield_return_offset: u32,
    nonce: u64,
}

impl AgentCpuRuntime {
    pub(crate) fn install(privilege: &PrivilegeBoundary, roots: AddressSpaceRoots) -> Option<Self> {
        storage::install(roots)?;
        let kernel_stack = privilege.stack_bounds();
        if current_privilege_level() != 0 || !stack_canary_valid(kernel_stack) {
            return None;
        }
        // SAFETY: installation holds IF clear and writes the one bounded DPL3
        // Agent-call gate used by every context on this boot CPU.
        unsafe {
            exception_runtime::install_user_interrupt_gate(
                AGENT_CALL_VECTOR,
                assembly::agent_kernel_agent_call_stub,
            )?;
        }
        Some(Self {
            kernel_stack,
            kernel_cr3: roots.kernel_cr3(),
        })
    }

    pub(crate) fn prepare(
        &self,
        memory: PreparedAgentMemory,
        context: AgentCallContext,
    ) -> Option<PreparedAgentCpu> {
        if memory.roots().kernel_cr3() != self.kernel_cr3
            || !memory.kernel_address_space_active()
            || !memory.signal_is_clear()
            || !stack_canary_valid(self.kernel_stack)
        {
            return None;
        }
        Some(PreparedAgentCpu {
            memory,
            runtime: *self,
            context,
        })
    }
}

impl PreparedAgentCpu {
    pub(crate) fn run_until_preempted(self) -> Option<PreemptedAgentCpu> {
        let roots = self.memory.roots();
        storage::begin_dispatch(roots)?;
        pit_timer::arm(assembly::agent_kernel_agent_timer_irq_stub)?;
        let layout = self.memory.layout();
        // SAFETY: private Agent pages, shared supervisor mappings, RSP0, gates,
        // and the per-dispatch evidence mailbox are all validated.
        unsafe {
            assembly::enter_user(
                storage::AGENT_KERNEL_HOST_CONTEXT_RSP.pointer(),
                self.memory.entry_rip(),
                layout.stack_top(),
                USER_CODE_SELECTOR,
                USER_DATA_SELECTOR,
                roots.agent_cr3(),
            );
        }
        pit_timer::disarm();

        let frame_rsp = storage::AGENT_KERNEL_AGENT_INTERRUPT_RSP.load(Ordering::Acquire);
        let frame_rip = storage::AGENT_KERNEL_AGENT_INTERRUPT_RIP.load(Ordering::Acquire);
        let frame = validation::read_frame(frame_rsp, self.runtime.kernel_stack)?;
        if storage::AGENT_KERNEL_AGENT_IRQ_COUNT.load(Ordering::Acquire) != 1
            || storage::AGENT_KERNEL_AGENT_IRQ_SEEN.load(Ordering::Acquire) != 1
            || storage::AGENT_KERNEL_AGENT_PREEMPTED.load(Ordering::Acquire) != 1
            || storage::AGENT_KERNEL_HOST_CONTEXT_RSP.load() == 0
            || storage::AGENT_KERNEL_AGENT_INTERRUPT_CR3.load(Ordering::Acquire)
                != roots.agent_cr3()
            || frame.rip != frame_rip
            || !validation::user_frame_valid(&frame, layout)
            || !validation::initial_registers_sanitized(&frame, layout)
            || !validation::kernel_boundary_valid(
                self.runtime.kernel_stack,
                self.runtime.kernel_cr3,
            )
        {
            return None;
        }

        Some(PreemptedAgentCpu {
            memory: self.memory,
            runtime: self.runtime,
            frame: SavedAgentFrame::new(frame),
            context: self.context,
            ticks: 1,
        })
    }
}

impl PreemptedAgentCpu {
    pub(crate) const fn tick_count(&self) -> u8 {
        self.ticks
    }

    pub(crate) fn signal_is_clear(&self) -> bool {
        self.memory.signal_is_clear()
    }

    pub(crate) fn resume_until_yield(mut self) -> Option<YieldedAgentCpu> {
        let roots = self.memory.roots();
        let layout = self.memory.layout();
        storage::begin_dispatch(roots)?;
        if !self.memory.release_for_agent_call() {
            return None;
        }
        call::resume_owned(&mut self.frame, roots, layout)?;

        let describe = call::capture(self.runtime.kernel_stack, roots, layout)?;
        let nonce = match describe.request() {
            AgentCallRequest::DescribeContext { nonce } => nonce,
            AgentCallRequest::Yield { .. } => return None,
        };
        let describe_return_offset = describe.return_offset();
        let mut reply_frame = describe.into_frame();
        self.context
            .encode_describe_reply(reply_frame.frame_mut(), nonce)
            .ok()?;

        storage::begin_dispatch(roots)?;
        call::resume_owned(&mut reply_frame, roots, layout)?;
        let yielded = call::capture(self.runtime.kernel_stack, roots, layout)?;
        if !self.context.matches_yield(yielded.request(), nonce) {
            return None;
        }

        Some(YieldedAgentCpu {
            yields: 1,
            calls: 2,
            address_space_switches: 4,
            describe_return_offset,
            yield_return_offset: yielded.return_offset(),
            nonce,
        })
    }
}

impl YieldedAgentCpu {
    pub(crate) const fn yield_count(&self) -> u8 {
        self.yields
    }

    pub(crate) const fn call_count(&self) -> u8 {
        self.calls
    }

    pub(crate) const fn address_space_switch_count(&self) -> u8 {
        self.address_space_switches
    }

    pub(crate) const fn describe_return_offset(&self) -> u32 {
        self.describe_return_offset
    }

    pub(crate) const fn yield_return_offset(&self) -> u32 {
        self.yield_return_offset
    }

    pub(crate) const fn nonce(&self) -> u64 {
        self.nonce
    }
}
