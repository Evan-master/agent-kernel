//! Public-kernel admission for one reclaimed-address-space Worker.

use agent_kernel_core::{
    AgentEntryKind, AgentExecutionState, AgentId, AgentImageKind, EventKind, IntentKind,
    TaskStatus, VerificationRequirement,
};

use super::{PreparedReuseWorkerFlow, REUSE_WORKERS};
use crate::{boot_agent_images::BootReuseWorkerImage, X86BootedKernel};

impl PreparedReuseWorkerFlow {
    pub(crate) fn prepare_unqueued(
        booted: &mut X86BootedKernel,
        agent: AgentId,
        contract: BootReuseWorkerImage,
    ) -> Option<Self> {
        if !REUSE_WORKERS.contains(&agent) {
            return None;
        }
        let report = *booted.report();
        let kernel = booted.kernel_mut();
        kernel.sys_register_agent(agent).ok()?;
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
                agent,
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
            .sys_launch_task_agent(agent, capability, task, image, AgentEntryKind::Worker)
            .ok()?;
        kernel.sys_accept_task(agent, task).ok()?;
        let flow = Self {
            agent,
            intent,
            task,
            image,
            capability,
        };
        flow.accepted_state_valid(booted).then_some(flow)
    }

    fn accepted_state_valid(&self, booted: &X86BootedKernel) -> bool {
        let kernel = booted.kernel();
        let task = kernel.tasks().iter().find(|record| record.id == self.task);
        let execution = kernel
            .execution_contexts()
            .iter()
            .find(|record| record.agent == self.agent);
        let entry = kernel.agent_entry(self.agent).ok();
        matches!(task, Some(task) if task.status == TaskStatus::Accepted
            && task.run_ticks == 0
            && task.delegated_capability == Some(self.capability)
            && task.result.is_none())
            && matches!(execution, Some(execution)
                if execution.state == AgentExecutionState::Idle && execution.task.is_none())
            && matches!(entry, Some(entry) if entry.image == self.image
                && entry.task == Some(self.task)
                && entry.capability == self.capability)
            && !kernel.run_queue().contains(&self.run_queue_entry())
            && matches!(kernel.events().last(), Some(event)
                if event.kind == EventKind::TaskAccepted
                    && event.agent == self.agent
                    && event.task == Some(self.task))
    }
}
