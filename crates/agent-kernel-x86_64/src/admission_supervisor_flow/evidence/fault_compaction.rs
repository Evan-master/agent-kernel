//! Full Fault Store retirement and complete Event evidence.

use agent_kernel_core::{Event, EventKind, FaultId, Operation};

use crate::{
    admission_supervisor_flow::{PreparedAdmissionSupervisorFlow, ADMISSION_SUPERVISOR},
    X86BootedKernel, X86_FAULT_CAPACITY,
};

impl PreparedAdmissionSupervisorFlow {
    pub(super) fn fault_store_compacted(&self, booted: &X86BootedKernel) -> bool {
        let kernel = booted.kernel();
        let mut events = kernel
            .events()
            .iter()
            .filter(|event| event.kind == EventKind::FaultCompacted);
        let Some(first) = events.next() else {
            return false;
        };
        let mut compacted = [first; X86_FAULT_CAPACITY];
        for slot in &mut compacted[1..] {
            let Some(event) = events.next() else {
                return false;
            };
            *slot = event;
        }

        X86_FAULT_CAPACITY == 4
            && kernel.faults().is_empty()
            && events.next().is_none()
            && compacted.iter().enumerate().all(|(index, event)| {
                event.sequence == 358 + index as u64
                    && proves_compaction_event(
                        booted,
                        event,
                        FaultId::new(index as u64 + 1),
                        self.supervisor.admission_authority,
                    )
            })
    }
}

fn proves_compaction_event(
    booted: &X86BootedKernel,
    compacted: &Event,
    fault: FaultId,
    authority: agent_kernel_core::CapabilityId,
) -> bool {
    let Some(source) = source_event(booted, fault) else {
        return false;
    };
    compacted.agent == ADMISSION_SUPERVISOR
        && compacted.target_agent == Some(source.agent)
        && compacted.resource == source.resource
        && compacted.capability == Some(authority)
        && compacted.operation == Some(Operation::Rollback)
        && compacted.task == source.task
        && compacted.fault == Some(fault)
        && compacted.fault_kind == source.fault_kind
        && compacted.fault_detail == source.fault_detail
        && source.resource.is_some()
        && source.task.is_some()
        && source.fault_kind.is_some()
        && source.fault_detail.is_some()
}

fn source_event(booted: &X86BootedKernel, fault: FaultId) -> Option<&Event> {
    booted
        .kernel()
        .events()
        .iter()
        .find(|event| event.kind == EventKind::TaskFaulted && event.fault == Some(fault))
}
