//! Inner role-independent loop for one running native Agent session.
//!
//! The current decoded operation selects a public semantic handler. Immediate
//! replies resume the same owned frame; wait, yield, and completion return
//! control to the outer scheduler loop.

mod agent_entry_retirement;
mod agent_image_record_retirement;
mod agent_image_signer;
mod agent_management;
mod agent_record_retirement;
mod capability;
mod capability_cleanup_revocation;
mod capability_compaction;
mod durable_archive;
mod event_archive;
mod fault_compaction;
mod intent_compaction;
mod mailbox;
mod memory_authority;
mod memory_cell_record_retirement;
mod memory_page;
mod memory_region;
mod namespace;
mod namespace_entry_retirement;
mod resource;
mod resource_record_retirement;
mod runtime_admission;
mod task;
mod task_compaction;
mod task_lifecycle;
mod waiter_compaction;

use agent_kernel_x86_64::agent_call::AgentCallRequest;
use agent_kernel_x86_64::tpm2::KernelStateSigner;

use super::{
    memory_reclamation, NativeExecutionReport, NativeRuntimeEvidence, NativeVerifyAuthority,
};
use crate::{
    agent_cpu::{AgentRunOutcome, PendingAgentCallCpu, ResumableAgentCpu, WaitingAgentCallCpu},
    agent_memory::RuntimeMemoryPool,
    native_agent_runtime::NativeAgentRuntime,
    X86BootedKernel,
};

