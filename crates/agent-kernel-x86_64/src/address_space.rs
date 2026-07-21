//! Pure x86_64 Agent address-space identity contract.
//!
//! This architecture-library module defines the dedicated Agent P4 slot and
//! validates raw CR3 roots. It performs no privileged operation, so host tests
//! can lock the values consumed by bare-metal page-table and switch code.

pub const PAGE_TABLE_BYTES: u64 = 4096;
pub const P4_ENTRY_COUNT: usize = 512;
pub const AGENT_PAGE_TABLE_FRAME_COUNT: usize = 4;
pub const AGENT_CODE_PAGE_CAPACITY: usize = 16;
pub const AGENT_STACK_FRAME_COUNT: usize = 4;
pub const AGENT_NON_CODE_FRAME_COUNT: usize = AGENT_STACK_FRAME_COUNT + 3;
pub const AGENT_CONTENT_FRAME_CAPACITY: usize =
    AGENT_CODE_PAGE_CAPACITY + AGENT_NON_CODE_FRAME_COUNT;
pub const AGENT_OWNED_FRAME_CAPACITY: usize =
    AGENT_PAGE_TABLE_FRAME_COUNT + AGENT_CONTENT_FRAME_CAPACITY;
pub const AGENT_REGION_BASE: u64 = 0x0000_4000_0000_0000;
pub const AGENT_P4_INDEX: usize = p4_index(AGENT_REGION_BASE);

