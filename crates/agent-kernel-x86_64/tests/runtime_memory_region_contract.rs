use agent_kernel_core::{MemoryCellId, ResourceId};
use agent_kernel_x86_64::{
    runtime_frame_pool::MAX_RUNTIME_REGION_PAGES,
    runtime_region::{
        RuntimeRegionLedger, RuntimeRegionObservationLog, RUNTIME_MEMORY_ACCESS_READ_WRITE,
        RUNTIME_REGION_CAPACITY, RUNTIME_REGION_OBSERVATION_CAPACITY, RUNTIME_REGION_SLOT_COUNT,
    },
    user_memory::{UserMemoryLayout, PAGE_BYTES},
};

#[test]
fn runtime_region_layout_is_fixed_bounded_and_page_aligned() {
    let layout = UserMemoryLayout::fixed();

    assert_eq!(RUNTIME_REGION_SLOT_COUNT, 8);
    assert_eq!(RUNTIME_REGION_CAPACITY, 4);
    assert_eq!(MAX_RUNTIME_REGION_PAGES, 4);
    assert_eq!(RUNTIME_MEMORY_ACCESS_READ_WRITE, 3);
    assert_eq!(
        layout.runtime_region_start(),
        layout.runtime_page_start() + PAGE_BYTES
    );
    assert_eq!(
        layout.runtime_region_end(),
        layout.runtime_region_start() + PAGE_BYTES * RUNTIME_REGION_SLOT_COUNT as u64
    );
    for slot in 0..RUNTIME_REGION_SLOT_COUNT {
        let start = layout.runtime_region_page_start(slot).unwrap();
        assert_eq!(
            start,
            layout.runtime_region_start() + PAGE_BYTES * slot as u64
        );
        assert!(layout.contains_runtime_region(start));
        assert!(layout.contains_runtime_region(start + PAGE_BYTES - 1));
    }
    assert!(layout
        .runtime_region_page_start(RUNTIME_REGION_SLOT_COUNT)
        .is_none());
    assert!(!layout.contains_runtime_region(layout.runtime_region_start() - 1));
    assert!(!layout.contains_runtime_region(layout.runtime_region_end()));
}

#[test]
fn region_ledger_uses_contiguous_first_fit_and_reuses_holes() {
    let mut ledger = RuntimeRegionLedger::new();
    let first = ledger.reserve(ResourceId::new(1), 3).unwrap();
    assert_reservation(first, 1, 0, 3, 1);
    assert!(ledger.commit_mapping(first, MemoryCellId::new(1)));

    let second = ledger.reserve(ResourceId::new(2), 2).unwrap();
    assert_reservation(second, 2, 3, 2, 2);
    assert!(ledger.commit_mapping(second, MemoryCellId::new(2)));

    let third = ledger.reserve(ResourceId::new(3), 3).unwrap();
    assert_reservation(third, 3, 5, 3, 3);
    assert!(ledger.commit_mapping(third, MemoryCellId::new(3)));
    assert_eq!(ledger.active_region_count(), 3);
    assert!(ledger.reserve(ResourceId::new(4), 1).is_none());

    let released = ledger
        .prepare_release(ResourceId::new(2), MemoryCellId::new(2))
        .unwrap();
    assert_eq!(released.start_slot(), 3);
    assert_eq!(released.page_count(), 2);
    assert!(ledger.commit_release(released));

    let reused = ledger.reserve(ResourceId::new(4), 2).unwrap();
    assert_reservation(reused, 4, 3, 2, 4);
    assert!(ledger.commit_mapping(reused, MemoryCellId::new(4)));
    assert!(ledger.reserve(ResourceId::new(5), 4).is_none());
}

