//! Type-state runtime for multiple suspended ring-3 Agent contexts.
//!
//! One installed CPU boundary resets evidence for each physical dispatch. Every
//! preempted context owns a copied privilege frame, so the shared TSS RSP0 stack
//! can accept another Agent interrupt before the first context resumes.

use core::sync::atomic::Ordering;

use agent_kernel_x86_64::{
    address_space::AddressSpaceRoots,
    agent_call::AgentCallContext,
    context::SavedAgentFrame,
    interrupt::AGENT_CALL_VECTOR,
    native_runtime::NativeRunBoundary,
    privilege::{USER_CODE_SELECTOR, USER_DATA_SELECTOR},
};

use super::{assembly, native_call_session::AgentCallProgress, storage, validation};
use crate::{
    agent_memory::PreparedAgentMemory,
    exception_runtime, pit_timer,
    privilege_runtime::{
        current_privilege_level, stack_canary_valid, PrivilegeBoundary, PrivilegedStackBounds,
    },
};

#[derive(Copy, Clone)]
pub(crate) struct AgentCpuRuntime {
    pub(super) kernel_stack: PrivilegedStackBounds,
    pub(super) kernel_cr3: u64,
}

pub(crate) struct PreparedAgentCpu {
    memory: PreparedAgentMemory,
    runtime: AgentCpuRuntime,
    context: AgentCallContext,
}

pub(crate) struct PreemptedAgentCpu {
    pub(super) memory: PreparedAgentMemory,
    pub(super) runtime: AgentCpuRuntime,
    pub(super) frame: SavedAgentFrame,
    pub(super) context: AgentCallContext,
    pub(super) progress: AgentCallProgress,
    ticks: u8,
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
    pub(crate) const fn context(&self) -> AgentCallContext {
        self.context
    }

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

        PreemptedAgentCpu::capture(
            self.memory,
            self.runtime,
            self.context,
            AgentCallProgress::new(),
            true,
        )
    }
}

impl PreemptedAgentCpu {
    pub(super) fn capture(
        mut memory: PreparedAgentMemory,
        runtime: AgentCpuRuntime,
        context: AgentCallContext,
        progress: AgentCallProgress,
        require_initial_registers: bool,
    ) -> Option<Self> {
        let roots = memory.roots();
        let layout = memory.layout();
        let frame_rsp = storage::AGENT_KERNEL_AGENT_INTERRUPT_RSP.load(Ordering::Acquire);
        let frame_rip = storage::AGENT_KERNEL_AGENT_INTERRUPT_RIP.load(Ordering::Acquire);
        let frame = validation::read_frame(frame_rsp, runtime.kernel_stack)?;
        if storage::run_boundary()? != NativeRunBoundary::QuantumExpired
            || storage::AGENT_KERNEL_HOST_CONTEXT_RSP.load() == 0
            || storage::AGENT_KERNEL_AGENT_INTERRUPT_CR3.load(Ordering::Acquire)
                != roots.agent_cr3()
            || frame.rip != frame_rip
            || !validation::user_frame_valid(&frame, layout)
            || (require_initial_registers
                && !validation::initial_registers_sanitized(&frame, layout))
            || !validation::kernel_boundary_valid(runtime.kernel_stack, runtime.kernel_cr3)
        {
            return None;
        }
        memory.record_physical_quantum_expiry()?;

        Some(Self {
            memory,
            runtime,
            frame: SavedAgentFrame::new(frame),
            context,
            progress,
            ticks: 1,
        })
    }

    pub(crate) const fn tick_count(&self) -> u8 {
        self.ticks
    }

    pub(crate) const fn context(&self) -> AgentCallContext {
        self.context
    }

    pub(crate) const fn has_call_progress(&self) -> bool {
        !self.progress.is_empty()
    }

    pub(crate) fn physical_quantum_generation(&self) -> u8 {
        self.memory.physical_quantum_generation()
    }
}
