//! Authorized recovery and immutable history checks for the Fault Worker.
//!
//! This boot-adapter child binds each consumed physical fault to the public
//! rollback and enqueue syscalls. It verifies two ordered semantic fault
//! records and the final completion sequence; exception capture stays in the
//! CPU layer and task creation stays in the parent module.

use agent_kernel_core::{AgentExecutionState, EventKind, FaultId, FaultKind, TaskStatus};
use agent_kernel_x86_64::native_runtime::{NativeAgentFault, INVALID_OPCODE_VECTOR};

use super::{PreparedFaultTaskFlow, FAULT_WORKER};
use crate::{
    native_agent_executor::NativeExecutionReport, native_agent_runtime::NativeAgentRuntime,
    X86BootedKernel,
};

const GENERAL_PROTECTION_DETAIL: u64 =
    NativeAgentFault::GeneralProtection { error_code: 0 }.detail();

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

    pub(crate) fn restart_for_runtime(
        &self,
        booted: &mut X86BootedKernel,
        runtime: &mut NativeAgentRuntime,
        report: &mut NativeExecutionReport,
        expected_fault: NativeAgentFault,
    ) -> Option<()> {
        let expected_generation = match expected_fault {
            NativeAgentFault::InvalidOpcode => 0,
            NativeAgentFault::GeneralProtection { error_code: 0 } => 1,
            NativeAgentFault::GeneralProtection { .. } => return None,
        };
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

    pub(crate) fn completed_after_restarts(&self, booted: &X86BootedKernel) -> bool {
        let kernel = booted.kernel();
        let task = kernel
            .tasks()
            .iter()
            .find(|task| task.id == self.worker.task);
        let context = kernel
            .execution_contexts()
            .iter()
            .find(|context| context.agent == FAULT_WORKER);
        let Some(last_fault) = task.and_then(|task| task.last_fault) else {
            return false;
        };
        matches!(task, Some(task)
            if task.status == TaskStatus::Completed
                && task.assignee == Some(FAULT_WORKER)
                && task.delegated_capability == Some(self.worker.capability)
                && task.run_ticks == 3
                && task.last_fault == Some(last_fault)
                && task.result.is_none())
            && matches!(context, Some(context)
                if context.state == AgentExecutionState::Idle
                    && context.task.is_none()
                    && context.run_ticks == 0
                    && context.quantum_remaining == 0)
            && self.fault_history_valid(booted, 2)
            && kernel
                .faults()
                .last()
                .is_some_and(|record| record.id == last_fault)
            && self.completion_events_ordered(booted)
            && !kernel.run_queue().contains(&self.run_queue_entry())
    }

    fn faulted_state_valid(
        &self,
        booted: &X86BootedKernel,
        expected_fault: NativeAgentFault,
        restart_generation: u8,
    ) -> bool {
        let kernel = booted.kernel();
        let task = kernel
            .tasks()
            .iter()
            .find(|task| task.id == self.worker.task);
        let context = kernel
            .execution_contexts()
            .iter()
            .find(|context| context.agent == FAULT_WORKER);
        let fault = task
            .and_then(|task| task.last_fault)
            .and_then(|id| kernel.faults().iter().find(|fault| fault.id == id));
        let expected_records = usize::from(restart_generation) + 1;
        matches!(task, Some(task)
            if task.status == TaskStatus::Faulted
                && task.assignee == Some(FAULT_WORKER)
                && task.delegated_capability == Some(self.worker.capability)
                && task.run_ticks == u64::from(restart_generation) + 1
                && task.quantum_remaining == 0)
            && matches!(context, Some(context)
                if context.state == AgentExecutionState::Faulted
                    && context.task == Some(self.worker.task)
                    && context.run_ticks == 0
                    && context.quantum_remaining == 0)
            && matches!(fault, Some(fault)
                if fault.agent == FAULT_WORKER
                    && fault.task == self.worker.task
                    && fault.kind == FaultKind::ExecutionTrap
                    && fault.detail == expected_fault.detail())
            && self.fault_history_valid(booted, expected_records)
            && !kernel.run_queue().contains(&self.run_queue_entry())
    }

    fn recovered_state_valid(
        &self,
        booted: &X86BootedKernel,
        fault: FaultId,
        expected_ticks: u64,
        expected_records: usize,
    ) -> bool {
        let kernel = booted.kernel();
        let task = kernel
            .tasks()
            .iter()
            .find(|task| task.id == self.worker.task);
        let context = kernel
            .execution_contexts()
            .iter()
            .find(|context| context.agent == FAULT_WORKER);
        matches!(task, Some(task)
            if task.status == TaskStatus::Accepted
                && task.run_ticks == expected_ticks
                && task.quantum_remaining == 0
                && task.last_fault == Some(fault))
            && matches!(context, Some(context)
                if context.state == AgentExecutionState::Idle
                    && context.task.is_none()
                    && context.run_ticks == 0
                    && context.quantum_remaining == 0)
            && self.fault_history_valid(booted, expected_records)
    }

    fn fault_history_valid(&self, booted: &X86BootedKernel, expected_records: usize) -> bool {
        let records = booted.kernel().faults();
        let expected_details = [u64::from(INVALID_OPCODE_VECTOR), GENERAL_PROTECTION_DETAIL];
        records.len() == expected_records
            && expected_records <= expected_details.len()
            && records.iter().enumerate().all(|(index, record)| {
                record.agent == FAULT_WORKER
                    && record.task == self.worker.task
                    && record.kind == FaultKind::ExecutionTrap
                    && record.detail == expected_details[index]
            })
    }

    fn completion_events_ordered(&self, booted: &X86BootedKernel) -> bool {
        let expected = [
            EventKind::TaskFaulted,
            EventKind::TaskFaultRecovered,
            EventKind::TaskQueued,
            EventKind::TaskFaulted,
            EventKind::TaskFaultRecovered,
            EventKind::TaskQueued,
            EventKind::TaskCompleted,
        ];
        let mut next = 0;
        for event in booted.kernel().events() {
            if next < expected.len()
                && event.task == Some(self.worker.task)
                && event.kind == expected[next]
            {
                next += 1;
            }
        }
        next == expected.len()
    }
}