#[test]
fn region_ledger_rejects_stale_tokens_and_preserves_generation_on_cancel() {
    let mut ledger = RuntimeRegionLedger::new();
    assert!(ledger.reserve(ResourceId::new(0), 1).is_none());
    assert!(ledger.reserve(ResourceId::new(1), 0).is_none());
    assert!(ledger
        .reserve(ResourceId::new(1), MAX_RUNTIME_REGION_PAGES + 1)
        .is_none());

    let stale = ledger.reserve(ResourceId::new(1), 2).unwrap();
    assert!(ledger.cancel(stale));
    assert_eq!(ledger.generation(), 0);

    let current = ledger.reserve(ResourceId::new(1), 2).unwrap();
    assert_eq!(current.generation(), 1);
    assert!(!ledger.cancel(stale));
    assert!(!ledger.commit_mapping(stale, MemoryCellId::new(1)));
    assert!(!ledger.commit_mapping(current, MemoryCellId::new(0)));
    assert!(ledger.commit_mapping(current, MemoryCellId::new(1)));

    let binding = ledger
        .binding(ResourceId::new(1), MemoryCellId::new(1))
        .unwrap();
    assert_eq!(binding.start_slot(), 0);
    assert_eq!(binding.page_count(), 2);
    assert_eq!(binding.generation(), 1);
    assert!(ledger
        .prepare_release(ResourceId::new(2), MemoryCellId::new(1))
        .is_none());
    let release = ledger
        .prepare_release(ResourceId::new(1), MemoryCellId::new(1))
        .unwrap();
    assert!(ledger.commit_release(release));
    assert!(!ledger.commit_release(release));
    assert!(ledger.is_clear());
    assert_eq!(ledger.generation(), 1);
}

#[test]
fn region_observation_log_is_ordered_bounded_and_tracks_hole_reuse() {
    let mut ledger = RuntimeRegionLedger::new();
    let mut observations = RuntimeRegionObservationLog::new();
    assert_eq!(RUNTIME_REGION_OBSERVATION_CAPACITY, 3);
    assert!(observations.is_empty());

    let first = ledger.reserve(ResourceId::new(1), 3).unwrap();
    assert!(ledger.commit_mapping(first, MemoryCellId::new(1)));
    let first_binding = ledger
        .binding(ResourceId::new(1), MemoryCellId::new(1))
        .unwrap();
    assert!(observations.record(first_binding, 0xa1, 0xa3));
    assert!(!observations.record(first_binding, 0xff, 0xff));
    assert_eq!(observations.len(), 1);

    let second = ledger.reserve(ResourceId::new(2), 2).unwrap();
    assert!(ledger.commit_mapping(second, MemoryCellId::new(2)));
    let second_binding = ledger
        .binding(ResourceId::new(2), MemoryCellId::new(2))
        .unwrap();
    assert!(observations.record(second_binding, 0xb1, 0xb2));

    let first_release = ledger
        .prepare_release(ResourceId::new(1), MemoryCellId::new(1))
        .unwrap();
    assert!(ledger.commit_release(first_release));
    let third = ledger.reserve(ResourceId::new(3), 3).unwrap();
    assert_eq!(third.start_slot(), 0);
    assert_eq!(third.generation(), 3);
    assert!(ledger.commit_mapping(third, MemoryCellId::new(3)));
    let third_binding = ledger
        .binding(ResourceId::new(3), MemoryCellId::new(3))
        .unwrap();
    assert!(observations.record(third_binding, 0xc1, 0xc3));

    let expected = [
        (1, 0, 3, 1, 0xa1, 0xa3),
        (2, 3, 2, 2, 0xb1, 0xb2),
        (3, 0, 3, 3, 0xc1, 0xc3),
    ];
    for (index, (cell, start, pages, generation, first, last)) in expected.into_iter().enumerate() {
        let observation = observations.get(index).unwrap();
        assert_eq!(observation.cell(), MemoryCellId::new(cell));
        assert_eq!(observation.start_slot(), start);
        assert_eq!(observation.page_count(), pages);
        assert_eq!(observation.generation(), generation);
        assert_eq!(observation.first(), first);
        assert_eq!(observation.last(), last);
    }

    let fourth = ledger.reserve(ResourceId::new(4), 1).unwrap();
    assert!(ledger.commit_mapping(fourth, MemoryCellId::new(4)));
    let fourth_binding = ledger
        .binding(ResourceId::new(4), MemoryCellId::new(4))
        .unwrap();
    let snapshot = observations;
    assert!(!observations.record(fourth_binding, 0xd1, 0xd1));
    assert_eq!(observations, snapshot);
    assert_eq!(observations.len(), RUNTIME_REGION_OBSERVATION_CAPACITY);
    assert!(observations
        .get(RUNTIME_REGION_OBSERVATION_CAPACITY)
        .is_none());
}

fn assert_reservation(
    reservation: agent_kernel_x86_64::runtime_region::RuntimeRegionReservation,
    resource: u64,
    start_slot: usize,
    page_count: usize,
    generation: u64,
) {
    assert_eq!(reservation.resource(), ResourceId::new(resource));
    assert_eq!(reservation.start_slot(), start_slot);
    assert_eq!(reservation.page_count(), page_count);
    assert_eq!(reservation.generation(), generation);
}
