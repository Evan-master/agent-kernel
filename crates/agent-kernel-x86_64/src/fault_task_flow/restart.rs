//! Authorized recovery actions for the native Fault Worker.
//!
//! This bare-metal boot-adapter child consumes each terminal CPU fault, invokes
//! the public rollback syscall under bootstrap authority, registers the fresh
//! physical context, and requeues the task. Immutable semantic and transcript
//! checks live in the `evidence` child; exception capture stays in the CPU
//! layer.

mod evidence;

use agent_kernel_core::{EventKind, FaultKind};
use agent_kernel_x86_64::native_runtime::NativeAgentFault;

use super::{
    expected_page_fault, PreparedFaultTaskFlow, FAULT_WORKER, FAULT_WORKER_PAGE_FAULT_ADDRESS,
    FAULT_WORKER_PAGE_FAULT_ERROR_CODE,
};
use crate::{
    native_agent_executor::NativeExecutionReport, native_agent_runtime::NativeAgentRuntime,
    X86BootedKernel,
};

impl PreparedFaultTaskFlow {
    pub(crate) fn invalid_opcode_faulted_after_runtime(&self, booted: &X86BootedKernel) -> bool {
        self.faulted_state_valid(booted, NativeAgentFault::InvalidOpcode, 0)
    }

    pub(crate) fn general_protection_faulted_after_runtime(
        &self,
        booted: &X86BootedKernel,
    ) -> bool {
        self.faulted_state_valid(
            booted,
            NativeAgentFault::GeneralProtection { error_code: 0 },
            1,
        )
    }

    pub(crate) fn page_faulted_after_runtime(&self, booted: &X86BootedKernel) -> bool {
        self.faulted_state_valid(booted, expected_page_fault(), 2)
    }

    pub(crate) fn restart_for_runtime(
        &self,
        booted: &mut X86BootedKernel,
        runtime: &mut NativeAgentRuntime,
        report: &mut NativeExecutionReport,
        expected_fault: NativeAgentFault,
    ) -> Option<()> {
        let expected_generation = expected_restart_generation(expected_fault)?;
        if !runtime.is_empty()
            || report.faulted_len() != 1
            || !self.faulted_state_valid(booted, expected_fault, expected_generation)
        {
            return None;
        }
        let context = self.call_context()?;
        let fault = booted
            .kernel()
            .tasks()
            .iter()
            .find(|task| task.id == self.worker.task)?
            .last_fault?;
        let faulted = report.take_faulted(FAULT_WORKER)?;
        if faulted.context() != context
            || faulted.fault() != expected_fault
            || faulted.restart_generation() != expected_generation
        {
            return None;
        }
        let prepared = faulted.restart()?;
        let authority = *booted.report();
        let event = booted
            .kernel_mut()
            .sys_recover_faulted_task(
                authority.bootstrap_agent,
                authority.bootstrap_capability,
                self.worker.task,
            )
            .ok()?;
        let expected_records = usize::from(expected_generation) + 1;
        let expected_ticks = u64::from(expected_generation) + 1;
        if event.kind != EventKind::TaskFaultRecovered
            || event.agent != authority.bootstrap_agent
            || event.capability != Some(authority.bootstrap_capability)
            || event.task != Some(self.worker.task)
            || event.fault != Some(fault)
            || event.fault_kind != Some(FaultKind::ExecutionTrap)
            || event.fault_detail != Some(expected_fault.detail())
            || !self.recovered_state_valid(booted, fault, expected_ticks, expected_records)
            || runtime.register_prepared(prepared).is_some()
        {
            return None;
        }
        self.queue_for_runtime(booted)?;
        (self.recovered_state_valid(booted, fault, expected_ticks, expected_records)
            && booted.kernel().run_queue() == [self.run_queue_entry()])
        .then_some(())
    }
}

const fn expected_restart_generation(fault: NativeAgentFault) -> Option<u8> {
    match fault {
        NativeAgentFault::InvalidOpcode => Some(0),
        NativeAgentFault::GeneralProtection { error_code: 0 } => Some(1),
        NativeAgentFault::PageFault {
            error_code: FAULT_WORKER_PAGE_FAULT_ERROR_CODE,
            address,
        } if address == FAULT_WORKER_PAGE_FAULT_ADDRESS => Some(2),
        NativeAgentFault::GeneralProtection { .. } | NativeAgentFault::PageFault { .. } => None,
    }
}
