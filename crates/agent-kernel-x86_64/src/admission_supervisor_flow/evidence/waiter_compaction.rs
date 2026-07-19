//! Dense Waiter Store reuse and complete retirement Event proof.

use agent_kernel_core::{Event, EventKind, Operation, SignalKey, WaiterId, WaiterKind};

use crate::{
    admission_supervisor_flow::{PreparedAdmissionSupervisorFlow, ADMISSION_SUPERVISOR},
    X86BootedKernel, X86_WAITER_CAPACITY,
};

impl PreparedAdmissionSupervisorFlow {
    pub(super) fn first_waiter_prefix_compacted(&self, booted: &X86BootedKernel) -> bool {
        let kernel = booted.kernel();
        let mut events = kernel
            .events()
            .iter()
            .filter(|event| event.kind == EventKind::WaiterCompacted);
        let Some(first) = events.next() else {
            return false;
        };
        let Some(second) = events.next() else {
            return false;
        };
        let Some(third) = events.next() else {
            return false;
        };
        let compacted = [first, second, third];
        let waiter = kernel.waiters().first();

        X86_WAITER_CAPACITY == 3
            && events.next().is_none()
            && compacted.iter().enumerate().all(|(index, event)| {
                event.sequence == first.sequence + index as u64
                    && proves_compaction_event(
                        booted,
                        event,
                        WaiterId::new(index as u64 + 1),
                        self.supervisor.admission_authority,
                    )
            })
            && matches!(kernel.waiters(), [record]
                if record.id == WaiterId::new(4)
                    && record.agent == ADMISSION_SUPERVISOR
                    && record.task == self.supervisor.task
                    && record.kind == WaiterKind::Mailbox
                    && record.active)
            && matches!(waiter.and_then(|record| source_event(booted, record.id)), Some(source)
                if source.sequence > third.sequence)
    }

    pub(super) fn waiter_store_compacted(&self, booted: &X86BootedKernel) -> bool {
        let mut events = booted
            .kernel()
            .events()
            .iter()
            .filter(|event| event.kind == EventKind::WaiterCompacted);
        let Some(first) = events.next() else {
            return false;
        };
        let Some(second) = events.next() else {
            return false;
        };
        let Some(third) = events.next() else {
            return false;
        };
        let Some(fourth) = events.next() else {
            return false;
        };
        let compacted = [first, second, third, fourth];

        booted.kernel().waiters().is_empty()
            && events.next().is_none()
            && compacted.iter().enumerate().all(|(index, event)| {
                proves_compaction_event(
                    booted,
                    event,
                    WaiterId::new(index as u64 + 1),
                    self.supervisor.admission_authority,
                )
            })
            && second.sequence == first.sequence + 1
            && third.sequence == second.sequence + 1
            && matches!(source_event(booted, WaiterId::new(4)), Some(source)
                if fourth.sequence > source.sequence)
    }
}

fn proves_compaction_event(
    booted: &X86BootedKernel,
    compacted: &Event,
    waiter: WaiterId,
    authority: agent_kernel_core::CapabilityId,
) -> bool {
    let Some(source) = source_event(booted, waiter) else {
        return false;
    };
    let signal_matches = match source.waiter_kind {
        Some(WaiterKind::Signal) => compacted.signal == source.signal && source.signal.is_some(),
        Some(WaiterKind::Mailbox) => {
            compacted.signal == Some(SignalKey::new(0)) && source.signal.is_none()
        }
        None => false,
    };
    compacted.agent == ADMISSION_SUPERVISOR
        && compacted.target_agent == Some(source.agent)
        && compacted.resource == source.resource
        && compacted.capability == Some(authority)
        && compacted.operation == Some(Operation::Rollback)
        && compacted.task == source.task
        && compacted.waiter == Some(waiter)
        && compacted.waiter_kind == source.waiter_kind
        && signal_matches
        && matches!(
            (source.kind, source.waiter_kind),
            (EventKind::TaskWaiting, Some(WaiterKind::Signal))
                | (EventKind::MessageWaitStarted, Some(WaiterKind::Mailbox))
        )
}

fn source_event(booted: &X86BootedKernel, waiter: WaiterId) -> Option<&Event> {
    booted.kernel().events().iter().find(|event| {
        event.waiter == Some(waiter)
            && matches!(
                event.kind,
                EventKind::TaskWaiting | EventKind::MessageWaitStarted
            )
    })
}
