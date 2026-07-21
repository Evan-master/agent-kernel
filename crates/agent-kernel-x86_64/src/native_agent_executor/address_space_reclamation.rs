//! Batch reclamation for a complete set of native CPU address-space owners.
//!
//! This executor child preflights all pool transitions on a copied ledger,
//! then consumes completed CPUs in the same order. Semantic transcript checks
//! run earlier while the completion report still owns the requested batch.

use agent_kernel_core::AgentId;
use agent_kernel_x86_64::{
    address_space::AgentMemoryIdentity, address_space_reclamation::AddressSpaceReclamation,
};

use super::NativeExecutionReport;
use crate::agent_memory::{NativeAddressSpaceFramePool, NATIVE_ADDRESS_SPACE_CAPACITY};

impl NativeExecutionReport {
    pub(crate) fn reclaim_completed_address_spaces<const COUNT: usize>(
        &mut self,
        pool: &mut NativeAddressSpaceFramePool,
        agents: [AgentId; COUNT],
    ) -> Option<()> {
        if COUNT == 0
            || COUNT > NATIVE_ADDRESS_SPACE_CAPACITY
            || self.completed.len() != COUNT
            || !self.faulted.is_empty()
        {
            return None;
        }

        let mut preview = *pool;
        let mut tokens: [Option<AddressSpaceReclamation>; COUNT] = [None; COUNT];
        let mut identities: [Option<AgentMemoryIdentity>; COUNT] = [None; COUNT];
        for (index, agent) in agents.iter().copied().enumerate() {
            if agent.raw() == 0 || agents[..index].contains(&agent) {
                return None;
            }
            let cpu = self.completed.get(agent).ok()?;
            let token = cpu.prepare_address_space_reclamation(&preview)?;
            identities[index] = Some(token.identity());
            if !preview.preview_commit(token) {
                return None;
            }
            tokens[index] = Some(token);
        }
        let expected_len = preview.len();
        if expected_len <= pool.len() {
            return None;
        }

        for (index, agent) in agents.iter().copied().enumerate() {
            let cpu = self.completed.take(agent).ok()?;
            let reclaimed = cpu.reclaim_address_space(pool, tokens[index]?)?;
            if !reclaimed.matches(agent, identities[index]?) {
                return None;
            }
        }
        if !self.completed.is_empty() || pool.len() != expected_len {
            return None;
        }
        tokens
            .iter()
            .all(|token| matches!(token, Some(token) if pool.owns_zeroed(token.identity())))
            .then_some(())
    }
}
