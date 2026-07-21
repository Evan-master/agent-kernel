//! Transactional admission for reclaimed native Agent address spaces.
//!
//! The service keeps one Agent-bound physical owner through frame allocation,
//! address-space reconstruction, CPU preparation, and runtime registration.
//! Every rejected transition clears and returns the complete variable owner.

mod types;

use agent_kernel_core::AgentId;
use agent_kernel_x86_64::{
    address_space::AgentMemoryIdentity, agent_call::AgentCallContext,
    agent_image::VerifiedAgentImage,
};

use crate::{
    agent_cpu::{AgentCpuPreparation, AgentCpuRuntime},
    agent_memory::{NativeAddressSpaceFramePool, PreparedAgentMemory, RuntimeMemoryPool},
    native_agent_runtime::NativeAgentRuntime,
};

#[derive(Copy, Clone, Eq, PartialEq)]
pub(crate) enum NativeAddressSpaceAdmissionStage {
    Allocation,
    MemoryBuild,
    CpuPreparation,
    RuntimeRegistration,
}

#[derive(Copy, Clone)]
pub(crate) struct NativeAddressSpaceAdmission {
    agent: AgentId,
    identity: AgentMemoryIdentity,
}

#[derive(Copy, Clone)]
pub(crate) struct NativeAddressSpaceAdmissionFailure {
    stage: NativeAddressSpaceAdmissionStage,
    agent: AgentId,
    identity: Option<AgentMemoryIdentity>,
    rolled_back: bool,
}

pub(crate) struct NativeAddressSpaceService;

impl NativeAddressSpaceService {
    pub(crate) fn admit(
        pool: &mut NativeAddressSpaceFramePool,
        runtime: &mut NativeAgentRuntime,
        cpu_runtime: &AgentCpuRuntime,
        memory_pool: &RuntimeMemoryPool,
        image: VerifiedAgentImage<'_>,
        context: AgentCallContext,
    ) -> Option<Result<NativeAddressSpaceAdmission, NativeAddressSpaceAdmissionFailure>> {
        let initial_pool_len = pool.len();
        let initial_runtime_len = runtime.len();
        let agent = context.agent();
        let Some(owner) = pool.allocate_zeroed(agent, image.code_page_count()) else {
            return Some(Err(NativeAddressSpaceAdmissionFailure::allocation(agent)));
        };
        let identity = owner.identity();
        let owned_frame_count = identity.owned_frame_count();
        if owner.agent() != agent
            || pool.len() != initial_pool_len.checked_sub(owned_frame_count)?
            || pool.owns(identity)
        {
            return rollback_owner(pool, owner, initial_pool_len);
        }

        let memory = match PreparedAgentMemory::prepare_reused(owner, image) {
            Ok(memory) => memory,
            Err(owner) => {
                return rollback_owner_at(
                    pool,
                    owner,
                    initial_pool_len,
                    NativeAddressSpaceAdmissionStage::MemoryBuild,
                );
            }
        };
        if memory.identity() != identity
            || memory.allocated_for() != Some(agent)
            || !memory.kernel_address_space_active()
            || !memory_pool.is_disjoint_from(&memory)
        {
            return rollback_memory(
                pool,
                memory,
                initial_pool_len,
                NativeAddressSpaceAdmissionStage::MemoryBuild,
                agent,
                identity,
            );
        }

        let cpu = match cpu_runtime.prepare_owned(memory, context) {
            AgentCpuPreparation::Prepared(cpu) => cpu,
            AgentCpuPreparation::Rejected(memory) => {
                return rollback_memory(
                    pool,
                    memory,
                    initial_pool_len,
                    NativeAddressSpaceAdmissionStage::CpuPreparation,
                    agent,
                    identity,
                );
            }
        };
        if let Some(rejected) = runtime.register_prepared(cpu) {
            let reclaimed = rejected
                .into_prepared()?
                .reclaim_unstarted_address_space(pool)?;
            if !reclaimed.matches(agent, identity)
                || runtime.len() != initial_runtime_len
                || !pool_restored(pool, initial_pool_len, identity)
            {
                return None;
            }
            return Some(Err(NativeAddressSpaceAdmissionFailure::rolled_back(
                NativeAddressSpaceAdmissionStage::RuntimeRegistration,
                agent,
                identity,
            )));
        }

        (runtime.len() == initial_runtime_len.checked_add(1)?
            && runtime.contains(agent)
            && pool.len() == initial_pool_len.checked_sub(owned_frame_count)?
            && !pool.owns(identity))
        .then_some(Ok(NativeAddressSpaceAdmission { agent, identity }))
    }
}

fn rollback_owner(
    pool: &mut NativeAddressSpaceFramePool,
    owner: agent_kernel_x86_64::address_space_reclamation::AllocatedAddressSpaceFrames,
    initial_pool_len: usize,
) -> Option<Result<NativeAddressSpaceAdmission, NativeAddressSpaceAdmissionFailure>> {
    rollback_owner_at(
        pool,
        owner,
        initial_pool_len,
        NativeAddressSpaceAdmissionStage::Allocation,
    )
}

fn rollback_owner_at(
    pool: &mut NativeAddressSpaceFramePool,
    owner: agent_kernel_x86_64::address_space_reclamation::AllocatedAddressSpaceFrames,
    initial_pool_len: usize,
    stage: NativeAddressSpaceAdmissionStage,
) -> Option<Result<NativeAddressSpaceAdmission, NativeAddressSpaceAdmissionFailure>> {
    let agent = owner.agent();
    let identity = owner.identity();
    let restored = pool.cancel_zeroed_allocation(owner).ok()?;
    (restored == identity && pool_restored(pool, initial_pool_len, identity)).then_some(Err(
        NativeAddressSpaceAdmissionFailure::rolled_back(stage, agent, identity),
    ))
}

fn rollback_memory(
    pool: &mut NativeAddressSpaceFramePool,
    memory: PreparedAgentMemory,
    initial_pool_len: usize,
    stage: NativeAddressSpaceAdmissionStage,
    agent: AgentId,
    identity: AgentMemoryIdentity,
) -> Option<Result<NativeAddressSpaceAdmission, NativeAddressSpaceAdmissionFailure>> {
    let reclaimed = memory.cancel_address_space(pool)?;
    (reclaimed.root() == identity.root()
        && reclaimed.frame_count() == identity.owned_frame_count()
        && pool_restored(pool, initial_pool_len, identity))
    .then_some(Err(NativeAddressSpaceAdmissionFailure::rolled_back(
        stage, agent, identity,
    )))
}

fn pool_restored(
    pool: &NativeAddressSpaceFramePool,
    initial_pool_len: usize,
    identity: AgentMemoryIdentity,
) -> bool {
    pool.len() == initial_pool_len && pool.owns_zeroed(identity)
}
