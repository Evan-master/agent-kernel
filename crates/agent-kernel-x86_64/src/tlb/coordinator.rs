//! Serialized TLB shootdown request and acknowledgement state.
//!
//! One coordinator admits one in-flight transaction. Validation precedes every
//! mutation, completion is exact target coverage, and timeout leaves the
//! request active so physical-frame reuse remains blocked.

use crate::cpu::{CpuIndex, CpuMask};

use super::{TlbAddressSpace, TlbFlushScope, TlbShootdownRequest, TlbShootdownStatus};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TlbShootdownProgress {
    Pending(CpuMask),
    Complete,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TlbShootdownError {
    InitiatorOffline(CpuIndex),
    ResidentCpuOffline(CpuMask),
    RequestActive { generation: u64 },
    NoActiveRequest,
    GenerationExhausted,
    StaleGeneration { expected: u64, actual: u64 },
    CpuNotTargeted(CpuIndex),
    DuplicateAcknowledgement(CpuIndex),
    RequestIncomplete { pending: CpuMask },
    RequestTimedOut { generation: u64 },
    RequestAlreadyComplete { generation: u64 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TlbShootdownCompletion {
    request: TlbShootdownRequest,
}

impl TlbShootdownCompletion {
    pub const fn generation(self) -> u64 {
        self.request.generation()
    }

    pub const fn address_space(self) -> TlbAddressSpace {
        self.request.address_space()
    }

    pub const fn request(self) -> TlbShootdownRequest {
        self.request
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TlbShootdownCoordinator {
    last_issued_generation: u64,
    completed_generation: u64,
    active: Option<TlbShootdownRequest>,
    acknowledged: CpuMask,
}

impl TlbShootdownCoordinator {
    pub const fn new() -> Self {
        Self::from_completed_generation(0)
    }

    pub const fn from_completed_generation(generation: u64) -> Self {
        Self {
            last_issued_generation: generation,
            completed_generation: generation,
            active: None,
            acknowledged: CpuMask::empty(),
        }
    }

    pub fn begin(
        &mut self,
        initiator: CpuIndex,
        online: CpuMask,
        resident: CpuMask,
        address_space: TlbAddressSpace,
        scope: TlbFlushScope,
    ) -> Result<TlbShootdownRequest, TlbShootdownError> {
        if let Some(active) = self.active {
            return Err(TlbShootdownError::RequestActive {
                generation: active.generation(),
            });
        }
        if !online.contains(initiator) {
            return Err(TlbShootdownError::InitiatorOffline(initiator));
        }
        let offline = resident.difference(online);
        if !offline.is_empty() {
            return Err(TlbShootdownError::ResidentCpuOffline(offline));
        }
        let generation = self
            .last_issued_generation
            .checked_add(1)
            .ok_or(TlbShootdownError::GenerationExhausted)?;
        let mut targets = resident.intersection(online);
        targets.remove(initiator);
        let request =
            TlbShootdownRequest::new(generation, address_space, scope, initiator, targets);
        self.active = Some(request);
        self.acknowledged = CpuMask::empty();
        self.last_issued_generation = generation;
        Ok(request)
    }

    pub fn acknowledge(
        &mut self,
        cpu: CpuIndex,
        generation: u64,
    ) -> Result<TlbShootdownProgress, TlbShootdownError> {
        let request = self.require_generation(generation)?;
        if request.status() == TlbShootdownStatus::TimedOut {
            return Err(TlbShootdownError::RequestTimedOut { generation });
        }
        if !request.targets().contains(cpu) {
            return Err(TlbShootdownError::CpuNotTargeted(cpu));
        }
        if self.acknowledged.contains(cpu) {
            return Err(TlbShootdownError::DuplicateAcknowledgement(cpu));
        }
        self.acknowledged.insert(cpu);
        let pending = request.targets().difference(self.acknowledged);
        if pending.is_empty() {
            if let Some(active) = self.active.as_mut() {
                active.set_status(TlbShootdownStatus::Complete);
            }
            Ok(TlbShootdownProgress::Complete)
        } else {
            Ok(TlbShootdownProgress::Pending(pending))
        }
    }

    pub fn mark_timed_out(&mut self, generation: u64) -> Result<CpuMask, TlbShootdownError> {
        let request = self.require_generation(generation)?;
        if request.status() == TlbShootdownStatus::Complete {
            return Err(TlbShootdownError::RequestAlreadyComplete { generation });
        }
        if request.status() == TlbShootdownStatus::TimedOut {
            return Err(TlbShootdownError::RequestTimedOut { generation });
        }
        let pending = request.targets().difference(self.acknowledged);
        if let Some(active) = self.active.as_mut() {
            active.set_status(TlbShootdownStatus::TimedOut);
        }
        Ok(pending)
    }

    pub fn finish(&mut self, generation: u64) -> Result<TlbShootdownCompletion, TlbShootdownError> {
        let request = self.require_generation(generation)?;
        match request.status() {
            TlbShootdownStatus::TimedOut => {
                return Err(TlbShootdownError::RequestTimedOut { generation });
            }
            TlbShootdownStatus::AwaitingAcknowledgements => {
                return Err(TlbShootdownError::RequestIncomplete {
                    pending: request.targets().difference(self.acknowledged),
                });
            }
            TlbShootdownStatus::Complete => {}
        }
        self.active = None;
        self.completed_generation = generation;
        self.acknowledged = CpuMask::empty();
        Ok(TlbShootdownCompletion { request })
    }

    pub const fn active_request(&self) -> Option<TlbShootdownRequest> {
        self.active
    }

    pub fn pending_targets(&self) -> Option<CpuMask> {
        self.active
            .map(|request| request.targets().difference(self.acknowledged))
    }

    pub const fn last_issued_generation(&self) -> u64 {
        self.last_issued_generation
    }

    pub const fn completed_generation(&self) -> u64 {
        self.completed_generation
    }

    pub const fn can_reuse_after(&self, generation: u64) -> bool {
        generation != 0 && generation <= self.completed_generation
    }

    fn require_generation(&self, actual: u64) -> Result<TlbShootdownRequest, TlbShootdownError> {
        let request = self.active.ok_or(TlbShootdownError::NoActiveRequest)?;
        let expected = request.generation();
        if expected == actual {
            Ok(request)
        } else {
            Err(TlbShootdownError::StaleGeneration { expected, actual })
        }
    }
}

impl Default for TlbShootdownCoordinator {
    fn default() -> Self {
        Self::new()
    }
}
