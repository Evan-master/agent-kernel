//! Transactional bridge from semantic admission requests to physical runtime.

use agent_kernel_core::{
    EventKind, RunQueueEntry, RuntimeAdmissionFailure, RuntimeAdmissionRecord,
    RuntimeAdmissionStatus,
};
use agent_kernel_x86_64::{agent_call::AgentCallContext, agent_image::VerifiedAgentImage};

use crate::{
    agent_cpu::AgentCpuRuntime,
    agent_memory::{NativeAddressSpaceFramePool, RuntimeMemoryPool},
    native_address_space_service::{
        NativeAddressSpaceAdmission, NativeAddressSpaceAdmissionStage, NativeAddressSpaceService,
    },
    native_agent_runtime::NativeAgentRuntime,
    X86BootedKernel,
};

pub(crate) struct NativeRuntimeAdmissionBroker;

impl NativeRuntimeAdmissionBroker {
    pub(crate) fn admit_next(
        booted: &mut X86BootedKernel,
        pool: &mut NativeAddressSpaceFramePool,
        runtime: &mut NativeAgentRuntime,
        cpu_runtime: &AgentCpuRuntime,
        memory_pool: &RuntimeMemoryPool,
        capsule: &[u8],
    ) -> Option<Result<NativeAddressSpaceAdmission, RuntimeAdmissionRecord>> {
        let permit = booted.kernel().sys_prepare_next_runtime_admission().ok()?;
        let entry = booted.kernel().agent_entry(permit.target()).ok()?;
        if entry.task != Some(permit.task()) || entry.image != permit.image() {
            return None;
        }
        let context = AgentCallContext::new_admitted(
            permit.target(),
            permit.task(),
            permit.image(),
            entry.capability,
            permit.requester(),
        )?;
        let image =
            VerifiedAgentImage::verify(booted.kernel().agent_image(permit.image()).ok()?, capsule)
                .ok()?;
        let initial_pool_len = pool.len();
        let initial_runtime_len = runtime.len();
        let initial_queue_len = booted.kernel().run_queue().len();
        let event_start = booted.kernel().events().len();

        match NativeAddressSpaceService::admit(
            pool,
            runtime,
            cpu_runtime,
            memory_pool,
            image,
            context,
        )? {
            Ok(admission) => {
                let committed = match booted.kernel_mut().sys_commit_runtime_admission(permit) {
                    Ok(record) => record,
                    Err(_) => {
                        rollback_registered(
                            pool,
                            runtime,
                            admission,
                            initial_pool_len,
                            initial_runtime_len,
                        )?;
                        return None;
                    }
                };
                admitted_state_valid(
                    booted,
                    pool,
                    runtime,
                    committed,
                    admission,
                    initial_pool_len,
                    initial_runtime_len,
                    initial_queue_len,
                    event_start,
                )
                .then_some(Ok(admission))
            }
            Err(failure) => {
                if pool.len() != initial_pool_len || runtime.len() != initial_runtime_len {
                    return None;
                }
                let rejected = booted
                    .kernel_mut()
                    .sys_reject_runtime_admission(permit, map_failure(failure.stage()))
                    .ok()?;
                (rejected.status == RuntimeAdmissionStatus::Rejected
                    && rejected.failure == Some(map_failure(failure.stage()))
                    && booted.kernel().run_queue().len() == initial_queue_len
                    && matches!(booted.kernel().events().get(event_start), Some(event)
                        if event.kind == EventKind::RuntimeAdmissionRejected
                            && event.runtime_admission == Some(rejected.id)))
                .then_some(Err(rejected))
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn admitted_state_valid(
    booted: &X86BootedKernel,
    pool: &NativeAddressSpaceFramePool,
    runtime: &NativeAgentRuntime,
    record: RuntimeAdmissionRecord,
    admission: NativeAddressSpaceAdmission,
    initial_pool_len: usize,
    initial_runtime_len: usize,
    initial_queue_len: usize,
    event_start: usize,
) -> bool {
    let events = booted.kernel().events().get(event_start..);
    record.status == RuntimeAdmissionStatus::Admitted
        && record.failure.is_none()
        && admission.agent() == record.target
        && runtime.len() == initial_runtime_len + 1
        && runtime.contains(record.target)
        && pool.len() + admission.identity().owned_frame_count() == initial_pool_len
        && booted.kernel().run_queue().len() == initial_queue_len + 1
        && booted.kernel().run_queue().last()
            == Some(&RunQueueEntry {
                task: record.task,
                agent: record.target,
            })
        && matches!(events, Some(events)
            if events.len() == 2
                && events[0].kind == EventKind::RuntimeAdmissionAdmitted
                && events[0].runtime_admission == Some(record.id)
                && events[1].kind == EventKind::TaskQueued
                && events[1].runtime_admission == Some(record.id))
}

fn rollback_registered(
    pool: &mut NativeAddressSpaceFramePool,
    runtime: &mut NativeAgentRuntime,
    admission: NativeAddressSpaceAdmission,
    initial_pool_len: usize,
    initial_runtime_len: usize,
) -> Option<()> {
    let reclaimed = runtime
        .take_prepared(admission.agent())?
        .reclaim_unstarted_address_space(pool)?;
    (reclaimed.matches(admission.agent(), admission.identity())
        && pool.len() == initial_pool_len
        && runtime.len() == initial_runtime_len
        && !runtime.contains(admission.agent()))
    .then_some(())
}

const fn map_failure(stage: NativeAddressSpaceAdmissionStage) -> RuntimeAdmissionFailure {
    match stage {
        NativeAddressSpaceAdmissionStage::Allocation => {
            RuntimeAdmissionFailure::AllocationUnavailable
        }
        NativeAddressSpaceAdmissionStage::MemoryBuild => RuntimeAdmissionFailure::MemoryBuild,
        NativeAddressSpaceAdmissionStage::CpuPreparation => RuntimeAdmissionFailure::CpuPreparation,
        NativeAddressSpaceAdmissionStage::RuntimeRegistration => {
            RuntimeAdmissionFailure::RuntimeRegistration
        }
    }
}
