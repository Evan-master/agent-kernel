//! Generation-bound BSP-to-AP startup handoff.
//!
//! The fixed layout is shared with the real/protected/long-mode trampoline.
//! Input fields are published before `Prepared`; terminal evidence is visible
//! only after an exact CPU, APIC ID, and generation acknowledgement.

use core::{
    mem::offset_of,
    sync::atomic::{AtomicU32, AtomicU64, Ordering},
};

use super::{ApicId, CpuIndex};

const AP_STARTUP_MAGIC: u64 = u64::from_le_bytes(*b"AKAPV12\0");
const CR3_ROOT_MASK: u64 = 0x000f_ffff_ffff_f000;
const CR3_CONTROL_MASK: u64 = (1 << 3) | (1 << 4);
const PAGE_MASK: u64 = 4095;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ApStartupStatus {
    Empty = 0,
    Writing = 1,
    Prepared = 2,
    Online = 3,
    Failed = 4,
}

impl ApStartupStatus {
    fn from_raw(raw: u32) -> Option<Self> {
        match raw {
            0 => Some(Self::Empty),
            1 => Some(Self::Writing),
            2 => Some(Self::Prepared),
            3 => Some(Self::Online),
            4 => Some(Self::Failed),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApStartupHandoffError {
    InvalidCpu,
    InvalidGeneration,
    InvalidCr3,
    InvalidStack,
    InvalidEntry,
    InvalidLocalApicBase,
    InvalidTimerInitialCount,
    InvalidPhysicalOffset,
    InvalidState,
    IdentityMismatch,
    CorruptState,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ApStartupDescriptor {
    cpu: CpuIndex,
    apic_id: ApicId,
    generation: u64,
    cr3: u64,
    stack_top: u64,
    entry: u64,
    local_apic_base: u64,
    timer_initial_count: u32,
    physical_offset: u64,
}

impl ApStartupDescriptor {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        cpu: CpuIndex,
        apic_id: ApicId,
        generation: u64,
        cr3: u64,
        stack_top: u64,
        entry: u64,
        local_apic_base: u64,
        timer_initial_count: u32,
        physical_offset: u64,
    ) -> Result<Self, ApStartupHandoffError> {
        if cpu == CpuIndex::BSP {
            return Err(ApStartupHandoffError::InvalidCpu);
        }
        if generation == 0 {
            return Err(ApStartupHandoffError::InvalidGeneration);
        }
        let cr3_root = cr3 & CR3_ROOT_MASK;
        if cr3_root == 0
            || cr3_root > u32::MAX as u64
            || cr3 & !(CR3_ROOT_MASK | CR3_CONTROL_MASK) != 0
        {
            return Err(ApStartupHandoffError::InvalidCr3);
        }
        if stack_top == 0 || stack_top & 15 != 0 || !canonical(stack_top) {
            return Err(ApStartupHandoffError::InvalidStack);
        }
        if entry == 0 || !canonical(entry) {
            return Err(ApStartupHandoffError::InvalidEntry);
        }
        if local_apic_base == 0 || local_apic_base & PAGE_MASK != 0 {
            return Err(ApStartupHandoffError::InvalidLocalApicBase);
        }
        if timer_initial_count == 0 {
            return Err(ApStartupHandoffError::InvalidTimerInitialCount);
        }
        let Some(local_apic_virtual) = physical_offset.checked_add(local_apic_base) else {
            return Err(ApStartupHandoffError::InvalidPhysicalOffset);
        };
        if !canonical(physical_offset) || !canonical(local_apic_virtual) {
            return Err(ApStartupHandoffError::InvalidPhysicalOffset);
        }
        Ok(Self {
            cpu,
            apic_id,
            generation,
            cr3,
            stack_top,
            entry,
            local_apic_base,
            timer_initial_count,
            physical_offset,
        })
    }

    pub const fn cpu(self) -> CpuIndex {
        self.cpu
    }

    pub const fn apic_id(self) -> ApicId {
        self.apic_id
    }

    pub const fn generation(self) -> u64 {
        self.generation
    }

    pub const fn cr3(self) -> u64 {
        self.cr3
    }

    pub const fn stack_top(self) -> u64 {
        self.stack_top
    }

    pub const fn entry(self) -> u64 {
        self.entry
    }

    pub const fn local_apic_base(self) -> u64 {
        self.local_apic_base
    }

    pub const fn timer_initial_count(self) -> u32 {
        self.timer_initial_count
    }

    pub const fn physical_offset(self) -> u64 {
        self.physical_offset
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ApStartupEvidence {
    pub cpu: CpuIndex,
    pub apic_id: ApicId,
    pub generation: u64,
    pub privileged_stack_start: u64,
    pub privileged_stack_end: u64,
    pub transition_slot: u64,
}

#[repr(C, align(64))]
pub struct ApStartupHandoff {
    magic: AtomicU64,
    status: AtomicU32,
    cpu_index: AtomicU32,
    apic_id: AtomicU32,
    reserved_input: AtomicU32,
    generation: AtomicU64,
    cr3: AtomicU64,
    stack_top: AtomicU64,
    entry: AtomicU64,
    local_apic_base: AtomicU64,
    physical_offset: AtomicU64,
    observed_stack_start: AtomicU64,
    observed_stack_end: AtomicU64,
    observed_transition_slot: AtomicU64,
    observed_apic_id: AtomicU32,
    reserved_output: AtomicU32,
}

impl ApStartupHandoff {
    pub const fn new() -> Self {
        Self {
            magic: AtomicU64::new(0),
            status: AtomicU32::new(ApStartupStatus::Empty as u32),
            cpu_index: AtomicU32::new(0),
            apic_id: AtomicU32::new(0),
            reserved_input: AtomicU32::new(0),
            generation: AtomicU64::new(0),
            cr3: AtomicU64::new(0),
            stack_top: AtomicU64::new(0),
            entry: AtomicU64::new(0),
            local_apic_base: AtomicU64::new(0),
            physical_offset: AtomicU64::new(0),
            observed_stack_start: AtomicU64::new(0),
            observed_stack_end: AtomicU64::new(0),
            observed_transition_slot: AtomicU64::new(0),
            observed_apic_id: AtomicU32::new(0),
            reserved_output: AtomicU32::new(0),
        }
    }

    pub fn status(&self) -> Result<ApStartupStatus, ApStartupHandoffError> {
        ApStartupStatus::from_raw(self.status.load(Ordering::Acquire))
            .ok_or(ApStartupHandoffError::CorruptState)
    }

    pub fn prepare(&self, descriptor: ApStartupDescriptor) -> Result<(), ApStartupHandoffError> {
        self.status
            .compare_exchange(
                ApStartupStatus::Empty as u32,
                ApStartupStatus::Writing as u32,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .map_err(|_| ApStartupHandoffError::InvalidState)?;
        self.magic.store(AP_STARTUP_MAGIC, Ordering::Relaxed);
        self.cpu_index
            .store(u32::from(descriptor.cpu.get()), Ordering::Relaxed);
        self.apic_id
            .store(descriptor.apic_id.get(), Ordering::Relaxed);
        self.generation
            .store(descriptor.generation, Ordering::Relaxed);
        self.cr3.store(descriptor.cr3, Ordering::Relaxed);
        self.stack_top
            .store(descriptor.stack_top, Ordering::Relaxed);
        self.entry.store(descriptor.entry, Ordering::Relaxed);
        self.local_apic_base
            .store(descriptor.local_apic_base, Ordering::Relaxed);
        self.reserved_input
            .store(descriptor.timer_initial_count, Ordering::Relaxed);
        self.physical_offset
            .store(descriptor.physical_offset, Ordering::Relaxed);
        self.observed_stack_start.store(0, Ordering::Relaxed);
        self.observed_stack_end.store(0, Ordering::Relaxed);
        self.observed_transition_slot.store(0, Ordering::Relaxed);
        self.observed_apic_id.store(0, Ordering::Relaxed);
        self.status
            .store(ApStartupStatus::Prepared as u32, Ordering::Release);
        Ok(())
    }

    pub fn descriptor(&self) -> Result<ApStartupDescriptor, ApStartupHandoffError> {
        if self.status()? != ApStartupStatus::Prepared
            || self.magic.load(Ordering::Relaxed) != AP_STARTUP_MAGIC
        {
            return Err(ApStartupHandoffError::InvalidState);
        }
        let cpu_raw = self.cpu_index.load(Ordering::Relaxed);
        let cpu = u16::try_from(cpu_raw)
            .ok()
            .and_then(CpuIndex::new)
            .ok_or(ApStartupHandoffError::IdentityMismatch)?;
        ApStartupDescriptor::new(
            cpu,
            ApicId::new(self.apic_id.load(Ordering::Relaxed)),
            self.generation.load(Ordering::Relaxed),
            self.cr3.load(Ordering::Relaxed),
            self.stack_top.load(Ordering::Relaxed),
            self.entry.load(Ordering::Relaxed),
            self.local_apic_base.load(Ordering::Relaxed),
            self.reserved_input.load(Ordering::Relaxed),
            self.physical_offset.load(Ordering::Relaxed),
        )
    }

    pub fn acknowledge_online(
        &self,
        evidence: ApStartupEvidence,
    ) -> Result<(), ApStartupHandoffError> {
        let expected = self.descriptor()?;
        if evidence.cpu != expected.cpu
            || evidence.apic_id != expected.apic_id
            || evidence.generation != expected.generation
            || evidence.privileged_stack_start == 0
            || evidence.privileged_stack_end <= evidence.privileged_stack_start
            || evidence.transition_slot == 0
        {
            return Err(ApStartupHandoffError::IdentityMismatch);
        }
        self.observed_stack_start
            .store(evidence.privileged_stack_start, Ordering::Relaxed);
        self.observed_stack_end
            .store(evidence.privileged_stack_end, Ordering::Relaxed);
        self.observed_transition_slot
            .store(evidence.transition_slot, Ordering::Relaxed);
        self.observed_apic_id
            .store(evidence.apic_id.get(), Ordering::Relaxed);
        self.status
            .store(ApStartupStatus::Online as u32, Ordering::Release);
        Ok(())
    }

    pub fn fail(&self, cpu: CpuIndex, generation: u64) -> Result<(), ApStartupHandoffError> {
        let expected = self.descriptor()?;
        if cpu != expected.cpu || generation != expected.generation {
            return Err(ApStartupHandoffError::IdentityMismatch);
        }
        self.status
            .store(ApStartupStatus::Failed as u32, Ordering::Release);
        Ok(())
    }

    pub fn evidence(&self) -> Result<ApStartupEvidence, ApStartupHandoffError> {
        if self.status()? != ApStartupStatus::Online {
            return Err(ApStartupHandoffError::InvalidState);
        }
        let cpu_raw = self.cpu_index.load(Ordering::Relaxed);
        let cpu = u16::try_from(cpu_raw)
            .ok()
            .and_then(CpuIndex::new)
            .ok_or(ApStartupHandoffError::IdentityMismatch)?;
        let apic_id = ApicId::new(self.observed_apic_id.load(Ordering::Relaxed));
        if apic_id.get() != self.apic_id.load(Ordering::Relaxed) {
            return Err(ApStartupHandoffError::IdentityMismatch);
        }
        Ok(ApStartupEvidence {
            cpu,
            apic_id,
            generation: self.generation.load(Ordering::Relaxed),
            privileged_stack_start: self.observed_stack_start.load(Ordering::Relaxed),
            privileged_stack_end: self.observed_stack_end.load(Ordering::Relaxed),
            transition_slot: self.observed_transition_slot.load(Ordering::Relaxed),
        })
    }

    pub fn reset_terminal(&self) -> Result<(), ApStartupHandoffError> {
        let state = self.status()?;
        if state != ApStartupStatus::Online && state != ApStartupStatus::Failed {
            return Err(ApStartupHandoffError::InvalidState);
        }
        self.status
            .compare_exchange(
                state as u32,
                ApStartupStatus::Empty as u32,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .map_err(|_| ApStartupHandoffError::InvalidState)?;
        Ok(())
    }
}

impl Default for ApStartupHandoff {
    fn default() -> Self {
        Self::new()
    }
}

const fn canonical(address: u64) -> bool {
    let upper = address >> 48;
    let sign = (address >> 47) & 1;
    (sign == 0 && upper == 0) || (sign == 1 && upper == 0xffff)
}

pub const AP_HANDOFF_STATUS_OFFSET: usize = offset_of!(ApStartupHandoff, status);
pub const AP_HANDOFF_CPU_INDEX_OFFSET: usize = offset_of!(ApStartupHandoff, cpu_index);
pub const AP_HANDOFF_APIC_ID_OFFSET: usize = offset_of!(ApStartupHandoff, apic_id);
pub const AP_HANDOFF_GENERATION_OFFSET: usize = offset_of!(ApStartupHandoff, generation);
pub const AP_HANDOFF_CR3_OFFSET: usize = offset_of!(ApStartupHandoff, cr3);
pub const AP_HANDOFF_STACK_TOP_OFFSET: usize = offset_of!(ApStartupHandoff, stack_top);
pub const AP_HANDOFF_ENTRY_OFFSET: usize = offset_of!(ApStartupHandoff, entry);
pub const AP_HANDOFF_LOCAL_APIC_BASE_OFFSET: usize = offset_of!(ApStartupHandoff, local_apic_base);
pub const AP_HANDOFF_PHYSICAL_OFFSET_OFFSET: usize = offset_of!(ApStartupHandoff, physical_offset);
pub const AP_STARTUP_STATUS_PREPARED: u32 = ApStartupStatus::Prepared as u32;
