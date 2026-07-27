//! Typed PCI Base Address Register values.
//!
//! This architecture-layer module retains only restored BAR observations:
//! slot, address-space kind, assigned base, and decoded power-of-two size.
//! Configuration mutation and rollback live in the probe module.

pub const PCI_BAR_CAPACITY: usize = 6;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct PciBarIndex(u8);

impl PciBarIndex {
    pub const fn new(index: u8) -> Option<Self> {
        if index < PCI_BAR_CAPACITY as u8 {
            Some(Self(index))
        } else {
            None
        }
    }

    pub const fn number(self) -> u8 {
        self.0
    }

    pub(super) const fn from_probe(index: u8) -> Self {
        Self(index)
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PciBarKind {
    Io,
    MemoryBelowOneMegabyte { prefetchable: bool },
    Memory32 { prefetchable: bool },
    Memory64 { prefetchable: bool },
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct PciBar {
    index: PciBarIndex,
    kind: PciBarKind,
    base: u64,
    size: u64,
}

impl PciBar {
    pub const fn index(self) -> PciBarIndex {
        self.index
    }

    pub const fn kind(self) -> PciBarKind {
        self.kind
    }

    pub const fn base(self) -> u64 {
        self.base
    }

    pub const fn size(self) -> u64 {
        self.size
    }

    pub const fn is_assigned(self) -> bool {
        self.base != 0
    }

    pub const fn end(self) -> Option<u64> {
        self.base.checked_add(self.size - 1)
    }

    pub(super) const fn new(index: PciBarIndex, kind: PciBarKind, base: u64, size: u64) -> Self {
        Self {
            index,
            kind,
            base,
            size,
        }
    }

    const EMPTY: Self = Self::new(PciBarIndex(0), PciBarKind::Io, 0, 0);
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct PciBarSet {
    bars: [PciBar; PCI_BAR_CAPACITY],
    len: usize,
}

impl PciBarSet {
    pub(super) const fn new() -> Self {
        Self {
            bars: [PciBar::EMPTY; PCI_BAR_CAPACITY],
            len: 0,
        }
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn bars(&self) -> &[PciBar] {
        &self.bars[..self.len]
    }

    pub fn get(&self, index: PciBarIndex) -> Option<PciBar> {
        self.bars().iter().copied().find(|bar| bar.index() == index)
    }

    pub fn all_assigned(&self) -> bool {
        !self.is_empty() && self.bars().iter().all(|bar| bar.is_assigned())
    }

    pub(super) fn push(&mut self, bar: PciBar) {
        self.bars[self.len] = bar;
        self.len += 1;
    }

    pub(super) const EMPTY: Self = Self::new();
}
