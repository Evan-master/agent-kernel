//! Validated values carried by a TLB shootdown transaction.
//!
//! Address-space generations distinguish reused CR3 frames, and private scope
//! fields prevent malformed ranges from reaching architecture invalidation
//! instructions.

use crate::{
    address_space::PAGE_TABLE_BYTES,
    cpu::{CpuIndex, CpuMask},
};

pub const MAX_TLB_RANGE_PAGES: u16 = 64;
const CR3_ROOT_MASK: u64 = 0x000f_ffff_ffff_f000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TlbAddressSpace {
    root: u64,
    generation: u64,
}

impl TlbAddressSpace {
    pub const fn new(root: u64, generation: u64) -> Option<Self> {
        if root == 0
            || root & (PAGE_TABLE_BYTES - 1) != 0
            || root & !CR3_ROOT_MASK != 0
            || generation == 0
        {
            None
        } else {
            Some(Self { root, generation })
        }
    }

    pub const fn root(self) -> u64 {
        self.root
    }

    pub const fn generation(self) -> u64 {
        self.generation
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TlbFlushKind {
    Page,
    Range,
    AddressSpace,
    AllContexts,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TlbFlushScope {
    kind: TlbFlushKind,
    start: u64,
    page_count: u16,
}

impl TlbFlushScope {
    pub const fn page(address: u64) -> Option<Self> {
        Self::range_with_kind(address, 1, TlbFlushKind::Page)
    }

    pub const fn range(start: u64, page_count: u16) -> Option<Self> {
        Self::range_with_kind(start, page_count, TlbFlushKind::Range)
    }

    pub const fn whole_address_space() -> Self {
        Self {
            kind: TlbFlushKind::AddressSpace,
            start: 0,
            page_count: 0,
        }
    }

    pub const fn all_contexts() -> Self {
        Self {
            kind: TlbFlushKind::AllContexts,
            start: 0,
            page_count: 0,
        }
    }

    pub const fn kind(self) -> TlbFlushKind {
        self.kind
    }

    pub const fn start(self) -> Option<u64> {
        match self.kind {
            TlbFlushKind::Page | TlbFlushKind::Range => Some(self.start),
            TlbFlushKind::AddressSpace | TlbFlushKind::AllContexts => None,
        }
    }

    pub const fn page_count(self) -> Option<u16> {
        match self.kind {
            TlbFlushKind::Page | TlbFlushKind::Range => Some(self.page_count),
            TlbFlushKind::AddressSpace | TlbFlushKind::AllContexts => None,
        }
    }

    const fn range_with_kind(start: u64, page_count: u16, kind: TlbFlushKind) -> Option<Self> {
        if page_count == 0
            || page_count > MAX_TLB_RANGE_PAGES
            || start & (PAGE_TABLE_BYTES - 1) != 0
            || !canonical(start)
        {
            return None;
        }
        let span = match (page_count as u64 - 1).checked_mul(PAGE_TABLE_BYTES) {
            Some(value) => value,
            None => return None,
        };
        let last = match start.checked_add(span) {
            Some(value) => value,
            None => return None,
        };
        if !canonical(last) {
            return None;
        }
        Some(Self {
            kind,
            start,
            page_count,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TlbShootdownStatus {
    AwaitingAcknowledgements,
    Complete,
    TimedOut,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TlbShootdownRequest {
    generation: u64,
    address_space: TlbAddressSpace,
    scope: TlbFlushScope,
    initiator: CpuIndex,
    targets: CpuMask,
    status: TlbShootdownStatus,
}

impl TlbShootdownRequest {
    pub(super) const fn new(
        generation: u64,
        address_space: TlbAddressSpace,
        scope: TlbFlushScope,
        initiator: CpuIndex,
        targets: CpuMask,
    ) -> Self {
        let status = if targets.is_empty() {
            TlbShootdownStatus::Complete
        } else {
            TlbShootdownStatus::AwaitingAcknowledgements
        };
        Self {
            generation,
            address_space,
            scope,
            initiator,
            targets,
            status,
        }
    }

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

    pub const fn status(self) -> TlbShootdownStatus {
        self.status
    }

    pub(super) fn set_status(&mut self, status: TlbShootdownStatus) {
        self.status = status;
    }
}

const fn canonical(address: u64) -> bool {
    let upper = address >> 48;
    let sign = (address >> 47) & 1;
    (sign == 0 && upper == 0) || (sign == 1 && upper == 0xffff)
}
