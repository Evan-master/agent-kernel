//! Fixed user-memory contract shared by isolated ring-3 Agent contexts.
//!
//! This architecture-library module defines virtual addresses and immutable
//! proof-program bytes. The bare-metal mapper owns physical allocation and page
//! permissions; host tests validate this pure layout.

use crate::address_space::{p4_index, AGENT_REGION_BASE};

pub use crate::address_space::AGENT_CODE_PAGE_COUNT;

pub const PAGE_BYTES: u64 = 4096;
pub const STACK_PAGE_COUNT: usize = 4;
pub const AGENT_CALL_RELEASE_OFFSET: usize = 0;
pub const PHYSICAL_QUANTUM_GENERATION_OFFSET: usize = 1;
pub const AGENT_RESTART_GENERATION_OFFSET: usize = 2;
pub const FIRST_AGENT_RESTART_GENERATION: u8 = 1;
pub const SECOND_AGENT_RESTART_GENERATION: u8 = 2;
pub const THIRD_AGENT_RESTART_GENERATION: u8 = 3;
pub const MAX_AGENT_RESTART_GENERATION: u8 = THIRD_AGENT_RESTART_GENERATION;
pub const LAZY_DATA_PROOF_VALUE: u8 = 0x5a;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct UserMemoryLayout {
    code_start: u64,
    signal_start: u64,
    guard_start: u64,
    stack_bottom: u64,
    stack_top: u64,
    lazy_data_start: u64,
    runtime_page_start: u64,
    runtime_region_start: u64,
    call_data_start: u64,
}

impl UserMemoryLayout {
    pub const fn fixed() -> Self {
        let code_start = AGENT_REGION_BASE;
        let signal_start = code_start + PAGE_BYTES * AGENT_CODE_PAGE_COUNT as u64;
        let guard_start = signal_start + PAGE_BYTES;
        let stack_bottom = guard_start + PAGE_BYTES;
        let stack_top = stack_bottom + PAGE_BYTES * STACK_PAGE_COUNT as u64;
        let lazy_data_start = stack_top;
        let runtime_page_start = lazy_data_start + PAGE_BYTES;
        let runtime_region_start = runtime_page_start + PAGE_BYTES;
        let call_data_start = runtime_region_start
            + PAGE_BYTES * crate::runtime_region::RUNTIME_REGION_SLOT_COUNT as u64;
        Self {
            code_start,
            signal_start,
            guard_start,
            stack_bottom,
            stack_top,
            lazy_data_start,
            runtime_page_start,
            runtime_region_start,
            call_data_start,
        }
    }

    pub const fn code_start(self) -> u64 {
        self.code_start
    }

    pub const fn code_end(self) -> u64 {
        self.signal_start
    }

    pub const fn code_page_start(self, page: usize) -> Option<u64> {
        if page >= AGENT_CODE_PAGE_COUNT {
            None
        } else {
            Some(self.code_start + PAGE_BYTES * page as u64)
        }
    }

    pub const fn signal_start(self) -> u64 {
        self.signal_start
    }

    pub const fn guard_start(self) -> u64 {
        self.guard_start
    }

    pub const fn stack_bottom(self) -> u64 {
        self.stack_bottom
    }

    pub const fn stack_top(self) -> u64 {
        self.stack_top
    }

    pub const fn lazy_data_start(self) -> u64 {
        self.lazy_data_start
    }

    pub const fn runtime_page_start(self) -> u64 {
        self.runtime_page_start
    }

    pub const fn runtime_region_start(self) -> u64 {
        self.runtime_region_start
    }

    pub const fn runtime_region_end(self) -> u64 {
        self.call_data_start
    }

    pub const fn call_data_start(self) -> u64 {
        self.call_data_start
    }

    pub const fn call_data_end(self) -> u64 {
        self.call_data_start + PAGE_BYTES
    }

    pub const fn runtime_region_page_start(self, slot: usize) -> Option<u64> {
        if slot >= crate::runtime_region::RUNTIME_REGION_SLOT_COUNT {
            None
        } else {
            Some(self.runtime_region_start + PAGE_BYTES * slot as u64)
        }
    }

    pub const fn p4_index(self) -> usize {
        p4_index(self.code_start)
    }

    pub const fn last_mapped_p4_index(self) -> usize {
        p4_index(self.call_data_end() - 1)
    }

    pub const fn contains_code(self, address: u64) -> bool {
        address >= self.code_start && address < self.code_end()
    }

    pub const fn contains_stack(self, address: u64) -> bool {
        address >= self.stack_bottom && address < self.stack_top
    }

    pub const fn contains_stack_pointer(self, address: u64) -> bool {
        address > self.stack_bottom && address <= self.stack_top
    }

    pub const fn contains_lazy_data(self, address: u64) -> bool {
        address >= self.lazy_data_start && address < self.lazy_data_start + PAGE_BYTES
    }

    pub const fn contains_runtime_page(self, address: u64) -> bool {
        address >= self.runtime_page_start && address < self.runtime_page_start + PAGE_BYTES
    }

    pub const fn contains_runtime_region(self, address: u64) -> bool {
        address >= self.runtime_region_start && address < self.runtime_region_end()
    }

    pub const fn contains_call_data(self, address: u64) -> bool {
        address >= self.call_data_start && address < self.call_data_end()
    }
}
