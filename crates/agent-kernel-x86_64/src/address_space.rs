//! Pure x86_64 Agent address-space identity contract.
//!
//! This architecture-library module defines the dedicated Agent P4 slot and
//! validates raw CR3 roots. It performs no privileged operation, so host tests
//! can lock the values consumed by bare-metal page-table and switch code.

pub const PAGE_TABLE_BYTES: u64 = 4096;
pub const P4_ENTRY_COUNT: usize = 512;
pub const AGENT_REGION_BASE: u64 = 0x0000_4000_0000_0000;
pub const AGENT_P4_INDEX: usize = p4_index(AGENT_REGION_BASE);

const P4_SHIFT: u32 = 39;
const P4_INDEX_MASK: u64 = 0x1ff;
const CR3_ROOT_MASK: u64 = 0x000f_ffff_ffff_f000;
const CR3_CONTROL_MASK: u64 = 0x0fff;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AddressSpaceKind {
    Kernel,
    Agent,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AddressSpaceRoots {
    kernel_root: u64,
    agent_root: u64,
    cr3_control: u64,
}

impl AddressSpaceRoots {
    pub const fn new(kernel_root: u64, agent_root: u64, cr3_control: u64) -> Option<Self> {
        if !valid_root(kernel_root)
            || !valid_root(agent_root)
            || kernel_root == agent_root
            || cr3_control & !CR3_CONTROL_MASK != 0
        {
            return None;
        }
        Some(Self {
            kernel_root,
            agent_root,
            cr3_control,
        })
    }

    pub const fn kernel_root(self) -> u64 {
        self.kernel_root
    }

    pub const fn agent_root(self) -> u64 {
        self.agent_root
    }

    pub const fn cr3_control(self) -> u64 {
        self.cr3_control
    }

    pub const fn kernel_cr3(self) -> u64 {
        self.kernel_root | self.cr3_control
    }

    pub const fn agent_cr3(self) -> u64 {
        self.agent_root | self.cr3_control
    }

    pub const fn classify(self, raw_cr3: u64) -> Option<AddressSpaceKind> {
        if raw_cr3 == self.kernel_cr3() {
            Some(AddressSpaceKind::Kernel)
        } else if raw_cr3 == self.agent_cr3() {
            Some(AddressSpaceKind::Agent)
        } else {
            None
        }
    }
}

pub const fn p4_index(virtual_address: u64) -> usize {
    ((virtual_address >> P4_SHIFT) & P4_INDEX_MASK) as usize
}

const fn valid_root(root: u64) -> bool {
    root & (PAGE_TABLE_BYTES - 1) == 0 && root & !CR3_ROOT_MASK == 0
}
