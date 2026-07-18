//! Pure lifecycle ledger for one retained x86 Agent runtime page.
//!
//! This architecture-library module binds a Memory Resource and MemoryCell to
//! one private page slot. It owns no page tables or physical pointers; the
//! bare-metal memory owner performs those effects around these deterministic
//! reservation, commit, cancellation, and release transitions.

use agent_kernel_core::{CapabilityId, MemoryCellId, ResourceId};

pub const RUNTIME_PAGE_ACCESS_READ_WRITE: u64 = 3;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum RuntimePageState {
    Available,
    Reserved {
        resource: ResourceId,
        capability: CapabilityId,
        generation: u64,
        token: u64,
    },
    Mapped {
        resource: ResourceId,
        capability: CapabilityId,
        cell: MemoryCellId,
        generation: u64,
        token: u64,
    },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RuntimePageReservation {
    resource: ResourceId,
    capability: CapabilityId,
    generation: u64,
    token: u64,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RuntimePageRelease {
    resource: ResourceId,
    capability: CapabilityId,
    cell: MemoryCellId,
    generation: u64,
    token: u64,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RuntimePageBinding {
    resource: ResourceId,
    capability: CapabilityId,
    cell: MemoryCellId,
    generation: u64,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RuntimePageLedger {
    state: RuntimePageState,
    generation: u64,
    next_token: u64,
}

impl RuntimePageLedger {
    pub const fn new() -> Self {
        Self {
            state: RuntimePageState::Available,
            generation: 0,
            next_token: 1,
        }
    }

    pub fn reserve(
        &mut self,
        resource: ResourceId,
        capability: CapabilityId,
    ) -> Option<RuntimePageReservation> {
        if resource.raw() == 0 || capability.raw() == 0 || self.state != RuntimePageState::Available
        {
            return None;
        }
        let generation = self.generation.checked_add(1)?;
        let token = self.next_token;
        self.next_token = self.next_token.checked_add(1)?;
        let reservation = RuntimePageReservation {
            resource,
            capability,
            generation,
            token,
        };
        self.state = RuntimePageState::Reserved {
            resource,
            capability,
            generation,
            token,
        };
        Some(reservation)
    }

    pub fn commit_mapping(
        &mut self,
        reservation: RuntimePageReservation,
        cell: MemoryCellId,
    ) -> bool {
        if cell.raw() == 0 || !self.reservation_matches(reservation) {
            return false;
        }
        self.state = RuntimePageState::Mapped {
            resource: reservation.resource,
            capability: reservation.capability,
            cell,
            generation: reservation.generation,
            token: reservation.token,
        };
        true
    }

    pub fn cancel(&mut self, reservation: RuntimePageReservation) -> bool {
        if !self.reservation_matches(reservation) {
            return false;
        }
        self.state = RuntimePageState::Available;
        true
    }

    pub const fn prepare_release(
        &self,
        resource: ResourceId,
        cell: MemoryCellId,
    ) -> Option<RuntimePageRelease> {
        match self.state {
            RuntimePageState::Mapped {
                resource: actual_resource,
                capability,
                cell: actual_cell,
                generation,
                token,
            } if resource.raw() == actual_resource.raw() && cell.raw() == actual_cell.raw() => {
                Some(RuntimePageRelease {
                    resource,
                    capability,
                    cell,
                    generation,
                    token,
                })
            }
            _ => None,
        }
    }

    pub fn commit_release(&mut self, release: RuntimePageRelease) -> bool {
        if self.binding() != Some(release.binding())
            || !matches!(
                self.state,
                RuntimePageState::Mapped { token, .. } if token == release.token
            )
        {
            return false;
        }
        self.generation = release.generation;
        self.state = RuntimePageState::Available;
        true
    }

    pub const fn binding(&self) -> Option<RuntimePageBinding> {
        match self.state {
            RuntimePageState::Mapped {
                resource,
                capability,
                cell,
                generation,
                ..
            } => Some(RuntimePageBinding {
                resource,
                capability,
                cell,
                generation,
            }),
            _ => None,
        }
    }

    pub const fn matches(&self, resource: ResourceId, cell: MemoryCellId, generation: u64) -> bool {
        matches!(
            self.binding(),
            Some(binding)
                if binding.resource.raw() == resource.raw()
                    && binding.cell.raw() == cell.raw()
                    && binding.generation == generation
        )
    }

    pub const fn is_available(&self) -> bool {
        matches!(self.state, RuntimePageState::Available)
    }

    pub const fn generation(&self) -> u64 {
        self.generation
    }

    const fn reservation_matches(&self, reservation: RuntimePageReservation) -> bool {
        matches!(
            self.state,
            RuntimePageState::Reserved {
                resource,
                capability,
                generation,
                token,
            } if resource.raw() == reservation.resource.raw()
                && capability.raw() == reservation.capability.raw()
                && generation == reservation.generation
                && token == reservation.token
        )
    }
}

impl Default for RuntimePageLedger {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimePageReservation {
    pub const fn resource(self) -> ResourceId {
        self.resource
    }

    pub const fn capability(self) -> CapabilityId {
        self.capability
    }

    pub const fn generation(self) -> u64 {
        self.generation
    }
}

impl RuntimePageRelease {
    pub const fn resource(self) -> ResourceId {
        self.resource
    }

    pub const fn capability(self) -> CapabilityId {
        self.capability
    }

    pub const fn cell(self) -> MemoryCellId {
        self.cell
    }

    pub const fn generation(self) -> u64 {
        self.generation
    }

    const fn binding(self) -> RuntimePageBinding {
        RuntimePageBinding {
            resource: self.resource,
            capability: self.capability,
            cell: self.cell,
            generation: self.generation,
        }
    }
}

impl RuntimePageBinding {
    pub const fn resource(self) -> ResourceId {
        self.resource
    }

    pub const fn capability(self) -> CapabilityId {
        self.capability
    }

    pub const fn cell(self) -> MemoryCellId {
        self.cell
    }

    pub const fn generation(self) -> u64 {
        self.generation
    }
}
