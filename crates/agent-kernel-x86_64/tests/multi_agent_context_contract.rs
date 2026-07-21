use agent_kernel_x86_64::{
    address_space::{
        AgentMemoryIdentity, AGENT_CODE_PAGE_CAPACITY, AGENT_CONTENT_FRAME_CAPACITY,
        AGENT_OWNED_FRAME_CAPACITY, AGENT_PAGE_TABLE_FRAME_COUNT,
    },
    context::{PrivilegeInterruptStackFrame, SavedAgentFrame, SAVED_AGENT_FRAME_BYTES},
};

#[test]
fn agent_memory_identities_reject_aliases_and_prove_disjoint_frames() {
    let first = identity_with_code_pages(0x1000, 4);
    let second = identity_with_code_pages(0x100_000, 4);
    let mut overlapping_content = content_frames(0x204_000, 4);
    overlapping_content[0] = first.code_frames()[0];
    let overlapping = AgentMemoryIdentity::new(
        [0x200_000, 0x201_000, 0x202_000, 0x203_000],
        overlapping_content,
        4,
        0,
    )
    .unwrap();
    let table_overlapping = AgentMemoryIdentity::new(
        [
            0x300_000,
            first.page_table_frames()[1],
            0x302_000,
            0x303_000,
        ],
        content_frames(0x304_000, 4),
        4,
        0,
    )
    .unwrap();

    assert_eq!(AGENT_CONTENT_FRAME_CAPACITY, 39);
    assert_eq!(AGENT_PAGE_TABLE_FRAME_COUNT, 4);
    assert_eq!(AGENT_OWNED_FRAME_CAPACITY, 43);
    assert!(first.is_disjoint_from(second));
    assert!(!first.is_disjoint_from(overlapping));
    assert!(!first.is_disjoint_from(table_overlapping));
    assert_eq!(first.root(), 0x1000);
    assert_eq!(first.page_table_frames()[3], 0x4000);
    assert_eq!(first.content_frames()[0], 0x5000);
    assert_eq!(first.owned_frame_count(), 15);
    assert!(first.contains(0x3000));
    assert!(first.contains(0xa000));
    assert!(!first.contains(0x100_000));
    assert!(AgentMemoryIdentity::new(
        [0x1000, 0x2000, 0x2000, 0x4000],
        content_frames(0x5000, 4),
        4,
        0,
    )
    .is_none());
    let mut table_alias = content_frames(0x5000, 4);
    table_alias[0] = 0x1000;
    assert!(
        AgentMemoryIdentity::new([0x1000, 0x2000, 0x3000, 0x4000], table_alias, 4, 0,).is_none()
    );
    assert!(AgentMemoryIdentity::new(
        [0x1001, 0x2000, 0x3000, 0x4000],
        content_frames(0x5000, 4),
        4,
        0,
    )
    .is_none());
    let zero_root = AgentMemoryIdentity::new(
        [0, 0x401_000, 0x402_000, 0x403_000],
        content_frames(0x404_000, 4),
        4,
        0,
    )
    .unwrap();
    let mut zero_content_frames = content_frames(0x504_000, 4);
    zero_content_frames[0] = 0;
    let zero_content = AgentMemoryIdentity::new(
        [0x500_000, 0x501_000, 0x502_000, 0x503_000],
        zero_content_frames,
        4,
        0,
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
    let sixteen_pages = identity_with_code_pages(0x200_000, 16);

    assert_eq!(AGENT_CODE_PAGE_CAPACITY, 16);
    assert_eq!(AGENT_CONTENT_FRAME_CAPACITY, 39);
    assert_eq!(AGENT_OWNED_FRAME_CAPACITY, 43);
    assert_eq!(one_page.code_page_count(), 1);
    assert_eq!(one_page.rodata_page_count(), 0);
    assert_eq!(one_page.content_frames().len(), 8);
    assert_eq!(one_page.owned_frame_count(), 12);
    assert_eq!(two_pages.owned_frame_count(), 13);
    assert_eq!(three_pages.owned_frame_count(), 14);
    assert_eq!(four_pages.owned_frame_count(), 15);
    assert_eq!(sixteen_pages.code_frames().len(), 16);
    assert_eq!(sixteen_pages.content_frames().len(), 23);
    assert_eq!(sixteen_pages.owned_frame_count(), 27);
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
        0,
    )
    .is_none());
    assert!(AgentMemoryIdentity::new(
        [0x200_000, 0x201_000, 0x202_000, 0x203_000],
        [0; AGENT_CONTENT_FRAME_CAPACITY],
        0,
        0,
    )
    .is_none());
    assert!(AgentMemoryIdentity::new(
        [0x200_000, 0x201_000, 0x202_000, 0x203_000],
        [0; AGENT_CONTENT_FRAME_CAPACITY],
        AGENT_CODE_PAGE_CAPACITY + 1,
        0,
    )
    .is_none());
}

fn identity_with_code_pages(base: u64, code_page_count: usize) -> AgentMemoryIdentity {
    AgentMemoryIdentity::new(
        [base, base + 0x1000, base + 0x2000, base + 0x3000],
        content_frames(base + 0x4000, code_page_count),
        code_page_count,
        0,
    )
    .unwrap()
}

fn content_frames(start: u64, code_page_count: usize) -> [u64; AGENT_CONTENT_FRAME_CAPACITY] {
    let mut content = [0; AGENT_CONTENT_FRAME_CAPACITY];
    for (index, frame) in content[..code_page_count + 7].iter_mut().enumerate() {
        *frame = start + index as u64 * 0x1000;
    }
    content
}

#[test]
fn saved_agent_frame_owns_a_complete_privilege_frame_by_value() {
    let mut hardware: PrivilegeInterruptStackFrame = unsafe { core::mem::zeroed() };
    hardware.rip = 0x4000_0000_0042;
    hardware.user_rsp = 0x4000_0002_6000;
    let saved = SavedAgentFrame::new(hardware);
    hardware.rip = 0;

    assert_eq!(hardware.rip, 0);
    assert_eq!(SAVED_AGENT_FRAME_BYTES, 160);
    assert_eq!(saved.frame().rip, 0x4000_0000_0042);
    assert_eq!(saved.frame().user_rsp, 0x4000_0002_6000);
}
