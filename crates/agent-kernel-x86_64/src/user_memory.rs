//! Fixed user-memory contract for the first ring-3 Agent proof.
//!
//! This architecture-library module defines virtual addresses and immutable
//! proof-program bytes. The bare-metal mapper owns physical allocation and page
//! permissions; host tests validate this pure layout.

use crate::address_space::{p4_index, AGENT_REGION_BASE};

pub const PAGE_BYTES: u64 = 4096;
pub const STACK_PAGE_COUNT: usize = 4;
pub const AGENT_CODE_BYTES: usize = 21;
pub const AGENT_CALL_RETURN_OFFSET: u64 = 19;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct UserMemoryLayout {
    code_start: u64,
    signal_start: u64,
    guard_start: u64,
    stack_bottom: u64,
    stack_top: u64,
}

impl UserMemoryLayout {
    pub const fn fixed() -> Self {
        let code_start = AGENT_REGION_BASE;
        let signal_start = code_start + PAGE_BYTES;
        let guard_start = signal_start + PAGE_BYTES;
        let stack_bottom = guard_start + PAGE_BYTES;
        let stack_top = stack_bottom + PAGE_BYTES * STACK_PAGE_COUNT as u64;
        Self {
            code_start,
            signal_start,
            guard_start,
            stack_bottom,
            stack_top,
        }
    }

    pub const fn code_start(self) -> u64 {
        self.code_start
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

    pub const fn p4_index(self) -> usize {
        p4_index(self.code_start)
    }

    pub const fn last_mapped_p4_index(self) -> usize {
        p4_index(self.stack_top - 1)
    }

    pub const fn contains_code(self, address: u64) -> bool {
        address >= self.code_start && address < self.code_start + PAGE_BYTES
    }

    pub const fn contains_stack(self, address: u64) -> bool {
        address >= self.stack_bottom && address < self.stack_top
    }

    pub const fn contains_stack_pointer(self, address: u64) -> bool {
        address > self.stack_bottom && address <= self.stack_top
    }
}

pub const fn agent_proof_program() -> [u8; AGENT_CODE_BYTES] {
    let signal = UserMemoryLayout::fixed().signal_start().to_le_bytes();
    [
        0x53, 0x5b, 0x48, 0xb8, signal[0], signal[1], signal[2], signal[3], signal[4], signal[5],
        signal[6], signal[7], 0x80, 0x38, 0x00, 0x74, 0xfb, 0xcd, 0x90, 0xeb, 0xfe,
    ]
}
