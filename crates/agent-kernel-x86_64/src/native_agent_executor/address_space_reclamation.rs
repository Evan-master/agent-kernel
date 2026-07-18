//! Batch reclamation for every completed native CPU address-space owner.
//!
//! This executor child preflights all pool transitions on a copied ledger,
//! then consumes completed CPUs in the same order. Semantic transcript checks
//! run earlier while the completion report still owns every address space.

use agent_kernel_core::AgentId;
use agent_kernel_x86_64::{
    address_space::AGENT_OWNED_FRAME_COUNT, address_space_reclamation::AddressSpaceReclamation,
};

use super::NativeExecutionReport;
use crate::agent_memory::{
    NativeAddressSpaceFramePool, NATIVE_ADDRESS_SPACE_CAPACITY, NATIVE_ADDRESS_SPACE_FRAME_CAPACITY,
};

impl NativeExecutionReport {
    pub(crate) fn reclaim_completed_address_spaces<const COUNT: usize>(
        &mut self,
        pool: &mut NativeAddressSpaceFramePool,
        agents: [AgentId; COUNT],
    ) -> Option<()> {
        let expected_len = pool
            .len()
            .checked_add(COUNT.checked_mul(AGENT_OWNED_FRAME_COUNT)?)?;
        if COUNT == 0
            || COUNT > NATIVE_ADDRESS_SPACE_CAPACITY
            || self.completed.len() != COUNT
            || !self.faulted.is_empty()
            || expected_len != NATIVE_ADDRESS_SPACE_FRAME_CAPACITY
        {
            return None;
        }

        let mut preview = *pool;
        let mut tokens: [Option<AddressSpaceReclamation>; COUNT] = [None; COUNT];
        let mut roots = [0; COUNT];
        for (index, agent) in agents.iter().copied().enumerate() {
            if agent.raw() == 0 || agents[..index].contains(&agent) {
                return None;
            }
            let cpu = self.completed.get(agent).ok()?;
            let token = cpu.prepare_address_space_reclamation(&preview)?;
            roots[index] = token.identity().root();
            if !preview.preview_commit(token) {
                return None;
            }
            tokens[index] = Some(token);
        }
        if preview.len() != expected_len {
            return None;
        }

        for (index, agent) in agents.iter().copied().enumerate() {
            let cpu = self.completed.take(agent).ok()?;
            let reclaimed = cpu.reclaim_address_space(pool, tokens[index]?)?;
            if !reclaimed.matches(agent, roots[index]) {
                return None;
            }
        }
        (self.completed.is_empty() && pool.len() == expected_len && pool.all_reclaimed_and_zero())
            .then_some(())
    }
}
