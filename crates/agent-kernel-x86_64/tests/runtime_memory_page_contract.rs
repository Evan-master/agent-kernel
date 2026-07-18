use agent_kernel_core::{MemoryCellId, ResourceId};
use agent_kernel_x86_64::{
    runtime_page::{RuntimePageLedger, RUNTIME_PAGE_ACCESS_READ_WRITE},
    user_memory::{UserMemoryLayout, PAGE_BYTES},
};

#[test]
fn runtime_page_has_a_private_fixed_virtual_slot() {
    let layout = UserMemoryLayout::fixed();

    assert_eq!(
        layout.runtime_page_start(),
        layout.lazy_data_start() + PAGE_BYTES
    );
    assert!(layout.contains_runtime_page(layout.runtime_page_start()));
    assert!(layout.contains_runtime_page(layout.runtime_page_start() + PAGE_BYTES - 1));
    assert!(!layout.contains_runtime_page(layout.runtime_page_start() - 1));
    assert!(!layout.contains_runtime_page(layout.runtime_page_start() + PAGE_BYTES));
    assert_eq!(RUNTIME_PAGE_ACCESS_READ_WRITE, 3);
}

#[test]
fn runtime_page_ledger_commits_releases_and_reuses_one_slot() {
    let resource = ResourceId::new(3);
    let cell = MemoryCellId::new(1);
    let mut ledger = RuntimePageLedger::new();

    let reservation = ledger.reserve(resource).unwrap();
    assert_eq!(reservation.resource(), resource);
    assert_eq!(reservation.generation(), 1);
    assert!(!ledger.is_available());
    assert!(ledger.commit_mapping(reservation, cell));
    assert!(ledger.matches(resource, cell, 1));

    let release = ledger.prepare_release(resource, cell).unwrap();
    assert_eq!(release.generation(), 1);
    assert!(ledger.commit_release(release));
    assert!(ledger.is_available());
    assert_eq!(ledger.generation(), 1);

    let second = ledger.reserve(ResourceId::new(4)).unwrap();
    assert_eq!(second.generation(), 2);
}

#[test]
fn runtime_page_ledger_rejects_stale_tokens_and_can_cancel() {
    let mut ledger = RuntimePageLedger::new();
    assert!(ledger.reserve(ResourceId::new(0)).is_none());

    let reservation = ledger.reserve(ResourceId::new(3)).unwrap();
    assert!(!ledger.commit_mapping(reservation, MemoryCellId::new(0)));
    assert!(ledger.cancel(reservation));
    assert!(ledger.is_available());
    assert_eq!(ledger.generation(), 0);

    let next = ledger.reserve(ResourceId::new(3)).unwrap();
    assert!(!ledger.cancel(reservation));
    assert!(!ledger.commit_mapping(reservation, MemoryCellId::new(1)));
    assert!(ledger.commit_mapping(next, MemoryCellId::new(1)));
    assert!(ledger
        .prepare_release(ResourceId::new(4), MemoryCellId::new(1))
        .is_none());
}
