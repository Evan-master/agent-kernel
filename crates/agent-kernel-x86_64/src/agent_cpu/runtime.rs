//! Type-state runtime for multiple suspended ring-3 Agent contexts.
//!
//! One installed CPU boundary resets evidence for each physical dispatch. Every
//! preempted context owns a copied privilege frame, so the shared TSS RSP0 stack
//! can accept another Agent interrupt before the first context resumes.

use agent_kernel_core::MemoryCellId;
use agent_kernel_x86_64::{
    address_space::AddressSpaceRoots,
    agent_call::AgentCallContext,
    apic::{LocalApicBase, LocalApicMmio, VolatileMmio, APIC_RESCHEDULE_VECTOR},
    context::SavedAgentFrame,
    cpu::CpuIndex,
    interrupt::AGENT_CALL_VECTOR,
    native_runtime::NativeRunBoundary,
    per_cpu::CpuTransitionStorage,
    privilege::{USER_CODE_SELECTOR, USER_DATA_SELECTOR},
};

use super::{
    assembly,
    native_call_session::{AgentCallProgress, AgentRunOutcome},
    storage, validation, FaultedAgentCpu,
};
use crate::{
    agent_memory::PreparedAgentMemory,
    exception_runtime, pit_timer,
    privilege_runtime::{
        current_privilege_level, stack_canary_valid, PrivilegeBoundary, PrivilegedStackBounds,
    },
};

#[derive(Copy, Clone)]
pub(crate) struct AgentCpuRuntime {
    cpu: CpuIndex,
    pub(super) kernel_stack: PrivilegedStackBounds,
    pub(super) kernel_cr3: u64,
    pub(super) transition: &'static CpuTransitionStorage,
    timer: CpuQuantumTimer,
}

#[derive(Copy, Clone)]
enum CpuQuantumTimer {
    LegacyPit,
    LocalApic {
        base: LocalApicBase,
        physical_offset: u64,
        initial_count: u32,
    },
}

pub(crate) struct PreparedAgentCpu {
    pub(super) memory: PreparedAgentMemory,
    runtime: AgentCpuRuntime,
    context: AgentCallContext,
}

pub(crate) enum AgentCpuPreparation {
    Prepared(PreparedAgentCpu),
    Rejected(PreparedAgentMemory),
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
    pub(crate) fn install(
        privilege: &PrivilegeBoundary,
        roots: AddressSpaceRoots,
        cpu: CpuIndex,
    ) -> Option<Self> {
        if privilege.cpu() != cpu {
            return None;
        }
        let transition = storage::install(roots, cpu)?;
        let kernel_stack = privilege.stack_bounds();
        if current_privilege_level() != 0 || !stack_canary_valid(kernel_stack) {
            return None;
        }
        // SAFETY: installation holds IF clear and writes the one bounded DPL3
        // Agent-call gate used by every context on this boot CPU.
        unsafe {
            exception_runtime::install_agent_exception_gate(
                agent_kernel_x86_64::native_runtime::INVALID_OPCODE_VECTOR,
                assembly::agent_kernel_agent_invalid_opcode_stub,
            )?;
            exception_runtime::install_agent_exception_gate(
                agent_kernel_x86_64::native_runtime::GENERAL_PROTECTION_VECTOR,
                assembly::agent_kernel_agent_general_protection_stub,
            )?;
            exception_runtime::install_agent_exception_gate(
                agent_kernel_x86_64::native_runtime::PAGE_FAULT_VECTOR,
                assembly::agent_kernel_agent_page_fault_stub,
            )?;
            exception_runtime::install_user_interrupt_gate(
                AGENT_CALL_VECTOR,
                assembly::agent_kernel_agent_call_stub,
            )?;
            exception_runtime::install_irq_gate(
                APIC_RESCHEDULE_VECTOR.get(),
                assembly::agent_kernel_agent_apic_timer_stub,
            )?;
        }
        pit_timer::install_gate(assembly::agent_kernel_agent_timer_irq_stub)?;
        Some(Self {
            cpu,
            kernel_stack,
            kernel_cr3: roots.kernel_cr3(),
            transition,
            timer: CpuQuantumTimer::LegacyPit,
        })
    }

