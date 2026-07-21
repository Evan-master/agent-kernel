use agent_kernel_x86_64::{
    address_space::{
        AgentMemoryIdentity, AGENT_CODE_PAGE_CAPACITY, AGENT_CONTENT_FRAME_CAPACITY,
        AGENT_OWNED_FRAME_CAPACITY, AGENT_PAGE_TABLE_FRAME_COUNT,
    },
    context::{PrivilegeInterruptStackFrame, SavedAgentFrame, SAVED_AGENT_FRAME_BYTES},
};

#[test]
fn agent_memory_identities_reject_aliases_and_prove_disjoint_frames() {
    let first = AgentMemoryIdentity::new(
        [0x1000, 0x2000, 0x3000, 0x4000],
        [
            0x5000, 0x6000, 0x7000, 0x8000, 0x9000, 0xa000, 0xb000, 0x2b_000, 0x2c_000, 0x2d_000,
            0x2e_000,
        ],
        AGENT_CODE_PAGE_CAPACITY,
    )
    .unwrap();
    let second = AgentMemoryIdentity::new(
        [0xc000, 0xd000, 0xe000, 0xf000],
        [
            0x10_000, 0x11_000, 0x12_000, 0x13_000, 0x14_000, 0x15_000, 0x16_000, 0x2f_000,
            0x30_000, 0x31_000, 0x32_000,
        ],
        AGENT_CODE_PAGE_CAPACITY,
    )
    .unwrap();
    let overlapping = AgentMemoryIdentity::new(
        [0x17_000, 0x18_000, 0x19_000, 0x1a_000],
        [
            0x5000, 0x1b_000, 0x1c_000, 0x1d_000, 0x1e_000, 0x1f_000, 0x20_000, 0x33_000, 0x34_000,
            0x35_000, 0x36_000,
        ],
        AGENT_CODE_PAGE_CAPACITY,
    )
    .unwrap();
    let table_overlapping = AgentMemoryIdentity::new(
        [0x21_000, 0x3000, 0x22_000, 0x23_000],
        [
            0x24_000, 0x25_000, 0x26_000, 0x27_000, 0x28_000, 0x29_000, 0x2a_000, 0x37_000,
            0x38_000, 0x39_000, 0x3a_000,
        ],
        AGENT_CODE_PAGE_CAPACITY,
    )
    .unwrap();

    assert_eq!(AGENT_CONTENT_FRAME_CAPACITY, 11);
    assert_eq!(AGENT_PAGE_TABLE_FRAME_COUNT, 4);
    assert_eq!(AGENT_OWNED_FRAME_CAPACITY, 15);
    assert!(first.is_disjoint_from(second));
    assert!(!first.is_disjoint_from(overlapping));
    assert!(!first.is_disjoint_from(table_overlapping));
    assert_eq!(first.root(), 0x1000);
    assert_eq!(first.page_table_frames()[3], 0x4000);
    assert_eq!(first.content_frames()[0], 0x5000);
    assert_eq!(first.owned_frame_count(), AGENT_OWNED_FRAME_CAPACITY);
    assert!(first.contains(0x3000));
    assert!(first.contains(0xa000));
    assert!(!first.contains(0x30_000));
    assert!(AgentMemoryIdentity::new(
        [0x1000, 0x2000, 0x2000, 0x4000],
        [
            0x5000, 0x6000, 0x7000, 0x8000, 0x9000, 0xa000, 0xb000, 0x2b_000, 0x2c_000, 0x2d_000,
            0x2e_000,
        ],
        AGENT_CODE_PAGE_CAPACITY,
    )
    .is_none());
    assert!(AgentMemoryIdentity::new(
        [0x1000, 0x2000, 0x3000, 0x4000],
        [
            0x1000, 0x6000, 0x7000, 0x8000, 0x9000, 0xa000, 0xb000, 0x2b_000, 0x2c_000, 0x2d_000,
            0x2e_000,
        ],
        AGENT_CODE_PAGE_CAPACITY,
    )
    .is_none());
    assert!(AgentMemoryIdentity::new(
        [0x1001, 0x2000, 0x3000, 0x4000],
        [
            0x5000, 0x6000, 0x7000, 0x8000, 0x9000, 0xa000, 0xb000, 0x2b_000, 0x2c_000, 0x2d_000,
            0x2e_000,
        ],
        AGENT_CODE_PAGE_CAPACITY,
    )
    .is_none());
    let zero_root = AgentMemoryIdentity::new(
        [0, 0x2000, 0x3000, 0x4000],
        [
            0x5000, 0x6000, 0x7000, 0x8000, 0x9000, 0xa000, 0xb000, 0x2b_000, 0x2c_000, 0x2d_000,
            0x2e_000,
        ],
        AGENT_CODE_PAGE_CAPACITY,
    )
    .unwrap();
    let zero_content = AgentMemoryIdentity::new(
        [0x1000, 0x2000, 0x3000, 0x4000],
        [
            0, 0x6000, 0x7000, 0x8000, 0x9000, 0xa000, 0xb000, 0x2b_000, 0x2c_000, 0x2d_000,
            0x2e_000,
        ],
        AGENT_CODE_PAGE_CAPACITY,
    )
    .unwrap();
    assert_eq!(zero_root.root(), 0);
    assert_eq!(zero_content.content_frames()[0], 0);
}