const P4_SHIFT: u32 = 39;
const P4_INDEX_MASK: u64 = 0x1ff;
const CR3_ROOT_MASK: u64 = 0x000f_ffff_ffff_f000;
const CR3_CONTROL_MASK: u64 = (1 << 3) | (1 << 4);

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

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AgentMemoryIdentity {
    page_table_frames: [u64; AGENT_PAGE_TABLE_FRAME_COUNT],
    content_frames: [u64; AGENT_CONTENT_FRAME_CAPACITY],
    code_page_count: u8,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AgentFrameSet<const CAPACITY: usize> {
    frames: [u64; CAPACITY],
    len: usize,
}

impl<const CAPACITY: usize> AgentFrameSet<CAPACITY> {
    const fn new(frames: [u64; CAPACITY], len: usize) -> Self {
        Self { frames, len }
    }

    pub fn as_slice(&self) -> &[u64] {
        &self.frames[..self.len]
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl<const CAPACITY: usize> core::ops::Deref for AgentFrameSet<CAPACITY> {
    type Target = [u64];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<const CAPACITY: usize> IntoIterator for AgentFrameSet<CAPACITY> {
    type Item = u64;
    type IntoIter = core::iter::Take<core::array::IntoIter<u64, CAPACITY>>;

    fn into_iter(self) -> Self::IntoIter {
        self.frames.into_iter().take(self.len)
    }
}

impl AgentMemoryIdentity {
    pub const fn new(
        page_table_frames: [u64; AGENT_PAGE_TABLE_FRAME_COUNT],
        content_frames: [u64; AGENT_CONTENT_FRAME_CAPACITY],
        code_page_count: usize,
    ) -> Option<Self> {
        let Some(content_frame_count) = agent_content_frame_count(code_page_count) else {
            return None;
        };
        let mut table = 0;
        while table < AGENT_PAGE_TABLE_FRAME_COUNT {
            if !valid_root(page_table_frames[table]) {
                return None;
            }
            let mut prior = 0;
            while prior < table {
                if page_table_frames[table] == page_table_frames[prior] {
                    return None;
                }
                prior += 1;
            }
            table += 1;
        }
        let mut content = 0;
        while content < content_frame_count {
            if !valid_root(content_frames[content]) {
                return None;
            }
            let mut table = 0;
            while table < AGENT_PAGE_TABLE_FRAME_COUNT {
                if content_frames[content] == page_table_frames[table] {
                    return None;
                }
                table += 1;
            }
            let mut prior = 0;
            while prior < content {
                if content_frames[content] == content_frames[prior] {
                    return None;
                }
                prior += 1;
            }
            content += 1;
        }
        while content < AGENT_CONTENT_FRAME_CAPACITY {
            if content_frames[content] != 0 {
                return None;
            }
            content += 1;
        }
        Some(Self {
            page_table_frames,
            content_frames,
            code_page_count: code_page_count as u8,
        })
    }

    pub const fn root(self) -> u64 {
        self.page_table_frames[0]
    }

    pub const fn page_table_frames(self) -> [u64; AGENT_PAGE_TABLE_FRAME_COUNT] {
        self.page_table_frames
    }

    pub const fn code_page_count(self) -> usize {
        self.code_page_count as usize
    }

    pub const fn code_frames(self) -> AgentFrameSet<AGENT_CODE_PAGE_CAPACITY> {
        let mut frames = [0; AGENT_CODE_PAGE_CAPACITY];
        let mut index = 0;
        while index < self.code_page_count() {
            frames[index] = self.content_frames[index];
            index += 1;
        }
        AgentFrameSet::new(frames, self.code_page_count())
    }

    pub const fn content_frames(self) -> AgentFrameSet<AGENT_CONTENT_FRAME_CAPACITY> {
        AgentFrameSet::new(self.content_frames, self.content_frame_count())
    }

    pub const fn content_frame_count(self) -> usize {
        self.code_page_count() + AGENT_NON_CODE_FRAME_COUNT
    }

    pub const fn owned_frame_count(self) -> usize {
        AGENT_PAGE_TABLE_FRAME_COUNT + self.content_frame_count()
    }

    pub const fn signal_frame(self) -> u64 {
        self.content_frames[self.code_page_count()]
    }

    pub const fn stack_frames(self) -> [u64; AGENT_STACK_FRAME_COUNT] {
        let mut frames = [0; AGENT_STACK_FRAME_COUNT];
        let start = self.code_page_count() + 1;
        let mut index = 0;
        while index < AGENT_STACK_FRAME_COUNT {
            frames[index] = self.content_frames[start + index];
            index += 1;
        }
        frames
    }

    pub const fn lazy_data_frame(self) -> u64 {
        self.content_frames[self.code_page_count() + 1 + AGENT_STACK_FRAME_COUNT]
    }

    pub const fn call_data_frame(self) -> u64 {
        self.content_frames[self.code_page_count() + 2 + AGENT_STACK_FRAME_COUNT]
    }

    pub const fn owned_frames(self) -> AgentFrameSet<AGENT_OWNED_FRAME_CAPACITY> {
        let mut frames = [0; AGENT_OWNED_FRAME_CAPACITY];
        let mut index = 0;
        while index < AGENT_PAGE_TABLE_FRAME_COUNT {
            frames[index] = self.page_table_frames[index];
            index += 1;
        }
        let mut content = 0;
        while content < self.content_frame_count() {
            frames[AGENT_PAGE_TABLE_FRAME_COUNT + content] = self.content_frames[content];
            content += 1;
        }
        AgentFrameSet::new(frames, self.owned_frame_count())
    }

    pub const fn contains(self, frame: u64) -> bool {
        let frames = self.owned_frames();
        let mut index = 0;
        while index < frames.len() {
            if frames.frames[index] == frame {
                return true;
            }
            index += 1;
        }
        false
    }

    pub const fn is_disjoint_from(self, other: Self) -> bool {
        let left_frames = self.owned_frames();
        let right_frames = other.owned_frames();
        let mut left = 0;
        while left < left_frames.len() {
            let mut right = 0;
            while right < right_frames.len() {
                if left_frames.frames[left] == right_frames.frames[right] {
                    return false;
                }
                right += 1;
            }
            left += 1;
        }
        true
    }
}

pub const fn agent_content_frame_count(code_page_count: usize) -> Option<usize> {
    if code_page_count == 0 || code_page_count > AGENT_CODE_PAGE_CAPACITY {
        None
    } else {
        Some(code_page_count + AGENT_NON_CODE_FRAME_COUNT)
    }
}

pub const fn agent_owned_frame_count(code_page_count: usize) -> Option<usize> {
    match agent_content_frame_count(code_page_count) {
        Some(content) => Some(AGENT_PAGE_TABLE_FRAME_COUNT + content),
        None => None,
    }
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
