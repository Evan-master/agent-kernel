use agent_kernel_core::AgentId;
use agent_kernel_x86_64::{
    address_space::{
        AgentMemoryIdentity, AGENT_CODE_PAGE_CAPACITY, AGENT_CONTENT_FRAME_CAPACITY,
        AGENT_OWNED_FRAME_CAPACITY, AGENT_RODATA_PAGE_CAPACITY,
    },
    address_space_reclamation::AddressSpaceFramePool,
};

type TwoAddressSpacePool = AddressSpaceFramePool<{ AGENT_OWNED_FRAME_CAPACITY * 2 }>;
type ThreeAddressSpacePool = AddressSpaceFramePool<{ AGENT_OWNED_FRAME_CAPACITY * 3 }>;

#[test]
fn reclamation_pool_prepares_atomically_and_rejects_stale_tokens() {
    let first = identity(0x1000, AGENT_CODE_PAGE_CAPACITY);
    let second = identity(0x40_000, AGENT_CODE_PAGE_CAPACITY);
    let mut pool = TwoAddressSpacePool::new();

    let first_token = pool.prepare(first).unwrap();
    let stale_second = pool.prepare(second).unwrap();
    assert_eq!(pool.len(), 0);
    assert!(pool.commit(first_token));
    assert_eq!(pool.len(), AGENT_OWNED_FRAME_CAPACITY);
    assert!(!pool.commit(stale_second));
    assert!(pool.prepare(first).is_none());

    let second_token = pool.prepare(second).unwrap();
    assert!(pool.commit(second_token));
    assert_eq!(pool.len(), AGENT_OWNED_FRAME_CAPACITY * 2);
    assert!(pool
        .prepare(identity(0x80_000, AGENT_CODE_PAGE_CAPACITY))
        .is_none());
    assert!(first
        .owned_frames()
        .iter()
        .chain(second.owned_frames().iter())
        .all(|frame| pool.contains(*frame)));
}

#[test]
fn reclaimed_frames_can_be_taken_once_for_future_allocation() {
    let identity = identity(0x100_000, AGENT_CODE_PAGE_CAPACITY);
    let mut pool = AddressSpaceFramePool::<{ AGENT_OWNED_FRAME_CAPACITY }>::new();
    let token = pool.prepare(identity).unwrap();
    let replay = token;
    assert!(pool.commit(token));

    let mut taken = [0; AGENT_OWNED_FRAME_CAPACITY];
    for frame in &mut taken {
        *frame = pool.take_frame().unwrap();
    }
    taken.sort_unstable();
    let mut expected = identity.owned_frames().as_slice().to_vec();
    expected.sort_unstable();

    assert_eq!(taken.as_slice(), expected);
    assert!(pool.is_empty());
    assert_eq!(pool.take_frame(), None);
    assert!(!pool.commit(replay));
}

#[test]
fn physical_frame_zero_is_preserved_as_owned_pool_data() {
    let identity = identity(0, AGENT_CODE_PAGE_CAPACITY);
    let mut pool = AddressSpaceFramePool::<{ AGENT_OWNED_FRAME_CAPACITY }>::new();
    let token = pool.prepare(identity).unwrap();
    assert!(pool.commit(token));
    assert!(pool.contains(0));

    let mut taken = [u64::MAX; AGENT_OWNED_FRAME_CAPACITY];
    for frame in &mut taken {
        *frame = pool.take_frame().unwrap();
    }
    taken.sort_unstable();

    assert_eq!(taken.as_slice(), identity.owned_frames().as_slice());
    assert!(pool.is_empty());
}

#[test]
fn complete_address_space_allocation_is_atomic_and_generation_bound() {
    let first = identity(0x1000, AGENT_CODE_PAGE_CAPACITY);
    let second = identity(0x40_000, AGENT_CODE_PAGE_CAPACITY);
    let mut pool = TwoAddressSpacePool::new();
    let first_reclamation = pool.prepare(first).unwrap();
    assert!(pool.commit(first_reclamation));
    let second_reclamation = pool.prepare(second).unwrap();
    assert!(pool.commit(second_reclamation));

    let allocation = pool
        .prepare_allocation(
            AgentId::new(10),
            AGENT_CODE_PAGE_CAPACITY,
            AGENT_RODATA_PAGE_CAPACITY,
        )
        .unwrap();
    let stale_replay = allocation;
    assert_eq!(allocation.identity(), second);
    assert_eq!(allocation.agent(), AgentId::new(10));
    assert_eq!(allocation.address_space_generation(), 3);
    assert_eq!(pool.len(), AGENT_OWNED_FRAME_CAPACITY * 2);

    let owner = pool.commit_allocation(allocation).unwrap();
    assert_eq!(owner.identity(), second);
    assert_eq!(owner.agent(), AgentId::new(10));
    assert_eq!(owner.address_space_generation(), 3);
    assert_eq!(pool.len(), AGENT_OWNED_FRAME_CAPACITY);
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
    let identity = identity(0, AGENT_CODE_PAGE_CAPACITY);
    let mut pool = AddressSpaceFramePool::<{ AGENT_OWNED_FRAME_CAPACITY }>::new();
    let reclamation = pool.prepare(identity).unwrap();
    assert!(pool.commit(reclamation));

    let allocation = pool
        .prepare_allocation(
            AgentId::new(10),
            AGENT_CODE_PAGE_CAPACITY,
            AGENT_RODATA_PAGE_CAPACITY,
        )
        .unwrap();
    let replay = allocation;
    let owner = pool.commit_allocation(allocation).unwrap();
    assert!(pool.is_empty());
    assert!(pool.commit_allocation(replay).is_none());

    let returned = owner.into_identity();
    assert_eq!(returned, identity);
    let returned_reclamation = pool.prepare(returned).unwrap();
    assert!(pool.commit(returned_reclamation));
    assert_eq!(pool.frames(), identity.owned_frames().as_slice());
    assert!(pool.contains(0));
}