pub(super) fn run(
    booted: &mut X86BootedKernel,
    runtime: &mut NativeAgentRuntime,
    memory_pool: &mut RuntimeMemoryPool,
    report: &mut NativeExecutionReport,
    evidence: &mut NativeRuntimeEvidence,
    verify_authority: Option<NativeVerifyAuthority>,
    mut durable_session: Option<&mut crate::NativeDurableSession<'_>>,
    state_signer: &mut Option<&mut dyn KernelStateSigner>,
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
            AgentCallRequest::RetireMessage { message, .. } => {
                mailbox::retire(booted, pending, message)?
            }
            AgentCallRequest::RetireOrphanedMessage {
                authority, message, ..
            } => mailbox::retire_orphaned(booted, pending, authority, message)?,
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
            AgentCallRequest::DeclareIntent {
                authority,
                resource,
                kind,
                verification,
                ..
            } => task_lifecycle::declare(booted, pending, authority, resource, kind, verification)?,
            AgentCallRequest::CreateTask {
                authority, intent, ..
            } => task_lifecycle::create(booted, pending, authority, intent)?,
            AgentCallRequest::DelegateTask {
                authority,
                delegated_task,
                target,
                ..
            } => task_lifecycle::delegate(booted, pending, authority, delegated_task, target)?,
            AgentCallRequest::RequestRuntimeAdmission {
                authority,
                target,
                target_task,
                ..
            } => runtime_admission::request(booted, pending, authority, target, target_task)?,
            AgentCallRequest::DiscoverRuntimeAdmission { .. } => {
                let resumable = pending.acknowledge_runtime_admission_discovery()?;
                crate::serial_write_line("AGENT_KERNEL_AGENT_CALL_RUNTIME_ADMISSION_DISCOVERY_OK");
                resumable
            }
            AgentCallRequest::CompactRuntimeAdmissions {
                authority, through, ..
            } => runtime_admission::compact(booted, pending, authority, through)?,
            AgentCallRequest::CompactTasks {
                authority, through, ..
            } => task_compaction::compact(booted, pending, authority, through)?,
            AgentCallRequest::CompactIntents {
                authority, through, ..
            } => intent_compaction::compact(booted, pending, authority, through)?,
            AgentCallRequest::CompactWaiters {
                authority, through, ..
            } => waiter_compaction::compact(booted, pending, authority, through)?,
            AgentCallRequest::CompactFaults {
                authority, through, ..
            } => fault_compaction::compact(booted, pending, authority, through)?,
            AgentCallRequest::ArchiveEvents {
                authority,
                through_sequence,
                ..
            } => event_archive::archive(booted, report, pending, authority, through_sequence)?,
            AgentCallRequest::PrepareDurableArchive {
                archive_authority,
                storage_authority,
                through_sequence,
                generation,
                ..
            } => durable_archive::prepare(
                booted,
                durable_session.as_deref_mut()?,
                pending,
                archive_authority,
                storage_authority,
                through_sequence,
                generation,
            )?,
            AgentCallRequest::CommitDurableArchiveFromMemory { generation, .. } => {
                durable_archive::commit(
                    booted,
                    durable_session.as_deref_mut()?,
                    report,
                    pending,
                    generation,
                )?
            }
            AgentCallRequest::SignDurableArchive { generation, .. } => {
                let signer = state_signer.as_mut()?;
                durable_archive::sign(
                    booted,
                    durable_session.as_deref_mut()?,
                    &mut **signer,
                    pending,
                    generation,
                )?
            }
            AgentCallRequest::RetireResourceRecord {
                authority, target, ..
            } => resource_record_retirement::retire(booted, pending, authority, target)?,
            AgentCallRequest::RevokeCapabilityForCleanup {
                authority, target, ..
            } => capability_cleanup_revocation::revoke(booted, pending, authority, target)?,
            AgentCallRequest::RetireMemoryCellRecord {
                authority, target, ..
            } => memory_cell_record_retirement::retire(
                booted,
                runtime,
                memory_pool,
                report,
                pending,
                authority,
                target,
            )?,
            AgentCallRequest::BindNamespaceEntry {
                authority,
                namespace: namespace_resource,
                key,
                object,
                ..
            } => namespace::bind(booted, pending, authority, namespace_resource, key, object)?,
            AgentCallRequest::ResolveNamespaceEntry {
                authority,
                namespace: namespace_resource,
                key,
                ..
            } => namespace::resolve(booted, pending, authority, namespace_resource, key)?,
            AgentCallRequest::ResolveNamespacePath {
                root,
                first,
                second,
                ..
            } => namespace::resolve_path(booted, pending, root, first, second)?,
            AgentCallRequest::ResolveNamespacePathFromMemory {
                root, generation, ..
            } => namespace::resolve_memory_path(booted, pending, root, generation)?,
            AgentCallRequest::CompareAndRebindNamespacePathFromMemory { generation, .. } => {
                namespace::compare_and_rebind_memory_path(booted, pending, generation)?
            }
            AgentCallRequest::RotateAgentImageSignerFromMemory { generation, .. } => {
                agent_image_signer::rotate(booted, pending, generation)?
            }
            AgentCallRequest::RebindNamespaceEntry {
                authority,
                entry,
                object,
                ..
            } => namespace::rebind(booted, pending, authority, entry, object)?,
            AgentCallRequest::RetireNamespaceEntry {
                authority, entry, ..
            } => namespace_entry_retirement::retire(booted, pending, authority, entry)?,
            AgentCallRequest::CompareAndRebindNamespaceEntry {
                authority,
                entry,
                expected_revision,
                object,
                ..
            } => namespace::compare_rebind(
                booted,
                pending,
                authority,
                entry,
                expected_revision,
                object,
            )?,
            AgentCallRequest::CompareAndRetireNamespaceEntry {
                authority,
                entry,
                expected_revision,
                ..
            } => namespace_entry_retirement::compare_retire(
                booted,
                pending,
                authority,
                entry,
                expected_revision,
            )?,
            AgentCallRequest::CompactCapability {
                authority, target, ..
            } => capability_compaction::compact(booted, pending, authority, target)?,
            AgentCallRequest::RetireAgentEntry {
                authority, target, ..
            } => agent_entry_retirement::retire(booted, runtime, pending, authority, target)?,
            AgentCallRequest::RetireAgentRecord {
                authority, target, ..
            } => agent_record_retirement::retire(booted, runtime, pending, authority, target)?,
            AgentCallRequest::RetireAgentImageRecord {
                authority, target, ..
            } => agent_image_record_retirement::retire(
                booted, runtime, report, pending, authority, target,
            )?,
            AgentCallRequest::RegisterManagedAgent {
                authority,
                resource,
                target,
                ..
            } => agent_management::register(booted, pending, authority, resource, target)?,
            AgentCallRequest::SuspendManagedAgent {
                authority, target, ..
            } => agent_management::suspend(booted, pending, authority, target)?,
            AgentCallRequest::ResumeManagedAgent {
                authority, target, ..
            } => agent_management::resume(booted, pending, authority, target)?,
            AgentCallRequest::RetireManagedAgent {
                authority, target, ..
            } => agent_management::retire(booted, pending, authority, target)?,
            AgentCallRequest::AllocateMemoryPage {
                capability,
                resource,
                ..
            } => memory_page::allocate(booted, memory_pool, pending, capability, resource)?,
            AgentCallRequest::InspectMemoryPage {
                capability, cell, ..
            } => memory_page::inspect(booted, memory_pool, pending, capability, cell)?,
            AgentCallRequest::ReleaseMemoryPage {
                capability, cell, ..
            } => memory_page::release(booted, memory_pool, pending, capability, cell)?,
            AgentCallRequest::AllocateMemoryRegion {
                capability,
                resource,
                page_count,
                ..
            } => memory_region::allocate(
                booted,
                memory_pool,
                pending,
                capability,
                resource,
                usize::try_from(page_count).ok()?,
            )?,
            AgentCallRequest::InspectMemoryRegion {
                capability, cell, ..
            } => memory_region::inspect(booted, memory_pool, pending, capability, cell)?,
            AgentCallRequest::ReleaseMemoryRegion {
                capability, cell, ..
            } => memory_region::release(booted, memory_pool, pending, capability, cell)?,
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
                task::completion_ready(booted, &pending)?;
                if let Some(session) = durable_session.as_deref_mut() {
                    durable_archive::cancel_for_context(session, pending.context())?;
                }
                let (pending, reclamation, reclaimed) =
                    memory_reclamation::reclaim_completion(booted, memory_pool, pending)?;
                let completed = task::complete(booted, pending, reclamation)?;
                report.record(completed)?;
                if reclaimed != 0 {
                    crate::serial_write_line("AGENT_KERNEL_NATIVE_COMPLETION_MEMORY_RECLAIMED_OK");
                }
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
                super::contain_fault(
                    booted,
                    memory_pool,
                    report,
                    evidence,
                    durable_session.as_deref_mut(),
                    cpu,
                )?;
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

pub(super) fn cancel_durable_preparation(
    session: &mut crate::NativeDurableSession<'_>,
    context: agent_kernel_x86_64::agent_call::AgentCallContext,
) -> Option<()> {
    durable_archive::cancel_for_context(session, context)
}
