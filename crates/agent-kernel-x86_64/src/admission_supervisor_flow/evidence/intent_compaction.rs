//! Active Intent Store and audit proof for Supervisor-triggered compaction.

use agent_kernel_core::{
    EventKind, IntentId, IntentKind, KernelError, Operation, TaskId, VerificationRequirement,
};

use crate::{
    admission_supervisor_flow::{PreparedAdmissionSupervisorFlow, ADMISSION_SUPERVISOR},
    X86BootedKernel, X86_INTENT_CAPACITY,
};

const COMPACTED_KINDS: [IntentKind; 6] = [
    IntentKind::Act,
    IntentKind::Act,
    IntentKind::Verify,
    IntentKind::Act,
    IntentKind::Act,
    IntentKind::Act,
];
const COMPACTED_VERIFICATION: [VerificationRequirement; 6] = [
    VerificationRequirement::Required,
    VerificationRequirement::Required,
    VerificationRequirement::Optional,
    VerificationRequirement::Required,
    VerificationRequirement::Required,
    VerificationRequirement::Optional,
];

impl PreparedAdmissionSupervisorFlow {
    pub(super) fn initial_intent_prefix_compacted(&self, booted: &X86BootedKernel) -> bool {
        let kernel = booted.kernel();
        let report = *booted.report();
        let mut events = kernel
            .events()
            .iter()
            .filter(|event| event.kind == EventKind::IntentCompacted);
        let Some(first) = events.next() else {
            return false;
        };
        let mut compacted = [*first; 6];
        for slot in &mut compacted[1..] {
            let Some(event) = events.next() else {
                return false;
            };
            *slot = *event;
        }

        kernel.intent_capacity() == X86_INTENT_CAPACITY
            && kernel.intents().len() == 6
            && kernel
                .intents()
                .iter()
                .enumerate()
                .all(|(index, intent)| intent.id == IntentId::new(index as u64 + 7))
            && (1..=6)
                .all(|raw| kernel.intent(IntentId::new(raw)) == Err(KernelError::IntentNotFound))
            && kernel.tasks().len() == 6
            && kernel.tasks().iter().enumerate().all(|(index, task)| {
                let raw = index as u64 + 7;
                task.id == TaskId::new(raw) && task.intent == IntentId::new(raw)
            })
            && compacted.iter().enumerate().all(|(index, event)| {
                event.sequence == first.sequence + index as u64
                    && event.agent == ADMISSION_SUPERVISOR
                    && event.capability == Some(self.supervisor.admission_authority)
                    && event.operation == Some(Operation::Rollback)
                    && event.resource == Some(report.bootstrap_resource)
                    && event.intent == Some(IntentId::new(index as u64 + 1))
                    && event.intent_kind == Some(COMPACTED_KINDS[index])
                    && event.verification == COMPACTED_VERIFICATION[index]
                    && event.target_agent == Some(report.bootstrap_agent)
            })
            && events.next().is_none()
    }
}