#[test]
fn concurrent_allocations_are_agent_bound_disjoint_and_cancellable() {
    let first = identity(0x1000, AGENT_CODE_PAGE_CAPACITY);
    let second = identity(0x40_000, AGENT_CODE_PAGE_CAPACITY);
    let mut pool = TwoAddressSpacePool::new();
    assert!(pool.commit(pool.prepare(first).unwrap()));
    assert!(pool.commit(pool.prepare(second).unwrap()));

    assert!(pool
        .prepare_allocation(
            AgentId::new(0),
            AGENT_CODE_PAGE_CAPACITY,
            AGENT_RODATA_PAGE_CAPACITY,
        )
        .is_none());
    let first_allocation = pool
        .prepare_allocation(
            AgentId::new(10),
            AGENT_CODE_PAGE_CAPACITY,
            AGENT_RODATA_PAGE_CAPACITY,
        )
        .unwrap();
    let stale_other_agent = pool
        .prepare_allocation(
            AgentId::new(11),
            AGENT_CODE_PAGE_CAPACITY,
            AGENT_RODATA_PAGE_CAPACITY,
        )
        .unwrap();
    let first_owner = pool.commit_allocation(first_allocation).unwrap();
    assert!(pool.commit_allocation(stale_other_agent).is_none());
    let second_owner = pool
        .commit_allocation(
            pool.prepare_allocation(
                AgentId::new(11),
                AGENT_CODE_PAGE_CAPACITY,
                AGENT_RODATA_PAGE_CAPACITY,
            )
            .unwrap(),
        )
        .unwrap();

    assert_eq!(first_owner.agent(), AgentId::new(10));
    assert_eq!(first_owner.identity(), second);
    assert_eq!(first_owner.address_space_generation(), 3);
    assert_eq!(second_owner.agent(), AgentId::new(11));
    assert_eq!(second_owner.identity(), first);
    assert_eq!(second_owner.address_space_generation(), 4);
    assert!(first_owner
        .identity()
        .is_disjoint_from(second_owner.identity()));
    assert!(pool.is_empty());

    assert!(pool.cancel_allocation(second_owner).is_ok());
    assert!(pool.cancel_allocation(first_owner).is_ok());
    assert_eq!(
        pool.frames(),
        [
            first.owned_frames().as_slice(),
            second.owned_frames().as_slice()
        ]
        .concat()
    );
}

