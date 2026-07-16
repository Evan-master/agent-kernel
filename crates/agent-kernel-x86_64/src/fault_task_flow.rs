//! Admission, fault, recovery, and restart evidence for the native Fault Worker.
//!
//! This boot adapter creates one ordinary delegated Worker task and keeps it
//! accepted until the normal Workers finish. The x86 executor owns physical
//! exception capture and physical restart; semantic mutation uses only public
//! kernel records and syscalls.

use agent_kernel_core::{
    AgentEntryKind, AgentExecutionState, AgentId, AgentImageDigest, AgentImageId, AgentImageKind,
    AgentImageRecord, CapabilityId, EventKind, FaultKind, IntentKind, RunQueueEntry, TaskId,
    TaskStatus, VerificationRequirement,
};
use agent_kernel_x86_64::{agent_call::AgentCallContext, native_runtime::INVALID_OPCODE_VECTOR};

use crate::{
    native_agent_executor::NativeExecutionReport, native_agent_runtime::NativeAgentRuntime,
    X86BootedKernel,
};

pub(super) const FAULT_WORKER: AgentId = AgentId::new(6);

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

    pub(super) fn faulted_after_runtime(&self, booted: &X86BootedKernel) -> bool {
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
        matches!(task, Some(task)
            if task.status == TaskStatus::Faulted
                && task.assignee == Some(FAULT_WORKER)
                && task.delegated_capability == Some(self.worker.capability)
                && task.run_ticks == 1
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
                    && fault.detail == u64::from(INVALID_OPCODE_VECTOR))
            && !kernel.run_queue().contains(&self.run_queue_entry())
    }

    pub(super) fn restart_for_runtime(
        &self,
        booted: &mut X86BootedKernel,
        runtime: &mut NativeAgentRuntime,
        report: &mut NativeExecutionReport,
    ) -> Option<()> {
        if !runtime.is_empty() || report.faulted_len() != 1 || !self.faulted_after_runtime(booted) {
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
        if faulted.context() != context {
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
        if event.kind != EventKind::TaskFaultRecovered
            || event.agent != authority.bootstrap_agent
            || event.capability != Some(authority.bootstrap_capability)
            || event.task != Some(self.worker.task)
            || event.fault != Some(fault)
            || event.fault_kind != Some(FaultKind::ExecutionTrap)
            || event.fault_detail != Some(u64::from(INVALID_OPCODE_VECTOR))
            || !self.recovered_state_valid(booted, fault)
            || runtime.register_prepared(prepared).is_some()
        {
            return None;
        }
        self.queue_for_runtime(booted)?;
        self.recovered_and_queued_state_valid(booted, fault)
            .then_some(())
    }

    pub(super) fn completed_after_restart(&self, booted: &X86BootedKernel) -> bool {
        let kernel = booted.kernel();
        let task = kernel
            .tasks()
            .iter()
            .find(|task| task.id == self.worker.task);
        let context = kernel
            .execution_contexts()
            .iter()
            .find(|context| context.agent == FAULT_WORKER);
        let Some(fault) = task.and_then(|task| task.last_fault) else {
            return false;
        };
        let fault_record = kernel.faults().iter().find(|record| record.id == fault);
        let faulted = kernel.events().iter().position(|event| {
            event.kind == EventKind::TaskFaulted && event.task == Some(self.worker.task)
        });
        let recovered = kernel.events().iter().position(|event| {
            event.kind == EventKind::TaskFaultRecovered && event.task == Some(self.worker.task)
        });
        let requeued = recovered.and_then(|recovered| {
            kernel
                .events()
                .iter()
                .enumerate()
                .skip(recovered + 1)
                .find_map(|(index, event)| {
                    (event.kind == EventKind::TaskQueued
                        && event.agent == FAULT_WORKER
                        && event.task == Some(self.worker.task))
                    .then_some(index)
                })
        });
        let completed = kernel.events().iter().position(|event| {
            event.kind == EventKind::TaskCompleted
                && event.agent == FAULT_WORKER
                && event.task == Some(self.worker.task)
        });
        matches!(task, Some(task)
            if task.status == TaskStatus::Completed
                && task.assignee == Some(FAULT_WORKER)
                && task.delegated_capability == Some(self.worker.capability)
                && task.run_ticks == 2
                && task.last_fault == Some(fault)
                && task.result.is_none())
            && matches!(context, Some(context)
                if context.state == AgentExecutionState::Idle
                    && context.task.is_none()
                    && context.run_ticks == 0
                    && context.quantum_remaining == 0)
            && matches!(fault_record, Some(record)
                if record.agent == FAULT_WORKER
                    && record.task == self.worker.task
                    && record.kind == FaultKind::ExecutionTrap
                    && record.detail == u64::from(INVALID_OPCODE_VECTOR))
            && kernel.faults().len() == 1
            && matches!((faulted, recovered, requeued, completed),
                (Some(faulted), Some(recovered), Some(requeued), Some(completed))
                    if faulted < recovered && recovered < requeued && requeued < completed)
            && !kernel.run_queue().contains(&self.run_queue_entry())
    }

    fn recovered_state_valid(
        &self,
        booted: &X86BootedKernel,
        fault: agent_kernel_core::FaultId,
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
                && task.run_ticks == 1
                && task.quantum_remaining == 0
                && task.last_fault == Some(fault))
            && matches!(context, Some(context)
                if context.state == AgentExecutionState::Idle
                    && context.task.is_none()
                    && context.run_ticks == 0
                    && context.quantum_remaining == 0)
    }

    fn recovered_and_queued_state_valid(
        &self,
        booted: &X86BootedKernel,
        fault: agent_kernel_core::FaultId,
    ) -> bool {
        self.recovered_state_valid(booted, fault)
            && booted.kernel().run_queue() == [self.run_queue_entry()]
            && matches!(booted.kernel().events().last(), Some(event)
                if event.kind == EventKind::TaskQueued
                    && event.agent == FAULT_WORKER
                    && event.task == Some(self.worker.task))
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
