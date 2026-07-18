//! Atomic lifecycle transitions for shared runtime frame reservations.
//!
//! This architecture-library child selects fixed indices, commits semantic
//! identity, and consumes immutable release tokens. Physical bytes remain
//! owned by the bare-metal pool adapter.

use agent_kernel_core::{AgentId, MemoryCellId, ResourceId};

use super::{
    FrameState, RuntimeFramePoolLedger, RuntimeFrameRelease, RuntimeFrameReservation,
    MAX_RUNTIME_REGION_PAGES, UNUSED_INDEX,
};

impl RuntimeFramePoolLedger {
    pub fn reserve(
        &mut self,
        agent: AgentId,
        resource: ResourceId,
        page_count: usize,
    ) -> Option<RuntimeFrameReservation> {
        if agent.raw() == 0
            || resource.raw() == 0
            || page_count == 0
            || page_count > MAX_RUNTIME_REGION_PAGES
            || self.available_frame_count() < page_count
        {
            return None;
        }
        let transaction = self.next_transaction;
        self.next_transaction = transaction.checked_add(1)?;
        let mut indices = [UNUSED_INDEX; MAX_RUNTIME_REGION_PAGES];
        let mut selected = 0;
        for (index, state) in self.frames.iter().enumerate() {
            if *state == FrameState::Available && selected < page_count {
                indices[selected] = u8::try_from(index).ok()?;
                selected += 1;
            }
        }
        if selected != page_count {
            return None;
        }
        for index in indices.iter().copied().take(page_count) {
            self.frames[usize::from(index)] = FrameState::Reserved {
                agent,
                resource,
                transaction,
            };
        }
        Some(RuntimeFrameReservation::new(
            agent,
            resource,
            page_count,
            indices,
            transaction,
        ))
    }

    pub fn cancel(&mut self, reservation: RuntimeFrameReservation) -> bool {
        if !self.reservation_matches(reservation) {
            return false;
        }
        self.clear_indices(&reservation.indices(), reservation.page_count());
        true
    }

    pub fn commit_mapping(
        &mut self,
        reservation: RuntimeFrameReservation,
        cell: MemoryCellId,
        generation: u64,
    ) -> bool {
        if cell.raw() == 0
            || generation == 0
            || !self.reservation_matches(reservation)
            || self.frames.iter().any(
                |state| matches!(state, FrameState::Mapped { cell: actual, .. } if *actual == cell),
            )
        {
            return false;
        }
        for index in reservation
            .indices()
            .iter()
            .copied()
            .take(reservation.page_count())
        {
            self.frames[usize::from(index)] = FrameState::Mapped {
                agent: reservation.agent(),
                resource: reservation.resource(),
                cell,
                generation,
                transaction: reservation.transaction(),
            };
        }
        true
    }

    pub fn prepare_release(
        &self,
        agent: AgentId,
        resource: ResourceId,
        cell: MemoryCellId,
        generation: u64,
    ) -> Option<RuntimeFrameRelease> {
        Some(self.binding(agent, resource, cell, generation)?.release())
    }

    pub fn commit_release(&mut self, release: RuntimeFrameRelease) -> bool {
        if !self.binding_matches_release(release) {
            return false;
        }
        self.clear_indices(&release.indices(), release.page_count());
        true
    }
}
