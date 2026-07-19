//! Audited Supervisor handler for native Fault Store compaction.
//!
//! The handler snapshots bounded Fault and Task state, invokes the public
//! facade, and validates dense removal, Task reference cleanup, and complete
//! ordered Event evidence before resuming the ring-3 caller.

use agent_kernel_core::{CapabilityId, EventKind, FaultId, FaultRecord, Operation, Task};

use super::super::state;
use crate::{
    agent_cpu::{PendingAgentCallCpu, ResumableAgentCpu},
    serial_write_line, X86BootedKernel, X86_FAULT_CAPACITY, X86_TASK_CAPACITY,
};

pub(super) fn compact(
    booted: &mut X86BootedKernel,
    pending: PendingAgentCallCpu,
    authority: CapabilityId,
    through: FaultId,
) -> Option<ResumableAgentCpu> {
    pending.authenticated_request()?;
    let context = pending.context();
    let fault_len = booted.kernel().faults().len();
    let task_len = booted.kernel().tasks().len();
    if fault_len > X86_FAULT_CAPACITY || task_len > X86_TASK_CAPACITY {
        return None;
    }
    let mut previous_faults: [Option<FaultRecord>; X86_FAULT_CAPACITY] = [None; X86_FAULT_CAPACITY];
    for (index, record) in booted.kernel().faults().iter().copied().enumerate() {
        previous_faults[index] = Some(record);
    }
    let mut previous_tasks: [Option<Task>; X86_TASK_CAPACITY] = [None; X86_TASK_CAPACITY];
    for (index, task) in booted.kernel().tasks().iter().copied().enumerate() {
        previous_tasks[index] = Some(task);
    }
    let through_index = booted
        .kernel()
        .faults()
        .iter()
        .position(|record| record.id == through)?;
    let expected_count = through_index + 1;
    let event_start = booted.kernel().events().len();
    let queue_len = booted.kernel().run_queue().len();

    let receipt = booted
        .kernel_mut()
        .sys_compact_fault_prefix(context.agent(), authority, through)
        .ok()?;
    let kernel = booted.kernel();
    let events = kernel.events().get(event_start..)?;
    let selected_contains = |fault: FaultId| {
        previous_faults[..expected_count]
            .iter()
            .flatten()
            .any(|record| record.id == fault)
    };
    if receipt.first() != previous_faults[0]?.id
        || receipt.through() != through
        || receipt.count() != expected_count
        || kernel.faults().len() + receipt.count() != fault_len
        || kernel.tasks().len() != task_len
        || kernel.run_queue().len() != queue_len
        || kernel.faults().iter().enumerate().any(|(index, record)| {
            previous_faults
                .get(index + receipt.count())
                .copied()
                .flatten()
                != Some(*record)
        })
        || kernel.tasks().iter().enumerate().any(|(index, task)| {
            let Some(mut expected) = previous_tasks[index] else {
                return true;
            };
            if expected.last_fault.is_some_and(selected_contains) {
                expected.last_fault = None;
            }
            expected != *task
        })
        || events.len() != receipt.count()
        || events.iter().enumerate().any(|(index, event)| {
            let Some(record) = previous_faults[index] else {
                return true;
            };
            event.sequence != (event_start + index + 1) as u64
                || event.kind != EventKind::FaultCompacted
                || event.agent != context.agent()
                || event.target_agent != Some(record.agent)
                || event.resource != Some(record.resource)
                || event.capability != Some(authority)
                || event.operation != Some(Operation::Rollback)
                || event.task != Some(record.task)
                || event.fault != Some(record.id)
                || event.fault_kind != Some(record.kind)
                || event.fault_detail != Some(record.detail)
        })
        || !state::running(booted, context)
    {
        return None;
    }

    serial_write_line("AGENT_KERNEL_AGENT_CALL_FAULT_COMPACTION_OK");
    pending.acknowledge_fault_compaction(receipt)
}
