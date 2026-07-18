//! Semantic admission and terminal evidence for the address-space Reuse Worker.
//!
//! This boot-layer flow uses public kernel calls to create one verified task,
//! binds its immutable Capsule, and validates exact native execution evidence.
//! Physical frame allocation and CPU construction remain in the boot adapter.

use agent_kernel_core::{
    AgentEntryKind, AgentExecutionState, AgentId, AgentImageId, AgentImageKind, CapabilityId,
    EventKind, IntentId, IntentKind, IntentStatus, RunQueueEntry, TaskId, TaskStatus,
    VerificationRequirement,
};
use agent_kernel_x86_64::{agent_call::AgentCallContext, agent_image::VerifiedAgentImage};

use crate::{
    boot_agent_images::BootReuseWorkerImage, native_agent_executor::NativeExecutionReport,
    X86BootedKernel,
};

pub(super) const REUSE_WORKER: AgentId = AgentId::new(10);

pub(super) struct PreparedReuseWorkerFlow {
    intent: IntentId,
    task: TaskId,
    image: AgentImageId,
    capability: CapabilityId,
}

impl PreparedReuseWorkerFlow {
    pub(super) fn prepare(
        booted: &mut X86BootedKernel,
        contract: BootReuseWorkerImage,
    ) -> Option<Self> {
        let report = *booted.report();
        let kernel = booted.kernel_mut();
        kernel.sys_register_agent(REUSE_WORKER).ok()?;
        let intent = kernel
            .sys_declare_intent(
                report.bootstrap_agent,
                report.bootstrap_capability,
                report.bootstrap_resource,
                IntentKind::Act,
                VerificationRequirement::Required,
            )
            .ok()?;
        let task = kernel
            .sys_create_task(report.bootstrap_agent, report.bootstrap_capability, intent)
            .ok()?;
        kernel
            .sys_delegate_task(
                report.bootstrap_agent,
                report.bootstrap_capability,
                task,
                REUSE_WORKER,
            )
            .ok()?;
        let capability = kernel
            .tasks()
            .iter()
            .find(|record| record.id == task)?
            .delegated_capability?;
        let image = kernel
            .sys_register_agent_image(
                report.bootstrap_agent,
                report.bootstrap_capability,
                report.bootstrap_resource,
                AgentImageKind::Worker,
                contract.digest(),
                1,
                1,
            )
            .ok()?;
        kernel
            .sys_verify_agent_image(report.bootstrap_agent, report.bootstrap_capability, image)
            .ok()?;
        kernel
            .sys_launch_task_agent(
                REUSE_WORKER,
                capability,
                task,
                image,
                AgentEntryKind::Worker,
            )
            .ok()?;
        kernel.sys_accept_task(REUSE_WORKER, task).ok()?;
        kernel.sys_enqueue_task(REUSE_WORKER, task).ok()?;

        let flow = Self {
            intent,
            task,
            image,
            capability,
        };
        flow.queued_state_valid(booted).then_some(flow)
    }

    pub(super) const fn agent(&self) -> AgentId {
        REUSE_WORKER
    }

    pub(super) const fn call_context(&self) -> Option<AgentCallContext> {
        AgentCallContext::new(REUSE_WORKER, self.task, self.image, self.capability)
    }

    pub(super) fn verified_image<'a>(
        &self,
        booted: &X86BootedKernel,
        bytes: &'a [u8],
    ) -> Option<VerifiedAgentImage<'a>> {
        VerifiedAgentImage::verify(booted.kernel().agent_image(self.image).ok()?, bytes).ok()
    }

    pub(super) fn completed_after_runtime(
        &self,
        booted: &X86BootedKernel,
        report: &NativeExecutionReport,
        contract: BootReuseWorkerImage,
    ) -> bool {
        let Some(context) = self.call_context() else {
            return false;
        };
        let Some(completed) = report.completed(REUSE_WORKER) else {
            return false;
        };
        let task = booted
            .kernel()
            .tasks()
            .iter()
            .find(|record| record.id == self.task);
        let execution = booted
            .kernel()
            .execution_contexts()
            .iter()
            .find(|record| record.agent == REUSE_WORKER);
        matches!(task, Some(task) if task.status == TaskStatus::Completed
            && task.assignee == Some(REUSE_WORKER)
            && task.delegated_capability == Some(self.capability)
            && task.result == Some(contract.result())
            && task.run_ticks == 1)
            && matches!(execution, Some(execution) if execution.state == AgentExecutionState::Idle
                && execution.task.is_none())
            && completed.context() == context
            && completed.nonce() == contract.nonce()
            && completed.call_count() == 3
            && completed.address_space_switch_count() == 6
            && completed.operations() == contract.expected_operations()
            && completed.return_offsets() == contract.expected_return_offsets()
            && completed.physical_quantum_generation() == 1
            && completed.restart_generation() == 0
            && completed.lazy_data_byte() == 0
            && completed.runtime_page_generation() == 0
            && !completed.runtime_page_released()
            && completed.runtime_page_observation().is_none()
            && completed.runtime_region_generation() == 0
            && !completed.runtime_regions_released()
            && completed.runtime_region_observations().is_empty()
            && completed.reclamation_log().is_empty()
            && matches!(booted.kernel().events().last(), Some(event)
                if event.kind == EventKind::TaskCompleted
                    && event.agent == REUSE_WORKER
                    && event.task == Some(self.task))
    }

    pub(super) fn verify_completed(&self, booted: &mut X86BootedKernel) -> Option<()> {
        let report = *booted.report();
        booted
            .kernel_mut()
            .sys_verify_task(
                report.bootstrap_agent,
                report.bootstrap_capability,
                self.task,
            )
            .ok()?;
        let task = booted
            .kernel()
            .tasks()
            .iter()
            .find(|record| record.id == self.task)?;
        let intent = booted
            .kernel()
            .intents()
            .iter()
            .find(|record| record.id == self.intent)?;
        (task.status == TaskStatus::Verified
            && intent.status == IntentStatus::Fulfilled
            && matches!(booted.kernel().events().last(), Some(event)
                if event.kind == EventKind::IntentFulfilled
                    && event.task == Some(self.task)))
        .then_some(())
    }

    fn queued_state_valid(&self, booted: &X86BootedKernel) -> bool {
        let kernel = booted.kernel();
        let task = kernel.tasks().iter().find(|record| record.id == self.task);
        let execution = kernel
            .execution_contexts()
            .iter()
            .find(|record| record.agent == REUSE_WORKER);
        let entry = kernel.agent_entry(REUSE_WORKER).ok();
        matches!(task, Some(task) if task.status == TaskStatus::Accepted
            && task.run_ticks == 0
            && task.delegated_capability == Some(self.capability)
            && task.result.is_none())
            && matches!(execution, Some(execution) if execution.state == AgentExecutionState::Idle
                && execution.task.is_none())
            && matches!(entry, Some(entry) if entry.image == self.image
                && entry.task == Some(self.task)
                && entry.capability == self.capability)
            && kernel.run_queue()
                == [RunQueueEntry {
                    task: self.task,
                    agent: REUSE_WORKER,
                }]
            && matches!(kernel.events().last(), Some(event)
                if event.kind == EventKind::TaskQueued
                    && event.agent == REUSE_WORKER
                    && event.task == Some(self.task))
    }
}
