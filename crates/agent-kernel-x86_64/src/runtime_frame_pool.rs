//! Pure ownership ledger for the shared x86 runtime frame pool.
//!
//! This architecture-library module reserves fixed frame indices across
//! Agents and binds each transaction to Resource, MemoryCell, and generation
//! identity. Bare-metal code owns physical addresses and byte clearing.

mod lifecycle;
mod types;

use agent_kernel_core::{AgentId, MemoryCellId, ResourceId};

pub use types::{RuntimeFrameBinding, RuntimeFrameRelease, RuntimeFrameReservation};

pub const RUNTIME_FRAME_POOL_CAPACITY: usize = 16;
pub const MAX_RUNTIME_REGION_PAGES: usize = 4;

const UNUSED_INDEX: u8 = u8::MAX;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum FrameState {
    Available,
    Reserved {
        agent: AgentId,
        resource: ResourceId,
        transaction: u64,
    },
    Mapped {
        agent: AgentId,
        resource: ResourceId,
        cell: MemoryCellId,
        generation: u64,
        transaction: u64,
    },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RuntimeFramePoolLedger {
    frames: [FrameState; RUNTIME_FRAME_POOL_CAPACITY],
    next_transaction: u64,
}

impl RuntimeFramePoolLedger {
    pub const fn new() -> Self {
        Self {
            frames: [FrameState::Available; RUNTIME_FRAME_POOL_CAPACITY],
            next_transaction: 1,
        }
    }

    pub fn binding(
        &self,
        agent: AgentId,
        resource: ResourceId,
        cell: MemoryCellId,
        generation: u64,
    ) -> Option<RuntimeFrameBinding> {
        let transaction = self.frames.iter().find_map(|state| match state {
            FrameState::Mapped {
                agent: actual_agent,
                resource: actual_resource,
                cell: actual_cell,
                generation: actual_generation,
                transaction,
            } if *actual_agent == agent
                && *actual_resource == resource
                && *actual_cell == cell
                && *actual_generation == generation =>
            {
                Some(*transaction)
            }
            _ => None,
        })?;
        let mut indices = [UNUSED_INDEX; MAX_RUNTIME_REGION_PAGES];
        let mut page_count = 0;
        for (index, state) in self.frames.iter().enumerate() {
            if matches!(state, FrameState::Mapped { transaction: actual, .. } if *actual == transaction)
            {
                if page_count == MAX_RUNTIME_REGION_PAGES {
                    return None;
                }
                indices[page_count] = u8::try_from(index).ok()?;
                page_count += 1;
            }
        }
        (page_count != 0).then(|| {
            RuntimeFrameBinding::new(
                agent,
                resource,
                cell,
                generation,
                page_count,
                indices,
                transaction,
            )
        })
    }

    pub fn available_frame_count(&self) -> usize {
        self.frames
            .iter()
            .filter(|state| **state == FrameState::Available)
            .count()
    }

    pub fn agent_is_clear(&self, agent: AgentId) -> bool {
        self.frames.iter().all(|state| match state {
            FrameState::Available => true,
            FrameState::Reserved { agent: owner, .. } | FrameState::Mapped { agent: owner, .. } => {
                *owner != agent
            }
        })
    }

    pub fn all_available(&self) -> bool {
        self.available_frame_count() == RUNTIME_FRAME_POOL_CAPACITY
    }

    fn reservation_matches(&self, reservation: RuntimeFrameReservation) -> bool {
        reservation.page_count() != 0
            && reservation.page_count() <= MAX_RUNTIME_REGION_PAGES
            && reservation
                .indices()
                .iter()
                .copied()
                .take(reservation.page_count())
                .all(|index| {
                    matches!(self.frames.get(usize::from(index)), Some(FrameState::Reserved {
                        agent,
                        resource,
                        transaction,
                    }) if *agent == reservation.agent()
                        && *resource == reservation.resource()
                        && *transaction == reservation.transaction())
                })
    }

    fn binding_matches_release(&self, release: RuntimeFrameRelease) -> bool {
        self.binding(
            release.agent(),
            release.resource(),
            release.cell(),
            release.generation(),
        ) == Some(release.binding())
    }

    fn clear_indices(&mut self, indices: &[u8; MAX_RUNTIME_REGION_PAGES], page_count: usize) {
        for index in indices.iter().copied().take(page_count) {
            self.frames[usize::from(index)] = FrameState::Available;
        }
    }
}

impl Default for RuntimeFramePoolLedger {
    fn default() -> Self {
        Self::new()
    }
}
