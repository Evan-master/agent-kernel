//! Admission, fault, recovery, and restart evidence for the native Fault Worker.
//!
//! This boot adapter creates one ordinary delegated Worker task and keeps it
//! accepted until the normal Workers finish. The x86 executor owns physical
//! exception capture and physical restart; semantic mutation uses only public
//! kernel records and syscalls.

mod restart;

use agent_kernel_core::{
    AgentEntryKind, AgentExecutionState, AgentId, AgentImageDigest, AgentImageId, AgentImageKind,
    AgentImageRecord, CapabilityId, EventKind, IntentKind, RunQueueEntry, TaskId, TaskStatus,
    VerificationRequirement,
};
use agent_kernel_x86_64::{
    agent_call::AgentCallContext, native_runtime::NativeAgentFault, user_memory::UserMemoryLayout,
};

use crate::X86BootedKernel;

pub(super) const FAULT_WORKER: AgentId = AgentId::new(6);
const FAULT_WORKER_PAGE_FAULT_ERROR_CODE: u16 = 7;
const FAULT_WORKER_PAGE_FAULT_ADDRESS: u64 = UserMemoryLayout::fixed().signal_start();

pub(crate) const fn expected_page_fault() -> NativeAgentFault {
    NativeAgentFault::PageFault {
        error_code: FAULT_WORKER_PAGE_FAULT_ERROR_CODE,
        address: FAULT_WORKER_PAGE_FAULT_ADDRESS,
    }
}

#[derive(Copy, Clone)]
struct FaultWorkerTask {
    task: TaskId,
    image: AgentImageId,
    capability: CapabilityId,
}

pub(super) struct FaultTaskFlow;

pub(super) struct PreparedFaultTaskFlow {
    worker: FaultWorkerTask,
}

impl FaultTaskFlow {
    pub(super) fn prepare(
        booted: &mut X86BootedKernel,
        digest: AgentImageDigest,
    ) -> Option<PreparedFaultTaskFlow> {
        let report = *booted.report();
        let kernel = booted.kernel_mut();
        kernel.sys_register_agent(FAULT_WORKER).ok()?;
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
                FAULT_WORKER,
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
                digest,
                1,
                1,
            )
            .ok()?;
        kernel
            .sys_verify_agent_image(report.bootstrap_agent, report.bootstrap_capability, image)
            .ok()?;
        kernel
            .sys_launch_task_agent(
                FAULT_WORKER,
                capability,
                task,
                image,
                AgentEntryKind::Worker,
            )
            .ok()?;
        kernel.sys_accept_task(FAULT_WORKER, task).ok()?;

        let flow = PreparedFaultTaskFlow {
            worker: FaultWorkerTask {
                task,
                image,
                capability,
            },
        };
        flow.prepared_state_valid(booted).then_some(flow)
    }
}

impl PreparedFaultTaskFlow {
    pub(super) const fn run_queue_entry(&self) -> RunQueueEntry {
        RunQueueEntry {
            task: self.worker.task,
            agent: FAULT_WORKER,
        }
    }

    pub(super) fn call_context(&self) -> Option<AgentCallContext> {
        AgentCallContext::new(
            FAULT_WORKER,
            self.worker.task,
            self.worker.image,
            self.worker.capability,
        )
    }

    pub(super) fn image_record(&self, booted: &X86BootedKernel) -> Option<AgentImageRecord> {
        booted.kernel().agent_image(self.worker.image).ok()
    }

    pub(super) fn queue_for_runtime(&self, booted: &mut X86BootedKernel) -> Option<()> {
        if booted
            .kernel()
            .run_queue()
            .iter()
            .any(|entry| entry.agent == FAULT_WORKER)
        {
            return None;
        }
        booted
            .kernel_mut()
            .sys_enqueue_task(FAULT_WORKER, self.worker.task)
            .ok()?;
        matches!(booted.kernel().events().last(), Some(event)
            if event.kind == EventKind::TaskQueued
                && event.agent == FAULT_WORKER
                && event.task == Some(self.worker.task))
        .then_some(())
    }

    fn prepared_state_valid(&self, booted: &X86BootedKernel) -> bool {
        let kernel = booted.kernel();
        let task = kernel
            .tasks()
            .iter()
            .find(|task| task.id == self.worker.task);
        let context = kernel
            .execution_contexts()
            .iter()
            .find(|context| context.agent == FAULT_WORKER);
        let entry = kernel.agent_entry(FAULT_WORKER).ok();
        matches!(task, Some(task)
            if task.status == TaskStatus::Accepted
                && task.delegated_capability == Some(self.worker.capability)
                && task.run_ticks == 0
                && task.quantum_remaining == 0
                && task.result.is_none())
            && matches!(context, Some(context)
                if context.state == AgentExecutionState::Idle
                    && context.task.is_none()
                    && context.run_ticks == 0
                    && context.quantum_remaining == 0)
            && matches!(entry, Some(entry)
                if entry.kind == AgentEntryKind::Worker
                    && entry.image == self.worker.image
                    && entry.task == Some(self.worker.task)
                    && entry.capability == self.worker.capability)
            && !kernel
                .run_queue()
                .iter()
                .any(|queued| queued.task == self.worker.task)
            && matches!(kernel.events().last(), Some(event)
                if event.kind == EventKind::TaskAccepted
                    && event.agent == FAULT_WORKER
                    && event.task == Some(self.worker.task))
    }
}
