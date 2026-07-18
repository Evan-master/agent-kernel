//! Fixed-capacity ownership ledger for reclaimed native address-space frames.
//!
//! This architecture-library module provides read-only preparation, stale-token
//! rejection, atomic whole-address-space commits, and one-shot frame transfer.
//! Physical clearing and page-table teardown stay in the bare-metal adapter.

use crate::address_space::{
    AgentMemoryIdentity, AGENT_CONTENT_FRAME_COUNT, AGENT_OWNED_FRAME_COUNT,
    AGENT_PAGE_TABLE_FRAME_COUNT,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AddressSpaceReclamation {
    identity: AgentMemoryIdentity,
    expected_len: usize,
    expected_generation: u64,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AddressSpaceAllocation {
    identity: AgentMemoryIdentity,
    expected_len: usize,
    expected_generation: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub struct AllocatedAddressSpaceFrames {
    identity: AgentMemoryIdentity,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AddressSpaceFramePool<const CAPACITY: usize> {
    frames: [u64; CAPACITY],
    len: usize,
    generation: u64,
}

impl<const CAPACITY: usize> AddressSpaceFramePool<CAPACITY> {
    pub const fn new() -> Self {
        Self {
            frames: [0; CAPACITY],
            len: 0,
            generation: 0,
        }
    }

    pub fn prepare(&self, identity: AgentMemoryIdentity) -> Option<AddressSpaceReclamation> {
        let end = self.len.checked_add(AGENT_OWNED_FRAME_COUNT)?;
        if end > CAPACITY
            || identity
                .owned_frames()
                .iter()
                .any(|frame| self.contains(*frame))
        {
            return None;
        }
        Some(AddressSpaceReclamation {
            identity,
            expected_len: self.len,
            expected_generation: self.generation,
        })
    }

    pub fn commit(&mut self, reclamation: AddressSpaceReclamation) -> bool {
        if self.len != reclamation.expected_len
            || self.generation != reclamation.expected_generation
        {
            return false;
        }
        let Some(end) = self.len.checked_add(AGENT_OWNED_FRAME_COUNT) else {
            return false;
        };
        let Some(next_generation) = self.generation.checked_add(1) else {
            return false;
        };
        if end > CAPACITY
            || reclamation
                .identity
                .owned_frames()
                .iter()
                .any(|frame| self.contains(*frame))
        {
            return false;
        }
        for (slot, frame) in self.frames[self.len..end]
            .iter_mut()
            .zip(reclamation.identity.owned_frames())
        {
            *slot = frame;
        }
        self.len = end;
        self.generation = next_generation;
        true
    }

    pub fn prepare_allocation(&self) -> Option<AddressSpaceAllocation> {
        let start = self.len.checked_sub(AGENT_OWNED_FRAME_COUNT)?;
        let mut page_table_frames = [0; AGENT_PAGE_TABLE_FRAME_COUNT];
        page_table_frames
            .copy_from_slice(&self.frames[start..start + AGENT_PAGE_TABLE_FRAME_COUNT]);
        let mut content_frames = [0; AGENT_CONTENT_FRAME_COUNT];
        content_frames.copy_from_slice(
            &self.frames[start + AGENT_PAGE_TABLE_FRAME_COUNT..start + AGENT_OWNED_FRAME_COUNT],
        );
        Some(AddressSpaceAllocation {
            identity: AgentMemoryIdentity::new(page_table_frames, content_frames)?,
            expected_len: self.len,
            expected_generation: self.generation,
        })
    }

    pub fn commit_allocation(
        &mut self,
        allocation: AddressSpaceAllocation,
    ) -> Option<AllocatedAddressSpaceFrames> {
        if self.len != allocation.expected_len || self.generation != allocation.expected_generation
        {
            return None;
        }
        let start = self.len.checked_sub(AGENT_OWNED_FRAME_COUNT)?;
        let selected = allocation.identity.owned_frames();
        if self.frames[start..self.len] != selected {
            return None;
        }
        let next_generation = self.generation.checked_add(1)?;
        self.frames[start..self.len].fill(0);
        self.len = start;
        self.generation = next_generation;
        Some(AllocatedAddressSpaceFrames {
            identity: allocation.identity,
        })
    }

    pub fn take_frame(&mut self) -> Option<u64> {
        let last = self.len.checked_sub(1)?;
        let next_generation = self.generation.checked_add(1)?;
        let frame = self.frames[last];
        self.frames[last] = 0;
        self.len = last;
        self.generation = next_generation;
        Some(frame)
    }

    pub fn contains(&self, frame: u64) -> bool {
        self.frames[..self.len].contains(&frame)
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn frames(&self) -> &[u64] {
        &self.frames[..self.len]
    }
}

impl<const CAPACITY: usize> Default for AddressSpaceFramePool<CAPACITY> {
    fn default() -> Self {
        Self::new()
    }
}

impl AddressSpaceReclamation {
    pub const fn identity(self) -> AgentMemoryIdentity {
        self.identity
    }

    pub const fn frame_count(self) -> usize {
        AGENT_OWNED_FRAME_COUNT
    }
}

impl AddressSpaceAllocation {
    pub const fn identity(self) -> AgentMemoryIdentity {
        self.identity
    }
}

impl AllocatedAddressSpaceFrames {
    pub const fn identity(&self) -> AgentMemoryIdentity {
        self.identity
    }

    pub const fn into_identity(self) -> AgentMemoryIdentity {
        self.identity
    }
}
