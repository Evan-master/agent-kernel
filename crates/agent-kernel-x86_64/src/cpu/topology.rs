//! Deterministic firmware CPU topology construction.
//!
//! The builder filters unusable MADT processors, rejects ambiguous identities,
//! and freezes a dense logical topology with the boot processor at index zero.
//! No allocation or hardware access occurs in this module.

use super::{ApicId, CpuIndex, CpuMask, FirmwareProcessor, MAX_CPU_COUNT};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TopologyInsert {
    Accepted,
    IgnoredDisabled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TopologyError {
    DuplicateApicId(ApicId),
    DuplicateProcessorUid(u32),
    CapacityExceeded,
    BootProcessorMissing(ApicId),
    BootProcessorDisabled(ApicId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CpuDescriptor {
    index: CpuIndex,
    processor: FirmwareProcessor,
}

impl CpuDescriptor {
    pub const fn index(self) -> CpuIndex {
        self.index
    }

    pub const fn processor(self) -> FirmwareProcessor {
        self.processor
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CpuTopology<const CAPACITY: usize = MAX_CPU_COUNT> {
    entries: [Option<CpuDescriptor>; CAPACITY],
    bsp: CpuDescriptor,
    len: usize,
}

impl<const CAPACITY: usize> CpuTopology<CAPACITY> {
    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        false
    }

    pub const fn bsp(&self) -> CpuDescriptor {
        self.bsp
    }

    pub fn get(&self, index: CpuIndex) -> Option<CpuDescriptor> {
        self.entries.get(index.as_usize()).copied().flatten()
    }

    pub fn index_for_apic_id(&self, apic_id: ApicId) -> Option<CpuIndex> {
        self.entries[..self.len]
            .iter()
            .flatten()
            .find(|entry| entry.processor.apic_id() == apic_id)
            .map(|entry| entry.index)
    }

    pub fn present_mask(&self) -> CpuMask {
        let mut mask = CpuMask::empty();
        for entry in self.entries[..self.len].iter().flatten() {
            mask.insert(entry.index);
        }
        mask
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CpuTopologyBuilder<const CAPACITY: usize = MAX_CPU_COUNT> {
    entries: [Option<FirmwareProcessor>; CAPACITY],
    len: usize,
}

impl<const CAPACITY: usize> CpuTopologyBuilder<CAPACITY> {
    pub const fn new() -> Self {
        Self {
            entries: [None; CAPACITY],
            len: 0,
        }
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn insert(
        &mut self,
        processor: FirmwareProcessor,
    ) -> Result<TopologyInsert, TopologyError> {
        if !processor.flags().usable() {
            return Ok(TopologyInsert::IgnoredDisabled);
        }
        for existing in self.entries[..self.len].iter().flatten() {
            if existing.apic_id() == processor.apic_id() {
                return Err(TopologyError::DuplicateApicId(processor.apic_id()));
            }
            if existing.uid() == processor.uid() {
                return Err(TopologyError::DuplicateProcessorUid(processor.uid()));
            }
        }
        if self.len >= CAPACITY || self.len >= MAX_CPU_COUNT {
            return Err(TopologyError::CapacityExceeded);
        }
        self.entries[self.len] = Some(processor);
        self.len += 1;
        Ok(TopologyInsert::Accepted)
    }

    pub fn freeze(self, bsp_apic_id: ApicId) -> Result<CpuTopology<CAPACITY>, TopologyError> {
        let Some(bsp_source_index) = self.entries[..self.len]
            .iter()
            .position(|entry| entry.is_some_and(|cpu| cpu.apic_id() == bsp_apic_id))
        else {
            return Err(TopologyError::BootProcessorMissing(bsp_apic_id));
        };
        let bsp_processor = self.entries[bsp_source_index]
            .ok_or(TopologyError::BootProcessorMissing(bsp_apic_id))?;
        if !bsp_processor.flags().enabled() {
            return Err(TopologyError::BootProcessorDisabled(bsp_apic_id));
        }

        let bsp = CpuDescriptor {
            index: CpuIndex::BSP,
            processor: bsp_processor,
        };
        let mut entries = [None; CAPACITY];
        entries[0] = Some(bsp);
        let mut destination = 1;
        for (source, processor) in self.entries[..self.len].iter().copied().enumerate() {
            if source == bsp_source_index {
                continue;
            }
            let processor = processor.ok_or(TopologyError::CapacityExceeded)?;
            let index = CpuIndex::new(destination as u16).ok_or(TopologyError::CapacityExceeded)?;
            entries[destination] = Some(CpuDescriptor { index, processor });
            destination += 1;
        }
        Ok(CpuTopology {
            entries,
            bsp,
            len: self.len,
        })
    }
}

impl<const CAPACITY: usize> Default for CpuTopologyBuilder<CAPACITY> {
    fn default() -> Self {
        Self::new()
    }
}
