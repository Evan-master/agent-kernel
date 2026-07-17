//! Policy-driven routing of the exact lazy-page fault to the native Handler.
//!
//! This child applies only the installed core policy and validates the atomic
//! mailbox wake audit trail. It does not execute the Handler or repair memory.

use agent_kernel_core::{
    EventKind, FaultId, FaultPolicyAction, MessageId, MessageKind, MessageStatus, TaskStatus,
};

use super::{PreparedFaultTaskFlow, FAULT_WORKER};
use crate::{fault_handler_flow::FAULT_HANDLER, X86BootedKernel};

#[derive(Copy, Clone)]
pub(crate) struct RoutedFault {
    fault: FaultId,
    message: MessageId,
}

impl PreparedFaultTaskFlow {
    pub(crate) fn route_lazy_fault_to_handler(
        &self,
        booted: &mut X86BootedKernel,
    ) -> Option<RoutedFault> {
        if !self.lazy_page_faulted_after_runtime(booted) {
            return None;
        }
        let authority = *booted.report();
        let task = booted
            .kernel()
            .tasks()
            .iter()
            .find(|task| task.id == self.worker.task)?;
        let fault = task.last_fault?;
        let event_start = booted.kernel().events().len();
        let outcome = booted
            .kernel_mut()
            .sys_apply_fault_policy(
                authority.bootstrap_agent,
                authority.bootstrap_capability,
                fault,
            )
            .ok()?;
        let message = outcome.message?;
        let record = booted
            .kernel()
            .messages()
            .iter()
            .find(|record| record.id == message)?;
        let events = booted.kernel().events().get(event_start..)?;
        let fault_task = booted
            .kernel()
            .tasks()
            .iter()
            .find(|task| task.id == self.worker.task)?;
        if outcome.action != FaultPolicyAction::RouteToHandler
            || outcome.event.kind != EventKind::FaultPolicyApplied
            || outcome.event.fault != Some(fault)
            || record.sender != authority.bootstrap_agent
            || record.recipient != FAULT_HANDLER
            || record.kind != MessageKind::Fault
            || record.status != MessageStatus::Pending
            || record.payload.resource != Some(authority.bootstrap_resource)
            || record.payload.intent != Some(fault_task.intent)
            || record.payload.task != Some(self.worker.task)
            || record.payload.fault != Some(fault)
            || record.payload.capability.is_some()
            || record.payload.action.is_some()
            || fault_task.status != TaskStatus::Faulted
            || fault_task.assignee != Some(FAULT_WORKER)
            || events.len() != 4
            || events[0].kind != EventKind::MessageSent
            || events[1].kind != EventKind::MessageWaitWoken
            || events[2].kind != EventKind::FaultRouted
            || events[3].kind != EventKind::FaultPolicyApplied
            || !events.iter().all(|event| event.message == Some(message))
            || events[1].target_agent != Some(FAULT_HANDLER)
            || events[2].fault != Some(fault)
            || events[3].fault != Some(fault)
        {
            return None;
        }
        Some(RoutedFault { fault, message })
    }
}

impl RoutedFault {
    pub(crate) const fn fault(self) -> FaultId {
        self.fault
    }

    pub(crate) const fn message(self) -> MessageId {
        self.message
    }
}
