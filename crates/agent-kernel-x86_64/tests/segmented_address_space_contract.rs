use agent_kernel_core::AgentId;
use agent_kernel_x86_64::{
    address_space::{
        AgentMemoryIdentity, AGENT_CODE_PAGE_CAPACITY, AGENT_CONTENT_FRAME_CAPACITY,
        AGENT_OWNED_FRAME_CAPACITY, AGENT_RODATA_PAGE_CAPACITY,
    },
    address_space_reclamation::AddressSpaceFramePool,
};

#[test]
fn segmented_identity_tracks_active_code_and_rodata_prefixes() {
    assert_eq!(AGENT_CODE_PAGE_CAPACITY, 16);
    assert_eq!(AGENT_RODATA_PAGE_CAPACITY, 16);
    assert_eq!(AGENT_CONTENT_FRAME_CAPACITY, 39);
    assert_eq!(AGENT_OWNED_FRAME_CAPACITY, 43);

    let segmented = identity(0x1000, 5, 1);
    assert_eq!(segmented.code_page_count(), 5);
    assert_eq!(segmented.rodata_page_count(), 1);
    assert_eq!(
        segmented.code_frames().as_slice(),
        &[0x5000, 0x6000, 0x7000, 0x8000, 0x9000]
    );
    assert_eq!(segmented.rodata_frames().as_slice(), &[0xa000]);
    assert_eq!(segmented.signal_frame(), 0xb000);
    assert_eq!(segmented.call_data_frame(), 0x11_000);
    assert_eq!(segmented.content_frame_count(), 13);
    assert_eq!(segmented.owned_frame_count(), 17);

    let maximal = identity(0x100_000, 16, 16);
    assert_eq!(maximal.content_frame_count(), 39);
    assert_eq!(maximal.owned_frame_count(), 43);
    assert!(segmented.is_disjoint_from(maximal));
}

#[test]
fn segmented_identity_rejects_invalid_counts_and_noncanonical_tail() {
    let tables = [0x1000, 0x2000, 0x3000, 0x4000];
    assert!(AgentMemoryIdentity::new(tables, [0; AGENT_CONTENT_FRAME_CAPACITY], 0, 1).is_none());
    assert!(AgentMemoryIdentity::new(tables, [0; AGENT_CONTENT_FRAME_CAPACITY], 1, 17).is_none());

    let mut content = content(0x5000, 1, 1);
    content[9] = 0x40_000;
    assert!(AgentMemoryIdentity::new(tables, content, 1, 1).is_none());
}

#[test]
fn reclaimed_pool_recomposes_exact_segmented_identity_size() {
    let original = identity(0x1000, 16, 16);
    let mut pool = AddressSpaceFramePool::<AGENT_OWNED_FRAME_CAPACITY>::new();
    assert!(pool.commit(pool.prepare(original).unwrap()));

    let compact = pool
        .commit_allocation(pool.prepare_allocation(AgentId::new(7), 5, 1).unwrap())
        .unwrap();
    assert_eq!(compact.identity().code_page_count(), 5);
    assert_eq!(compact.identity().rodata_page_count(), 1);
    assert_eq!(compact.identity().owned_frame_count(), 17);
    assert_eq!(pool.len(), 26);
    assert!(pool.cancel_allocation(compact).is_ok());
    assert_eq!(pool.frames(), original.owned_frames().as_slice());
}

fn identity(base: u64, code_pages: usize, rodata_pages: usize) -> AgentMemoryIdentity {
    AgentMemoryIdentity::new(
        [base, base + 0x1000, base + 0x2000, base + 0x3000],
        content(base + 0x4000, code_pages, rodata_pages),
        code_pages,
        rodata_pages,
    )
    .unwrap()
}

fn content(
    start: u64,
    code_pages: usize,
    rodata_pages: usize,
) -> [u64; AGENT_CONTENT_FRAME_CAPACITY] {
    let mut frames = [0; AGENT_CONTENT_FRAME_CAPACITY];
    for (index, frame) in frames[..code_pages + rodata_pages + 7]
        .iter_mut()
        .enumerate()
    {
        *frame = start + index as u64 * 0x1000;
    }
    frames
}
