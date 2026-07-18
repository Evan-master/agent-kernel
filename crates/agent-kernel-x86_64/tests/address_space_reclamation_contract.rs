use agent_kernel_x86_64::{
    address_space::{AgentMemoryIdentity, AGENT_OWNED_FRAME_COUNT},
    address_space_reclamation::AddressSpaceFramePool,
};

type TwoAddressSpacePool = AddressSpaceFramePool<{ AGENT_OWNED_FRAME_COUNT * 2 }>;

#[test]
fn reclamation_pool_prepares_atomically_and_rejects_stale_tokens() {
    let first = identity(0x1000);
    let second = identity(0x20_000);
    let mut pool = TwoAddressSpacePool::new();

    let first_token = pool.prepare(first).unwrap();
    let stale_second = pool.prepare(second).unwrap();
    assert_eq!(pool.len(), 0);
    assert!(pool.commit(first_token));
    assert_eq!(pool.len(), AGENT_OWNED_FRAME_COUNT);
    assert!(!pool.commit(stale_second));
    assert!(pool.prepare(first).is_none());

    let second_token = pool.prepare(second).unwrap();
    assert!(pool.commit(second_token));
    assert_eq!(pool.len(), AGENT_OWNED_FRAME_COUNT * 2);
    assert!(pool.prepare(identity(0x40_000)).is_none());
    assert!(first
        .owned_frames()
        .iter()
        .chain(second.owned_frames().iter())
        .all(|frame| pool.contains(*frame)));
}

#[test]
fn reclaimed_frames_can_be_taken_once_for_future_allocation() {
    let identity = identity(0x50_000);
    let mut pool = AddressSpaceFramePool::<{ AGENT_OWNED_FRAME_COUNT }>::new();
    let token = pool.prepare(identity).unwrap();
    let replay = token;
    assert!(pool.commit(token));

    let mut taken = [0; AGENT_OWNED_FRAME_COUNT];
    for frame in &mut taken {
        *frame = pool.take_frame().unwrap();
    }
    taken.sort_unstable();
    let mut expected = identity.owned_frames();
    expected.sort_unstable();

    assert_eq!(taken, expected);
    assert!(pool.is_empty());
    assert_eq!(pool.take_frame(), None);
    assert!(!pool.commit(replay));
}

#[test]
fn physical_frame_zero_is_preserved_as_owned_pool_data() {
    let identity = identity(0);
    let mut pool = AddressSpaceFramePool::<{ AGENT_OWNED_FRAME_COUNT }>::new();
    let token = pool.prepare(identity).unwrap();
    assert!(pool.commit(token));
    assert!(pool.contains(0));

    let mut taken = [u64::MAX; AGENT_OWNED_FRAME_COUNT];
    for frame in &mut taken {
        *frame = pool.take_frame().unwrap();
    }
    taken.sort_unstable();

    assert_eq!(taken, identity.owned_frames());
    assert!(pool.is_empty());
}

#[test]
fn complete_address_space_allocation_is_atomic_and_generation_bound() {
    let first = identity(0x1000);
    let second = identity(0x20_000);
    let mut pool = TwoAddressSpacePool::new();
    let first_reclamation = pool.prepare(first).unwrap();
    assert!(pool.commit(first_reclamation));
    let second_reclamation = pool.prepare(second).unwrap();
    assert!(pool.commit(second_reclamation));

    let allocation = pool.prepare_allocation().unwrap();
    let stale_replay = allocation;
    assert_eq!(allocation.identity(), second);
    assert_eq!(pool.len(), AGENT_OWNED_FRAME_COUNT * 2);

    let owner = pool.commit_allocation(allocation).unwrap();
    assert_eq!(owner.identity(), second);
    assert_eq!(pool.len(), AGENT_OWNED_FRAME_COUNT);
    assert!(first
        .owned_frames()
        .iter()
        .all(|frame| pool.contains(*frame)));
    assert!(second
        .owned_frames()
        .iter()
        .all(|frame| !pool.contains(*frame)));
    assert!(pool.commit_allocation(stale_replay).is_none());
    assert_eq!(owner.into_identity(), second);
}

#[test]
fn allocated_address_space_with_frame_zero_can_be_restored_exactly() {
    let identity = identity(0);
    let mut pool = AddressSpaceFramePool::<{ AGENT_OWNED_FRAME_COUNT }>::new();
    let reclamation = pool.prepare(identity).unwrap();
    assert!(pool.commit(reclamation));

    let allocation = pool.prepare_allocation().unwrap();
    let replay = allocation;
    let owner = pool.commit_allocation(allocation).unwrap();
    assert!(pool.is_empty());
    assert!(pool.commit_allocation(replay).is_none());

    let returned = owner.into_identity();
    assert_eq!(returned, identity);
    let returned_reclamation = pool.prepare(returned).unwrap();
    assert!(pool.commit(returned_reclamation));
    assert_eq!(pool.frames(), identity.owned_frames());
    assert!(pool.contains(0));
}

fn identity(base: u64) -> AgentMemoryIdentity {
    AgentMemoryIdentity::new(
        [base, base + 0x1000, base + 0x2000, base + 0x3000],
        [
            base + 0x4000,
            base + 0x5000,
            base + 0x6000,
            base + 0x7000,
            base + 0x8000,
            base + 0x9000,
            base + 0xa000,
        ],
    )
    .unwrap()
}
