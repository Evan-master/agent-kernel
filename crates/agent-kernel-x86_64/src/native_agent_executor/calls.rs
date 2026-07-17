//! Inner role-independent loop for one running native Agent session.
//!
//! The current decoded operation selects a public semantic handler. Immediate
//! replies resume the same owned frame; wait, yield, and completion return
//! control to the outer scheduler loop.

mod capability;
mod mailbox;
mod resource;
mod task;

use agent_kernel_x86_64::agent_call::AgentCallRequest;

use super::{NativeExecutionReport, NativeRuntimeEvidence, NativeVerifyAuthority};
use crate::{
    agent_cpu::{AgentRunOutcome, PendingAgentCallCpu, ResumableAgentCpu, WaitingAgentCallCpu},
    native_agent_runtime::NativeAgentRuntime,
    X86BootedKernel,
};

pub(super) fn run(
    booted: &mut X86BootedKernel,
    runtime: &mut NativeAgentRuntime,
    report: &mut NativeExecutionReport,
    evidence: &mut NativeRuntimeEvidence,
    verify_authority: Option<NativeVerifyAuthority>,
    mut pending: PendingAgentCallCpu,
) -> Option<()> {
    loop {
        let request = pending.request();
        let resumable = match request {
            AgentCallRequest::DescribeContext { .. } => pending.acknowledge_describe()?,
            AgentCallRequest::SubmitTaskResult { result, .. } => {
                task::submit_result(booted, pending, result)?
            }
            AgentCallRequest::SendMessage {
                recipient,
                kind,
                payload,
                ..
            } => mailbox::send(booted, pending, recipient, kind, payload)?,
            AgentCallRequest::ReceiveMessage { .. } => {
                match mailbox::receive_or_wait(booted, pending)? {
                    mailbox::ReceiveDisposition::Continue(cpu) => cpu,
                    mailbox::ReceiveDisposition::Waiting(cpu) => {
                        if runtime.park_waiting_call(cpu).is_some() {
                            return None;
                        }
                        return Some(());
                    }
                }
            }
            AgentCallRequest::AcknowledgeMessage { message, .. } => {
                mailbox::acknowledge(booted, pending, message)?
            }
            AgentCallRequest::CreateResource {
                authority,
                parent,
                kind,
                operations,
                ..
            } => resource::create(booted, pending, authority, parent, kind, operations)?,
            AgentCallRequest::RetireResource {
                resource,
                capability,
                ..
            } => resource::retire(booted, pending, resource, capability)?,
            AgentCallRequest::DeriveCapability {
                source,
                target,
                operations,
                ..
            } => capability::derive(booted, pending, source, target, operations)?,
            AgentCallRequest::RevokeDerivedCapability { source, target, .. } => {
                capability::revoke(booted, pending, source, target)?
            }
            AgentCallRequest::Yield { .. } => {
                let yielded = task::yield_running(booted, pending)?;
                if runtime.park_yielded_call(yielded).is_some() {
                    return None;
                }
                return Some(());
            }
            AgentCallRequest::InspectTaskResult { target_task, .. } => {
                task::inspect_result(booted, pending, target_task, verify_authority?)?
            }
            AgentCallRequest::VerifyTask { target_task, .. } => {
                task::verify(booted, pending, target_task, verify_authority?)?
            }
            AgentCallRequest::CompleteTask { .. } => {
                let completed = task::complete(booted, pending)?;
                report.record(completed)?;
                return Some(());
            }
        };
        match resume_next(resumable)? {
            AgentRunOutcome::Call(next) => pending = next,
            AgentRunOutcome::Preempted(cpu) => {
                super::expire_quantum(booted, runtime, evidence, cpu)?;
                return Some(());
            }
            AgentRunOutcome::Fault(cpu) => {
                super::contain_fault(booted, report, evidence, cpu)?;
                return Some(());
            }
        }
    }
}

pub(super) fn resume_waiting_receive(
    booted: &mut X86BootedKernel,
    waiting: WaitingAgentCallCpu,
) -> Option<ResumableAgentCpu> {
    mailbox::resume_waiting(booted, waiting)
}

fn resume_next(cpu: ResumableAgentCpu) -> Option<AgentRunOutcome> {
    cpu.resume_until_boundary()
}