    pub(crate) fn attach_application_processor(
        privilege: &PrivilegeBoundary,
        transition: &'static CpuTransitionStorage,
        kernel_cr3: u64,
        cpu: CpuIndex,
        local_apic_base: LocalApicBase,
        physical_offset: u64,
        initial_count: u32,
    ) -> Option<Self> {
        let kernel_stack = privilege.stack_bounds();
        if privilege.cpu() != cpu
            || transition.kernel_cr3() != kernel_cr3
            || initial_count == 0
            || current_privilege_level() != 0
            || !stack_canary_valid(kernel_stack)
        {
            return None;
        }
        LocalApicMmio::new(local_apic_base, physical_offset, VolatileMmio)?;
        Some(Self {
            cpu,
            kernel_stack,
            kernel_cr3,
            transition,
            timer: CpuQuantumTimer::LocalApic {
                base: local_apic_base,
                physical_offset,
                initial_count,
            },
        })
    }

    pub(crate) const fn cpu(self) -> CpuIndex {
        self.cpu
    }

    pub(super) fn accepts_memory(self, memory: &PreparedAgentMemory) -> bool {
        memory.roots().kernel_cr3() == self.kernel_cr3
            && self.transition.kernel_cr3() == self.kernel_cr3
            && memory.kernel_address_space_active()
            && current_privilege_level() == 0
            && stack_canary_valid(self.kernel_stack)
    }

    pub(super) fn arm_quantum_timer(self) -> Option<()> {
        match self.timer {
            CpuQuantumTimer::LegacyPit => pit_timer::arm(),
            CpuQuantumTimer::LocalApic {
                base,
                physical_offset,
                initial_count,
            } => LocalApicMmio::new(base, physical_offset, VolatileMmio)?
                .arm_timer_one_shot(APIC_RESCHEDULE_VECTOR, initial_count),
        }
    }

    pub(super) fn finish_quantum_timer(self, boundary: Option<NativeRunBoundary>) {
        match self.timer {
            CpuQuantumTimer::LegacyPit => pit_timer::disarm(),
            CpuQuantumTimer::LocalApic {
                base,
                physical_offset,
                ..
            } => {
                if let Some(mut apic) = LocalApicMmio::new(base, physical_offset, VolatileMmio) {
                    apic.mask_timer(APIC_RESCHEDULE_VECTOR);
                    if boundary == Some(NativeRunBoundary::QuantumExpired) {
                        apic.end_of_interrupt();
                    }
                }
            }
        }
    }

    pub(crate) fn prepare(
        &self,
        memory: PreparedAgentMemory,
        context: AgentCallContext,
    ) -> Option<PreparedAgentCpu> {
        match self.prepare_owned(memory, context) {
            AgentCpuPreparation::Prepared(cpu) => Some(cpu),
            AgentCpuPreparation::Rejected(_) => None,
        }
    }

    pub(crate) fn prepare_owned(
        &self,
        memory: PreparedAgentMemory,
        context: AgentCallContext,
    ) -> AgentCpuPreparation {
        self.prepare_with_restart_generation(memory, context, 0)
    }

    pub(super) fn prepare_restarted(
        &self,
        memory: PreparedAgentMemory,
        context: AgentCallContext,
        expected_restart_generation: u8,
    ) -> Option<PreparedAgentCpu> {
        if expected_restart_generation == 0
            || expected_restart_generation
                > agent_kernel_x86_64::user_memory::MAX_AGENT_RESTART_GENERATION
        {
            return None;
        }
        match self.prepare_with_restart_generation(memory, context, expected_restart_generation) {
            AgentCpuPreparation::Prepared(cpu) => Some(cpu),
            AgentCpuPreparation::Rejected(_) => None,
        }
    }

