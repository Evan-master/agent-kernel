//! Terminal ownership for one validated ring-3 Agent exception.
//!
//! This CPU-layer module copies the no-error-code #UD frame from RSP0, binds
//! it to the owning Agent address space and call session, and makes the captured
//! context non-resumable. A restart consumes and discards that frame before
//! constructing a fresh entry context. Semantic mutation remains in the executor.

use core::sync::atomic::Ordering;

use agent_kernel_x86_64::{
    agent_call::AgentCallContext,
    context::SavedAgentFrame,
    native_runtime::{NativeAgentFault, NativeRunBoundary},
};

use super::{
    native_call_session::AgentCallProgress,
    runtime::{AgentCpuRuntime, PreparedAgentCpu},
    storage, validation,
};
use crate::agent_memory::PreparedAgentMemory;

pub(crate) struct FaultedAgentCpu {
    memory: PreparedAgentMemory,
    runtime: AgentCpuRuntime,
    frame: SavedAgentFrame,
    context: AgentCallContext,
    fault: NativeAgentFault,
    had_call_progress: bool,
}

impl FaultedAgentCpu {
    pub(super) fn capture(
        memory: PreparedAgentMemory,
        runtime: AgentCpuRuntime,
        context: AgentCallContext,
        progress: AgentCallProgress,
        expected_fault: NativeAgentFault,
    ) -> Option<Self> {
        let roots = memory.roots();
        let layout = memory.layout();
        let frame_rsp = storage::AGENT_KERNEL_AGENT_FAULT_RSP.load(Ordering::Acquire);
        let frame_rip = storage::AGENT_KERNEL_AGENT_FAULT_RIP.load(Ordering::Acquire);
        let frame = validation::read_frame(frame_rsp, runtime.kernel_stack)?;
        if storage::run_boundary()? != NativeRunBoundary::AgentFault(expected_fault)
            || storage::AGENT_KERNEL_HOST_CONTEXT_RSP.load() == 0
            || storage::AGENT_KERNEL_AGENT_FAULT_CR3.load(Ordering::Acquire) != roots.agent_cr3()
            || frame.rip != frame_rip
            || !validation::user_frame_valid(&frame, layout)
            || !validation::kernel_boundary_valid(runtime.kernel_stack, runtime.kernel_cr3)
        {
            return None;
        }
        frame.rip.checked_sub(memory.entry_rip())?;

        Some(Self {
            memory,
            runtime,
            frame: SavedAgentFrame::new(frame),
            context,
            fault: expected_fault,
            had_call_progress: !progress.is_empty(),
        })
    }

    pub(crate) const fn context(&self) -> AgentCallContext {
        self.context
    }

    pub(crate) const fn fault(&self) -> NativeAgentFault {
        self.fault
    }

    pub(crate) fn fault_offset(&self) -> Option<u32> {
        let offset = self
            .frame
            .frame()
            .rip
            .checked_sub(self.memory.entry_rip())?;
        u32::try_from(offset).ok()
    }

    pub(crate) const fn had_call_progress(&self) -> bool {
        self.had_call_progress
    }

    pub(crate) fn physical_quantum_generation(&self) -> u8 {
        self.memory.physical_quantum_generation()
    }

    pub(crate) fn restart_generation(&self) -> u8 {
        self.memory.restart_generation()
    }

    pub(crate) fn restart(self) -> Option<PreparedAgentCpu> {
        let Self {
            memory,
            runtime,
            context,
            ..
        } = self;
        runtime.prepare_restarted(memory.reset_for_first_restart()?, context)
    }
}