#[test]
fn partial_batch_reclamation_reuses_workers_without_exposing_resident_frames() {
    let first = identity(0x1000, AGENT_CODE_PAGE_CAPACITY);
    let second = identity(0x40_000, AGENT_CODE_PAGE_CAPACITY);
    let third = identity(0x80_000, AGENT_CODE_PAGE_CAPACITY);
    let mut pool = ThreeAddressSpacePool::new();
    assert!(pool.commit(pool.prepare(first).unwrap()));
    assert!(pool.commit(pool.prepare(second).unwrap()));
    assert!(pool.commit(pool.prepare(third).unwrap()));

    let supervisor = pool
        .commit_allocation(
            pool.prepare_allocation(
                AgentId::new(12),
                AGENT_CODE_PAGE_CAPACITY,
                AGENT_RODATA_PAGE_CAPACITY,
            )
            .unwrap(),
        )
        .unwrap();
    let worker_a = pool
        .commit_allocation(
            pool.prepare_allocation(
                AgentId::new(10),
                AGENT_CODE_PAGE_CAPACITY,
                AGENT_RODATA_PAGE_CAPACITY,
            )
            .unwrap(),
        )
        .unwrap();
    let worker_b = pool
        .commit_allocation(
            pool.prepare_allocation(
                AgentId::new(11),
                AGENT_CODE_PAGE_CAPACITY,
                AGENT_RODATA_PAGE_CAPACITY,
            )
            .unwrap(),
        )
        .unwrap();
    assert!(pool.is_empty());

    let supervisor_identity = supervisor.identity();
    let worker_a_identity = worker_a.identity();
    let worker_b_identity = worker_b.identity();
    assert!(pool.commit(pool.prepare(worker_a.into_identity()).unwrap()));
    assert!(pool.commit(pool.prepare(worker_b.into_identity()).unwrap()));
    assert_eq!(pool.len(), AGENT_OWNED_FRAME_CAPACITY * 2);
    assert!(supervisor_identity
        .owned_frames()
        .iter()
        .all(|frame| !pool.contains(*frame)));

    let next_a = pool
        .commit_allocation(
            pool.prepare_allocation(
                AgentId::new(13),
                AGENT_CODE_PAGE_CAPACITY,
                AGENT_RODATA_PAGE_CAPACITY,
            )
            .unwrap(),
        )
        .unwrap();
    let next_b = pool
        .commit_allocation(
            pool.prepare_allocation(
                AgentId::new(14),
                AGENT_CODE_PAGE_CAPACITY,
                AGENT_RODATA_PAGE_CAPACITY,
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(next_a.identity(), worker_b_identity);
    assert_eq!(next_b.identity(), worker_a_identity);
    assert!(next_a.identity().is_disjoint_from(supervisor_identity));
    assert!(next_b.identity().is_disjoint_from(supervisor_identity));
    assert!(pool.is_empty());

    assert!(pool.cancel_allocation(next_a).is_ok());
    assert!(pool.cancel_allocation(next_b).is_ok());
    assert!(pool.commit(pool.prepare(supervisor.into_identity()).unwrap()));
    assert_eq!(pool.len(), AGENT_OWNED_FRAME_CAPACITY * 3);
}

#[test]
fn failed_cancellation_returns_the_noncopy_owner_without_pool_mutation() {
    let first = identity(0x1000, AGENT_CODE_PAGE_CAPACITY);
    let second = identity(0x40_000, AGENT_CODE_PAGE_CAPACITY);
    let mut pool = AddressSpaceFramePool::<{ AGENT_OWNED_FRAME_CAPACITY }>::new();
    assert!(pool.commit(pool.prepare(first).unwrap()));
    let owner = pool
        .commit_allocation(
            pool.prepare_allocation(
                AgentId::new(10),
                AGENT_CODE_PAGE_CAPACITY,
                AGENT_RODATA_PAGE_CAPACITY,
            )
            .unwrap(),
        )
        .unwrap();
    assert!(pool.commit(pool.prepare(second).unwrap()));

    let returned = pool.cancel_allocation(owner).unwrap_err();
    assert_eq!(returned.agent(), AgentId::new(10));
    assert_eq!(returned.identity(), first);
    assert_eq!(pool.frames(), second.owned_frames().as_slice());
}

#[test]
fn allocation_recomposes_exact_identity_size_from_reclaimed_frames() {
    let original = identity(0x400_000, AGENT_CODE_PAGE_CAPACITY);
    let mut pool = AddressSpaceFramePool::<{ AGENT_OWNED_FRAME_CAPACITY }>::new();
    assert!(pool.commit(pool.prepare(original).unwrap()));

    let small = pool
        .commit_allocation(pool.prepare_allocation(AgentId::new(20), 1, 0).unwrap())
        .unwrap();
    assert_eq!(small.identity().code_page_count(), 1);
    assert_eq!(small.identity().owned_frame_count(), 12);
    assert_eq!(pool.len(), AGENT_OWNED_FRAME_CAPACITY - 12);
    assert!(pool.cancel_allocation(small).is_ok());
    assert_eq!(pool.frames(), original.owned_frames().as_slice());

    let restored = pool
        .commit_allocation(
            pool.prepare_allocation(
                AgentId::new(21),
                AGENT_CODE_PAGE_CAPACITY,
                AGENT_RODATA_PAGE_CAPACITY,
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(restored.identity(), original);
}

fn identity(base: u64, code_page_count: usize) -> AgentMemoryIdentity {
    let mut content = [0; AGENT_CONTENT_FRAME_CAPACITY];
    for (index, frame) in content[..code_page_count + AGENT_RODATA_PAGE_CAPACITY + 7]
        .iter_mut()
        .enumerate()
    {
        *frame = base + (index as u64 + 4) * 0x1000;
    }
    AgentMemoryIdentity::new(
        [base, base + 0x1000, base + 0x2000, base + 0x3000],
        content,
        code_page_count,
        AGENT_RODATA_PAGE_CAPACITY,
    )
    .unwrap()
}
