//! Terminal ownership for one validated ring-3 Agent exception.
//!
//! This CPU-layer module copies a supported #UD, #GP, or #PF frame from RSP0,
//! binds it to the owning Agent address space and call session, validates CPU
//! error and fault-address evidence, and makes the captured context
//! non-resumable. A restart consumes and discards that frame before constructing
//! a fresh entry context. Semantic mutation remains in the executor.

use core::sync::atomic::Ordering;

use agent_kernel_core::MemoryCellId;
use agent_kernel_x86_64::{
    agent_call::AgentCallContext,
    context::SavedAgentFrame,
    native_runtime::{NativeAgentFault, NativeRunBoundary},
    runtime_reclamation::RuntimeReclamationLog,
};

mod reclamation;

use super::{
    native_call_session::{AgentCallProgress, ResumableAgentCpu},
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
    progress: AgentCallProgress,
    reclamation: RuntimeReclamationLog,
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
        let expected_error_code = u64::from(expected_fault.error_code());
        let expected_address = expected_fault.fault_address().unwrap_or(0);
        let frame = match expected_fault {
            NativeAgentFault::InvalidOpcode => {
                validation::read_frame(frame_rsp, runtime.kernel_stack)?
            }
            NativeAgentFault::GeneralProtection { .. } | NativeAgentFault::PageFault { .. } => {
                let frame = validation::read_error_code_frame(frame_rsp, runtime.kernel_stack)?;
                if frame.error_code() != expected_error_code {
                    return None;
                }
                frame.without_error_code()
            }
        };
        if storage::run_boundary()? != NativeRunBoundary::AgentFault(expected_fault)
            || storage::AGENT_KERNEL_HOST_CONTEXT_RSP.load() == 0
            || storage::AGENT_KERNEL_AGENT_FAULT_CR3.load(Ordering::Acquire) != roots.agent_cr3()
            || storage::AGENT_KERNEL_AGENT_FAULT_ERROR_CODE.load(Ordering::Acquire)
                != expected_error_code
            || storage::AGENT_KERNEL_AGENT_FAULT_ADDRESS.load(Ordering::Acquire) != expected_address
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
            progress,
            reclamation: RuntimeReclamationLog::new(),
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

    pub(crate) fn had_call_progress(&self) -> bool {
        !self.progress.is_empty()
    }

    pub(crate) const fn call_nonce(&self) -> Option<u64> {
        self.progress.nonce()
    }

    pub(crate) const fn call_count(&self) -> usize {
        self.progress.call_count()
    }

    pub(crate) fn operations(&self) -> &[agent_kernel_x86_64::agent_call::AgentCallOperation] {
        self.progress.operations()
    }

    pub(crate) fn return_offsets(&self) -> &[u32] {
        self.progress.return_offsets()
    }

    pub(crate) fn physical_quantum_generation(&self) -> u8 {
        self.memory.physical_quantum_generation()
    }

    pub(crate) fn restart_generation(&self) -> u8 {
        self.memory.restart_generation()
    }

    pub(crate) fn runtime_memory_is_clear(&self) -> bool {
        self.memory.runtime_memory_is_clear()
    }

    pub(crate) fn references_memory_cell(&self, cell: MemoryCellId) -> bool {
        self.memory.references_memory_cell(cell)
    }

    pub(crate) fn restart(self) -> Option<PreparedAgentCpu> {
        let Self {
            memory,
            runtime,
            context,
            ..
        } = self;
        let (memory, restart_generation) = memory.reset_for_next_restart()?;
        runtime.prepare_restarted(memory, context, restart_generation)
    }

    pub(crate) fn repair_lazy_page(self) -> Option<ResumableAgentCpu> {
        let Self {
            mut memory,
            runtime,
            frame,
            context,
            fault,
            progress,
            reclamation,
        } = self;
        let NativeAgentFault::PageFault {
            error_code: 6,
            address,
        } = fault
        else {
            return None;
        };
        if !reclamation.is_empty() {
            return None;
        }
        memory.activate_lazy_data_page(address)?;
        Some(ResumableAgentCpu::from_repaired_fault(
            memory, runtime, frame, context, progress,
        ))
    }
}
