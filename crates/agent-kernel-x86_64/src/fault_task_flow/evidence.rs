//! Shared semantic state and immutable transcript proof for Fault Worker recovery.
//!
//! This boot-adapter child validates task/context states, all four ordered
//! execution-trap records, and the final recovery/completion sequence. Restart
//! and page-repair siblings perform mutations; this module reads only public
//! kernel state and fixed Fault Worker contracts.

use agent_kernel_core::{AgentExecutionState, EventKind, FaultId, FaultKind, TaskStatus};
use agent_kernel_x86_64::native_runtime::{NativeAgentFault, INVALID_OPCODE_VECTOR};

use super::{expected_lazy_page_fault, expected_page_fault, PreparedFaultTaskFlow, FAULT_WORKER};
use crate::X86BootedKernel;

const GENERAL_PROTECTION_DETAIL: u64 =
    NativeAgentFault::GeneralProtection { error_code: 0 }.detail();
const PAGE_FAULT_DETAIL: u64 = expected_page_fault().detail();
const LAZY_PAGE_FAULT_DETAIL: u64 = expected_lazy_page_fault().detail();

impl PreparedFaultTaskFlow {
    pub(crate) fn completed_after_fault_recovery(&self, booted: &X86BootedKernel) -> bool {
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
                && task.run_ticks == 4
                && task.last_fault == Some(last_fault)
                && task.result.is_none())
            && matches!(context, Some(context)
                if context.state == AgentExecutionState::Idle
                    && context.task.is_none()
                    && context.run_ticks == 0
                    && context.quantum_remaining == 0)
            && self.fault_history_valid(booted, 4)
            && kernel
                .faults()
                .last()
                .is_some_and(|record| record.id == last_fault)
            && self.completion_events_ordered(booted)
            && !kernel.run_queue().contains(&self.run_queue_entry())
    }

    pub(super) fn faulted_state_valid(
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

    pub(super) fn recovered_state_valid(
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
        let expected_details = [
            u64::from(INVALID_OPCODE_VECTOR),
            GENERAL_PROTECTION_DETAIL,
            PAGE_FAULT_DETAIL,
            LAZY_PAGE_FAULT_DETAIL,
        ];
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
