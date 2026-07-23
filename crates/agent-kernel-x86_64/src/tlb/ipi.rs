//! Atomic single-flight transport for a validated TLB shootdown request.

use core::sync::atomic::{AtomicU16, AtomicU64, AtomicU8, Ordering};

use crate::cpu::{CpuIndex, CpuMask, CPU_MASK_WORD_COUNT};

use super::{
    TlbAddressSpace, TlbFlushKind, TlbFlushScope, TlbShootdownRequest, TlbShootdownStatus,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum TlbIpiState {
    Idle = 0,
    Publishing = 1,
    Active = 2,
    Complete = 3,
    TimedOut = 4,
}

impl TlbIpiState {
    fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            0 => Some(Self::Idle),
            1 => Some(Self::Publishing),
            2 => Some(Self::Active),
            3 => Some(Self::Complete),
            4 => Some(Self::TimedOut),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TlbIpiError {
    InvalidRequest,
    RequestActive,
    NoActiveRequest,
    CorruptRequest,
    StaleGeneration { expected: u64, actual: u64 },
    CpuNotTargeted(CpuIndex),
    DuplicateAcknowledgement(CpuIndex),
    Incomplete(CpuMask),
    TimedOut,
    InvalidState,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TlbIpiWork {
    generation: u64,
    address_space: TlbAddressSpace,
    scope: TlbFlushScope,
    initiator: CpuIndex,
    targets: CpuMask,
}

impl TlbIpiWork {
    pub const fn generation(self) -> u64 {
        self.generation
    }

    pub const fn address_space(self) -> TlbAddressSpace {
        self.address_space
    }

    pub const fn scope(self) -> TlbFlushScope {
        self.scope
    }

    pub const fn initiator(self) -> CpuIndex {
        self.initiator
    }

    pub const fn targets(self) -> CpuMask {
        self.targets
    }
}

pub struct TlbIpiMailbox {
    state: AtomicU8,
    generation: AtomicU64,
    root: AtomicU64,
    address_space_generation: AtomicU64,
    scope_kind: AtomicU8,
    scope_start: AtomicU64,
    scope_pages: AtomicU16,
    initiator: AtomicU16,
    targets: [AtomicU64; CPU_MASK_WORD_COUNT],
    acknowledged: [AtomicU64; CPU_MASK_WORD_COUNT],
}

impl TlbIpiMailbox {
    pub const fn new() -> Self {
        Self {
            state: AtomicU8::new(TlbIpiState::Idle as u8),
            generation: AtomicU64::new(0),
            root: AtomicU64::new(0),
            address_space_generation: AtomicU64::new(0),
            scope_kind: AtomicU8::new(0),
            scope_start: AtomicU64::new(0),
            scope_pages: AtomicU16::new(0),
            initiator: AtomicU16::new(0),
            targets: [const { AtomicU64::new(0) }; CPU_MASK_WORD_COUNT],
            acknowledged: [const { AtomicU64::new(0) }; CPU_MASK_WORD_COUNT],
        }
    }

    pub fn state(&self) -> Result<TlbIpiState, TlbIpiError> {
        TlbIpiState::from_raw(self.state.load(Ordering::Acquire)).ok_or(TlbIpiError::CorruptRequest)
    }

    pub fn publish(&self, request: TlbShootdownRequest) -> Result<(), TlbIpiError> {
        if request.status() != TlbShootdownStatus::AwaitingAcknowledgements
            || request.targets().is_empty()
        {
            return Err(TlbIpiError::InvalidRequest);
        }
        self.state
            .compare_exchange(
                TlbIpiState::Idle as u8,
                TlbIpiState::Publishing as u8,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .map_err(|_| TlbIpiError::RequestActive)?;
        self.generation
            .store(request.generation(), Ordering::Relaxed);
        self.root
            .store(request.address_space().root(), Ordering::Relaxed);
        self.address_space_generation
            .store(request.address_space().generation(), Ordering::Relaxed);
        let (kind, start, pages) = encode_scope(request.scope());
        self.scope_kind.store(kind, Ordering::Relaxed);
        self.scope_start.store(start, Ordering::Relaxed);
        self.scope_pages.store(pages, Ordering::Relaxed);
        self.initiator
            .store(request.initiator().get(), Ordering::Relaxed);
        for (slot, word) in self.targets.iter().zip(request.targets().words()) {
            slot.store(word, Ordering::Relaxed);
        }
        for slot in &self.acknowledged {
            slot.store(0, Ordering::Relaxed);
        }
        self.state
            .store(TlbIpiState::Active as u8, Ordering::Release);
        Ok(())
    }

    pub fn work_for(&self, cpu: CpuIndex) -> Result<TlbIpiWork, TlbIpiError> {
        match self.state()? {
            TlbIpiState::Active => {}
            TlbIpiState::TimedOut => return Err(TlbIpiError::TimedOut),
            _ => return Err(TlbIpiError::NoActiveRequest),
        }
        let work = self.decode_work()?;
        if !work.targets.contains(cpu) {
            return Err(TlbIpiError::CpuNotTargeted(cpu));
        }
        Ok(work)
    }

    pub fn acknowledge(&self, cpu: CpuIndex, generation: u64) -> Result<(), TlbIpiError> {
        let work = self.work_for(cpu)?;
        if work.generation != generation {
            return Err(TlbIpiError::StaleGeneration {
                expected: work.generation,
                actual: generation,
            });
        }
        let raw = cpu.as_usize();
        let word = raw / u64::BITS as usize;
        let bit = 1u64 << (raw % u64::BITS as usize);
        let previous = self.acknowledged[word].fetch_or(bit, Ordering::AcqRel);
        if previous & bit != 0 {
            return Err(TlbIpiError::DuplicateAcknowledgement(cpu));
        }
        Ok(())
    }

    pub fn finish(&self, generation: u64) -> Result<TlbIpiWork, TlbIpiError> {
        let work = self.decode_work()?;
        if work.generation != generation {
            return Err(TlbIpiError::StaleGeneration {
                expected: work.generation,
                actual: generation,
            });
        }
        match self.state()? {
            TlbIpiState::Active => {}
            TlbIpiState::TimedOut => return Err(TlbIpiError::TimedOut),
            _ => return Err(TlbIpiError::InvalidState),
        }
        let acknowledged = self.acknowledged_mask();
        if acknowledged != work.targets {
            return Err(TlbIpiError::Incomplete(
                work.targets.difference(acknowledged),
            ));
        }
        self.state
            .compare_exchange(
                TlbIpiState::Active as u8,
                TlbIpiState::Complete as u8,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .map_err(|_| TlbIpiError::InvalidState)?;
        Ok(work)
    }

    pub fn mark_timed_out(&self, generation: u64) -> Result<CpuMask, TlbIpiError> {
        let work = self.decode_work()?;
        if work.generation != generation {
            return Err(TlbIpiError::StaleGeneration {
                expected: work.generation,
                actual: generation,
            });
        }
        self.state
            .compare_exchange(
                TlbIpiState::Active as u8,
                TlbIpiState::TimedOut as u8,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .map_err(|_| TlbIpiError::InvalidState)?;
        Ok(work.targets.difference(self.acknowledged_mask()))
    }

    pub fn reset_complete(&self) -> Result<(), TlbIpiError> {
        self.state
            .compare_exchange(
                TlbIpiState::Complete as u8,
                TlbIpiState::Idle as u8,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .map_err(|_| TlbIpiError::InvalidState)?;
        Ok(())
    }

    pub fn acknowledged_mask(&self) -> CpuMask {
        let mut words = [0; CPU_MASK_WORD_COUNT];
        for (output, slot) in words.iter_mut().zip(&self.acknowledged) {
            *output = slot.load(Ordering::Acquire);
        }
        CpuMask::from_words(words)
    }

    fn decode_work(&self) -> Result<TlbIpiWork, TlbIpiError> {
        let generation = self.generation.load(Ordering::Relaxed);
        let address_space = TlbAddressSpace::new(
            self.root.load(Ordering::Relaxed),
            self.address_space_generation.load(Ordering::Relaxed),
        )
        .ok_or(TlbIpiError::CorruptRequest)?;
        let scope = decode_scope(
            self.scope_kind.load(Ordering::Relaxed),
            self.scope_start.load(Ordering::Relaxed),
            self.scope_pages.load(Ordering::Relaxed),
        )?;
        let initiator = CpuIndex::new(self.initiator.load(Ordering::Relaxed))
            .ok_or(TlbIpiError::CorruptRequest)?;
        let mut words = [0; CPU_MASK_WORD_COUNT];
        for (output, slot) in words.iter_mut().zip(&self.targets) {
            *output = slot.load(Ordering::Relaxed);
        }
        let targets = CpuMask::from_words(words);
        if generation == 0 || targets.is_empty() || targets.contains(initiator) {
            return Err(TlbIpiError::CorruptRequest);
        }
        Ok(TlbIpiWork {
            generation,
            address_space,
            scope,
            initiator,
            targets,
        })
    }
}

impl Default for TlbIpiMailbox {
    fn default() -> Self {
        Self::new()
    }
}

fn encode_scope(scope: TlbFlushScope) -> (u8, u64, u16) {
    match scope.kind() {
        TlbFlushKind::Page => (0, scope.start().unwrap_or(0), 1),
        TlbFlushKind::Range => (
            1,
            scope.start().unwrap_or(0),
            scope.page_count().unwrap_or(0),
        ),
        TlbFlushKind::AddressSpace => (2, 0, 0),
        TlbFlushKind::AllContexts => (3, 0, 0),
    }
}

fn decode_scope(kind: u8, start: u64, pages: u16) -> Result<TlbFlushScope, TlbIpiError> {
    match kind {
        0 => TlbFlushScope::page(start),
        1 => TlbFlushScope::range(start, pages),
        2 if start == 0 && pages == 0 => Some(TlbFlushScope::whole_address_space()),
        3 if start == 0 && pages == 0 => Some(TlbFlushScope::all_contexts()),
        _ => None,
    }
    .ok_or(TlbIpiError::CorruptRequest)
}
