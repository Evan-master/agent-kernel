//! Pure x86_64 privilege descriptor contracts.
//!
//! This architecture-library module encodes the permanent GDT, long-mode TSS,
//! and selectors consumed by the bare-metal installer. It performs no
//! privileged instruction, so host tests can lock every byte-level invariant.

use core::mem::size_of;

pub const GDT_ENTRY_COUNT: usize = 7;
pub const KERNEL_CODE_SELECTOR: u16 = 0x08;
pub const KERNEL_DATA_SELECTOR: u16 = 0x10;
pub const USER_DATA_SELECTOR: u16 = 0x1b;
pub const USER_CODE_SELECTOR: u16 = 0x23;
pub const TSS_SELECTOR: u16 = 0x28;

pub const PRIVILEGED_STACK_GUARD_BYTES: usize = 4096;
pub const PRIVILEGED_STACK_BYTES: usize = 32 * 1024;
pub const PRIVILEGED_STACK_SLOT_BYTES: usize =
    PRIVILEGED_STACK_GUARD_BYTES + PRIVILEGED_STACK_BYTES;

const LOWER_CANONICAL_END: usize = 0x0000_7fff_ffff_ffff;
const UPPER_CANONICAL_START: usize = 0xffff_8000_0000_0000;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct PrivilegedStackLayout {
    guard_start: usize,
    guard_end: usize,
    stack_start: usize,
    stack_end: usize,
}

impl PrivilegedStackLayout {
    pub const fn new(guard_start: usize) -> Option<Self> {
        if !guard_start.is_multiple_of(PRIVILEGED_STACK_GUARD_BYTES) {
            return None;
        }
        let Some(guard_end) = guard_start.checked_add(PRIVILEGED_STACK_GUARD_BYTES) else {
            return None;
        };
        let stack_start = guard_end;
        let Some(stack_end) = stack_start.checked_add(PRIVILEGED_STACK_BYTES) else {
            return None;
        };
        let Some(last_byte) = stack_end.checked_sub(1) else {
            return None;
        };
        if !canonical(guard_start) || !canonical(last_byte) {
            return None;
        }
        Some(Self {
            guard_start,
            guard_end,
            stack_start,
            stack_end,
        })
    }

    pub const fn guard_start(self) -> usize {
        self.guard_start
    }

    pub const fn guard_end(self) -> usize {
        self.guard_end
    }

    pub const fn stack_start(self) -> usize {
        self.stack_start
    }

    pub const fn stack_end(self) -> usize {
        self.stack_end
    }

    pub const fn contains_stack_address(self, address: usize) -> bool {
        address >= self.stack_start && address < self.stack_end
    }
}

const fn canonical(address: usize) -> bool {
    address <= LOWER_CANONICAL_END || address >= UPPER_CANONICAL_START
}

pub const KERNEL_CODE_DESCRIPTOR: u64 = 0x00af_9a00_0000_ffff;
pub const KERNEL_DATA_DESCRIPTOR: u64 = 0x00cf_9200_0000_ffff;
pub const USER_DATA_DESCRIPTOR: u64 = 0x00cf_f200_0000_ffff;
pub const USER_CODE_DESCRIPTOR: u64 = 0x00af_fa00_0000_ffff;

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct TaskStateSegment64 {
    reserved_0: u32,
    pub rsp0: u64,
    rsp1: u64,
    rsp2: u64,
    reserved_1: u64,
    ist1: u64,
    ist2: u64,
    ist3: u64,
    ist4: u64,
    ist5: u64,
    ist6: u64,
    ist7: u64,
    reserved_2: u64,
    reserved_3: u16,
    pub iomap_base: u16,
}

impl TaskStateSegment64 {
    pub const fn new(rsp0: u64) -> Self {
        Self {
            reserved_0: 0,
            rsp0,
            rsp1: 0,
            rsp2: 0,
            reserved_1: 0,
            ist1: 0,
            ist2: 0,
            ist3: 0,
            ist4: 0,
            ist5: 0,
            ist6: 0,
            ist7: 0,
            reserved_2: 0,
            reserved_3: 0,
            iomap_base: size_of::<Self>() as u16,
        }
    }

    pub fn rsp0(&self) -> u64 {
        // SAFETY: packed TSS fields must be copied with an unaligned read.
        unsafe { core::ptr::addr_of!(self.rsp0).read_unaligned() }
    }

    pub fn iomap_base(&self) -> u16 {
        // SAFETY: packed TSS fields must be copied with an unaligned read.
        unsafe { core::ptr::addr_of!(self.iomap_base).read_unaligned() }
    }
}

pub const fn tss_descriptor(base: u64) -> (u64, u64) {
    let limit = (size_of::<TaskStateSegment64>() - 1) as u64;
    let low = (limit & 0xffff)
        | ((base & 0x00ff_ffff) << 16)
        | (0x89 << 40)
        | (((limit >> 16) & 0x0f) << 48)
        | (((base >> 24) & 0xff) << 56);
    let high = (base >> 32) & 0xffff_ffff;
    (low, high)
}

pub const fn gdt_entries(tss_base: u64) -> [u64; GDT_ENTRY_COUNT] {
    let (tss_low, tss_high) = tss_descriptor(tss_base);
    [
        0,
        KERNEL_CODE_DESCRIPTOR,
        KERNEL_DATA_DESCRIPTOR,
        USER_DATA_DESCRIPTOR,
        USER_CODE_DESCRIPTOR,
        tss_low,
        tss_high,
    ]
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct GdtPointer {
    limit: u16,
    base: u64,
}

impl GdtPointer {
    pub const fn for_table(base: u64, entry_count: usize) -> Option<Self> {
        if entry_count == 0 {
            return None;
        }
        let Some(bytes) = entry_count.checked_mul(size_of::<u64>()) else {
            return None;
        };
        let Some(limit) = bytes.checked_sub(1) else {
            return None;
        };
        if limit > u16::MAX as usize {
            return None;
        }
        Some(Self {
            limit: limit as u16,
            base,
        })
    }

    pub const fn limit(self) -> u16 {
        self.limit
    }

    pub const fn base(self) -> u64 {
        self.base
    }
}