    fn prepare_with_restart_generation(
        &self,
        memory: PreparedAgentMemory,
        context: AgentCallContext,
        expected_restart_generation: u8,
    ) -> AgentCpuPreparation {
        if memory.roots().kernel_cr3() != self.kernel_cr3
            || !memory.kernel_address_space_active()
            || !memory.dispatch_signals_are_clear()
            || memory.restart_generation() != expected_restart_generation
            || !memory.allocation_matches(context.agent())
            || !stack_canary_valid(self.kernel_stack)
        {
            return AgentCpuPreparation::Rejected(memory);
        }
        AgentCpuPreparation::Prepared(PreparedAgentCpu {
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

    pub(crate) fn references_memory_cell(&self, cell: MemoryCellId) -> bool {
        self.memory.references_memory_cell(cell)
    }

    pub(crate) const fn runtime(&self) -> AgentCpuRuntime {
        self.runtime
    }

    pub(crate) fn rebind_runtime(mut self, runtime: AgentCpuRuntime) -> Option<Self> {
        if !runtime.accepts_memory(&self.memory) {
            return None;
        }
        self.runtime = runtime;
        Some(self)
    }

    pub(crate) fn run_until_boundary(self) -> Option<AgentRunOutcome> {
        let roots = self.memory.roots();
        storage::begin_dispatch(self.runtime.transition, roots)?;
        self.runtime.arm_quantum_timer()?;
        let layout = self.memory.layout();
        // SAFETY: private Agent pages, shared supervisor mappings, RSP0, gates,
        // and the per-dispatch evidence mailbox are all validated.
        unsafe {
            assembly::enter_user(
                self.runtime.transition.host_rsp_pointer(),
                self.memory.entry_rip(),
                layout.stack_top(),
                USER_CODE_SELECTOR,
                USER_DATA_SELECTOR,
                roots.agent_cr3(),
            );
        }
        let boundary = self.runtime.transition.run_boundary();
        self.runtime.finish_quantum_timer(boundary);
        match boundary? {
            NativeRunBoundary::QuantumExpired => {
                Some(AgentRunOutcome::Preempted(PreemptedAgentCpu::capture(
                    self.memory,
                    self.runtime,
                    self.context,
                    AgentCallProgress::new(),
                    true,
                )?))
            }
            NativeRunBoundary::AgentFault(fault) => {
                Some(AgentRunOutcome::Fault(FaultedAgentCpu::capture(
                    self.memory,
                    self.runtime,
                    self.context,
                    AgentCallProgress::new(),
                    fault,
                )?))
            }
            NativeRunBoundary::AgentCall => None,
        }
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
        let frame_rsp = runtime.transition.interrupt_rsp();
        let frame_rip = runtime.transition.interrupt_rip();
        let frame = validation::read_frame(frame_rsp, runtime.kernel_stack)?;
        if runtime.transition.run_boundary()? != NativeRunBoundary::QuantumExpired
            || runtime.transition.host_rsp() == 0
            || runtime.transition.interrupt_cr3() != roots.agent_cr3()
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

    pub(crate) const fn runtime(&self) -> AgentCpuRuntime {
        self.runtime
    }

    pub(crate) fn rebind_runtime(mut self, runtime: AgentCpuRuntime) -> Option<Self> {
        if !runtime.accepts_memory(&self.memory) {
            return None;
        }
        self.runtime = runtime;
        Some(self)
    }

    pub(crate) const fn tick_count(&self) -> u8 {
        self.ticks
    }

    pub(crate) const fn context(&self) -> AgentCallContext {
        self.context
    }

    pub(crate) fn references_memory_cell(&self, cell: MemoryCellId) -> bool {
        self.memory.references_memory_cell(cell)
    }

    pub(crate) const fn has_call_progress(&self) -> bool {
        !self.progress.is_empty()
    }

    pub(crate) fn physical_quantum_generation(&self) -> u8 {
        self.memory.physical_quantum_generation()
    }
}
