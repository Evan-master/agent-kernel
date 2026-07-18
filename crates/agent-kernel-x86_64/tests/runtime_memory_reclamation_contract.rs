use agent_kernel_core::{CapabilityId, MemoryCellId, ResourceId};
use agent_kernel_x86_64::{
    runtime_page::RuntimePageLedger,
    runtime_reclamation::{
        RuntimeMemoryKind, RuntimeReclamationLog, RuntimeReclamationPlan,
        RUNTIME_RECLAMATION_CAPACITY,
    },
    runtime_region::RuntimeRegionLedger,
};

#[test]
fn reclamation_plan_orders_page_before_live_regions_and_preserves_authority() {
    let mut page = RuntimePageLedger::new();
    let page_reservation = page
        .reserve(ResourceId::new(1), CapabilityId::new(7))
        .unwrap();
    assert!(page.commit_mapping(page_reservation, MemoryCellId::new(1)));

    let mut regions = RuntimeRegionLedger::new();
    let first = regions
        .reserve(ResourceId::new(2), CapabilityId::new(8), 2)
        .unwrap();
    assert!(regions.commit_mapping(first, MemoryCellId::new(2)));
    let second = regions
        .reserve(ResourceId::new(3), CapabilityId::new(9), 1)
        .unwrap();
    assert!(regions.commit_mapping(second, MemoryCellId::new(3)));

    let plan = RuntimeReclamationPlan::new(page.binding(), regions.bindings()).unwrap();
    assert_eq!(RUNTIME_RECLAMATION_CAPACITY, 5);
    assert_eq!(plan.len(), 3);

    let expected = [
        (RuntimeMemoryKind::Page, 1, 7, 1, 1),
        (RuntimeMemoryKind::Region, 2, 8, 2, 2),
        (RuntimeMemoryKind::Region, 3, 9, 3, 1),
    ];
    for (index, (kind, resource, capability, cell, pages)) in expected.into_iter().enumerate() {
        let candidate = plan.get(index).unwrap();
        assert_eq!(candidate.kind(), kind);
        assert_eq!(candidate.resource(), ResourceId::new(resource));
        assert_eq!(candidate.capability(), CapabilityId::new(capability));
        assert_eq!(candidate.cell(), MemoryCellId::new(cell));
        assert_eq!(candidate.page_count(), pages);
    }
    assert!(plan.get(plan.len()).is_none());
    assert!(plan.get(usize::MAX).is_none());

    let mut log = RuntimeReclamationLog::new();
    assert!(log.record(plan.get(0).unwrap(), 0xa1, 0xa1));
    assert!(log.record(plan.get(1).unwrap(), 0xb1, 0xb2));
    assert!(log.record(plan.get(2).unwrap(), 0xc1, 0xc1));
    assert!(log.matches_plan(plan));
    assert!(!log.record(plan.get(2).unwrap(), 0xff, 0xff));
    assert!(log.get(usize::MAX).is_none());

    let region_evidence = log.get(1).unwrap();
    assert_eq!(region_evidence.kind(), RuntimeMemoryKind::Region);
    assert_eq!(region_evidence.capability(), CapabilityId::new(8));
    assert_eq!(region_evidence.generation(), 1);
    assert_eq!(region_evidence.first(), 0xb1);
    assert_eq!(region_evidence.last(), 0xb2);
}

#[test]
fn reclamation_plan_rejects_duplicate_semantic_bindings() {
    let mut page = RuntimePageLedger::new();
    let page_reservation = page
        .reserve(ResourceId::new(1), CapabilityId::new(7))
        .unwrap();
    assert!(page.commit_mapping(page_reservation, MemoryCellId::new(1)));

    let mut regions = RuntimeRegionLedger::new();
    let duplicate = regions
        .reserve(ResourceId::new(1), CapabilityId::new(7), 1)
        .unwrap();
    assert!(regions.commit_mapping(duplicate, MemoryCellId::new(1)));

    assert!(RuntimeReclamationPlan::new(page.binding(), regions.bindings()).is_none());
}