#[test]
fn agent_memory_identity_owns_only_the_active_code_prefix() {
    let one_page = identity_with_code_pages(0x40_000, 1);
    let two_pages = identity_with_code_pages(0x80_000, 2);
    let three_pages = identity_with_code_pages(0xc0_000, 3);
    let four_pages = identity_with_code_pages(0x100_000, 4);

    assert_eq!(one_page.code_page_count(), 1);
    assert_eq!(one_page.content_frames().len(), 8);
    assert_eq!(one_page.owned_frame_count(), 12);
    assert_eq!(two_pages.owned_frame_count(), 13);
    assert_eq!(three_pages.owned_frame_count(), 14);
    assert_eq!(four_pages.owned_frame_count(), 15);
    assert_eq!(one_page.signal_frame(), 0x45_000);
    assert_eq!(one_page.call_data_frame(), 0x4b_000);

    let mut noncanonical = [0; AGENT_CONTENT_FRAME_CAPACITY];
    for (index, frame) in noncanonical[..8].iter_mut().enumerate() {
        *frame = 0x204_000 + index as u64 * 0x1000;
    }
    noncanonical[8] = 0x300_000;
    assert!(AgentMemoryIdentity::new(
        [0x200_000, 0x201_000, 0x202_000, 0x203_000],
        noncanonical,
        1,
    )
    .is_none());
    assert!(AgentMemoryIdentity::new(
        [0x200_000, 0x201_000, 0x202_000, 0x203_000],
        [0; AGENT_CONTENT_FRAME_CAPACITY],
        0,
    )
    .is_none());
}

fn identity_with_code_pages(base: u64, code_page_count: usize) -> AgentMemoryIdentity {
    let mut content = [0; AGENT_CONTENT_FRAME_CAPACITY];
    for (index, frame) in content[..code_page_count + 7].iter_mut().enumerate() {
        *frame = base + (index as u64 + 4) * 0x1000;
    }
    AgentMemoryIdentity::new(
        [base, base + 0x1000, base + 0x2000, base + 0x3000],
        content,
        code_page_count,
    )
    .unwrap()
}

#[test]
fn saved_agent_frame_owns_a_complete_privilege_frame_by_value() {
    let mut hardware: PrivilegeInterruptStackFrame = unsafe { core::mem::zeroed() };
    hardware.rip = 0x4000_0000_0042;
    hardware.user_rsp = 0x4000_0000_a000;
    let saved = SavedAgentFrame::new(hardware);
    hardware.rip = 0;

    assert_eq!(hardware.rip, 0);
    assert_eq!(SAVED_AGENT_FRAME_BYTES, 160);
    assert_eq!(saved.frame().rip, 0x4000_0000_0042);
    assert_eq!(saved.frame().user_rsp, 0x4000_0000_a000);
}
