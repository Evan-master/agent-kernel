//! Authorized lazy-page repair and same-frame requeue for the Fault Worker.
//!
//! This boot-adapter child consumes the one exact not-present physical fault,
//! activates its retained data frame, records public rollback-authorized
//! recovery, parks the normalized frame as `RecoveredFault`, and requeues the
//! semantic task. General paging policy and exception capture remain outside.

use agent_kernel_core::{EventKind, FaultKind};

use super::{expected_lazy_page_fault, PreparedFaultTaskFlow, FAULT_WORKER};
use crate::{
    native_agent_executor::NativeExecutionReport, native_agent_runtime::NativeAgentRuntime,
    X86BootedKernel,
};

impl PreparedFaultTaskFlow {
    pub(crate) fn lazy_page_faulted_after_runtime(&self, booted: &X86BootedKernel) -> bool {
        self.faulted_state_valid(booted, expected_lazy_page_fault(), 3)
    }

    pub(crate) fn repair_page_for_runtime(
        &self,
        booted: &mut X86BootedKernel,
        runtime: &mut NativeAgentRuntime,
        report: &mut NativeExecutionReport,
    ) -> Option<()> {
        let expected_fault = expected_lazy_page_fault();
        if !runtime.is_empty()
            || report.faulted_len() != 1
            || !self.faulted_state_valid(booted, expected_fault, 3)
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
            || faulted.restart_generation() != 3
            || faulted.had_call_progress()
        {
            return None;
        }
        let repaired = faulted.repair_lazy_page()?;
        let authority = *booted.report();
        let event = booted
            .kernel_mut()
            .sys_recover_faulted_task(
                authority.bootstrap_agent,
                authority.bootstrap_capability,
                self.worker.task,
            )
            .ok()?;
        if event.kind != EventKind::TaskFaultRecovered
            || event.agent != authority.bootstrap_agent
            || event.capability != Some(authority.bootstrap_capability)
            || event.task != Some(self.worker.task)
            || event.fault != Some(fault)
            || event.fault_kind != Some(FaultKind::ExecutionTrap)
            || event.fault_detail != Some(expected_fault.detail())
            || !self.recovered_state_valid(booted, fault, 4, 4)
            || runtime.park_recovered_fault(repaired).is_some()
        {
            return None;
        }
        self.queue_for_runtime(booted)?;
        (self.recovered_state_valid(booted, fault, 4, 4)
            && booted.kernel().run_queue() == [self.run_queue_entry()])
        .then_some(())
    }
}
