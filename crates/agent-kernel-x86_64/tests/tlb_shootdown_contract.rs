use agent_kernel_x86_64::{
    cpu::{CpuIndex, CpuMask},
    tlb::{
        TlbAddressSpace, TlbFlushKind, TlbFlushScope, TlbShootdownCoordinator, TlbShootdownError,
        TlbShootdownProgress, TlbShootdownStatus, MAX_TLB_RANGE_PAGES,
    },
};

fn cpu(raw: u16) -> CpuIndex {
    CpuIndex::new(raw).unwrap()
}

fn mask(indices: &[u16]) -> CpuMask {
    let mut mask = CpuMask::empty();
    for index in indices {
        mask.insert(cpu(*index));
    }
    mask
}

#[test]
fn address_space_and_flush_scopes_are_canonical() {
    assert!(TlbAddressSpace::new(0x4000, 7).is_some());
    assert!(TlbAddressSpace::new(0, 7).is_none());
    assert!(TlbAddressSpace::new(0x4001, 7).is_none());
    assert!(TlbAddressSpace::new(0x4000, 0).is_none());

    let page = TlbFlushScope::page(0x0000_4000_0000_1000).unwrap();
    assert_eq!(page.kind(), TlbFlushKind::Page);
    assert_eq!(page.start(), Some(0x0000_4000_0000_1000));
    assert_eq!(page.page_count(), Some(1));
    assert!(TlbFlushScope::page(0x1001).is_none());
    assert!(TlbFlushScope::page(0x0000_8000_0000_0000).is_none());

    let range = TlbFlushScope::range(0xffff_8000_0000_0000, 8).unwrap();
    assert_eq!(range.kind(), TlbFlushKind::Range);
    assert_eq!(range.page_count(), Some(8));
    assert!(TlbFlushScope::range(0x2000, 0).is_none());
    assert!(TlbFlushScope::range(0x2000, MAX_TLB_RANGE_PAGES + 1).is_none());
    assert!(TlbFlushScope::range(0x0000_7fff_ffff_f000, 2).is_none());

    assert_eq!(
        TlbFlushScope::whole_address_space().kind(),
        TlbFlushKind::AddressSpace
    );
    assert_eq!(
        TlbFlushScope::all_contexts().kind(),
        TlbFlushKind::AllContexts
    );
}

#[test]
fn shootdown_requires_exact_target_acknowledgement_before_reuse() {
    let address_space = TlbAddressSpace::new(0x8000, 3).unwrap();
    let scope = TlbFlushScope::page(0x4000_0000_0000).unwrap();
    let online = mask(&[0, 1, 64]);
    let mut coordinator = TlbShootdownCoordinator::new();

    let request = coordinator
        .begin(cpu(0), online, online, address_space, scope)
        .unwrap();
    assert_eq!(request.generation(), 1);
    assert_eq!(request.initiator(), cpu(0));
    assert_eq!(request.targets(), mask(&[1, 64]));
    assert_eq!(
        request.status(),
        TlbShootdownStatus::AwaitingAcknowledgements
    );
    assert!(!coordinator.can_reuse_after(1));

    assert_eq!(
        coordinator.finish(1),
        Err(TlbShootdownError::RequestIncomplete {
            pending: mask(&[1, 64])
        })
    );
    assert_eq!(
        coordinator.acknowledge(cpu(0), 1),
        Err(TlbShootdownError::CpuNotTargeted(cpu(0)))
    );
    assert_eq!(
        coordinator.acknowledge(cpu(1), 2),
        Err(TlbShootdownError::StaleGeneration {
            expected: 1,
            actual: 2
        })
    );
    assert_eq!(
        coordinator.acknowledge(cpu(1), 1),
        Ok(TlbShootdownProgress::Pending(mask(&[64])))
    );
    assert_eq!(
        coordinator.acknowledge(cpu(1), 1),
        Err(TlbShootdownError::DuplicateAcknowledgement(cpu(1)))
    );
    assert_eq!(
        coordinator.acknowledge(cpu(64), 1),
        Ok(TlbShootdownProgress::Complete)
    );
    assert!(!coordinator.can_reuse_after(1));

    let completion = coordinator.finish(1).unwrap();
    assert_eq!(completion.generation(), 1);
    assert_eq!(completion.address_space(), address_space);
    assert!(coordinator.can_reuse_after(1));
    assert!(coordinator.active_request().is_none());
}

#[test]
fn shootdown_rejects_overlap_and_invalid_cpu_snapshots_atomically() {
    let address_space = TlbAddressSpace::new(0x9000, 4).unwrap();
    let scope = TlbFlushScope::whole_address_space();
    let online = mask(&[0, 1]);
    let mut coordinator = TlbShootdownCoordinator::new();

    assert_eq!(
        coordinator.begin(cpu(2), online, online, address_space, scope),
        Err(TlbShootdownError::InitiatorOffline(cpu(2)))
    );
    assert_eq!(
        coordinator.begin(cpu(0), online, mask(&[0, 1, 2]), address_space, scope),
        Err(TlbShootdownError::ResidentCpuOffline(mask(&[2])))
    );
    assert_eq!(coordinator.last_issued_generation(), 0);

    coordinator
        .begin(cpu(0), online, online, address_space, scope)
        .unwrap();
    assert_eq!(
        coordinator.begin(cpu(0), online, online, address_space, scope),
        Err(TlbShootdownError::RequestActive { generation: 1 })
    );
    assert_eq!(coordinator.last_issued_generation(), 1);
    assert_eq!(coordinator.pending_targets(), Some(mask(&[1])));
}

#[test]
fn targetless_shootdown_still_requires_explicit_finish() {
    let address_space = TlbAddressSpace::new(0xa000, 1).unwrap();
    let mut coordinator = TlbShootdownCoordinator::new();
    let request = coordinator
        .begin(
            cpu(0),
            mask(&[0, 1]),
            mask(&[0]),
            address_space,
            TlbFlushScope::all_contexts(),
        )
        .unwrap();

    assert_eq!(request.targets(), CpuMask::empty());
    assert_eq!(request.status(), TlbShootdownStatus::Complete);
    assert!(!coordinator.can_reuse_after(1));
    coordinator.finish(1).unwrap();
    assert!(coordinator.can_reuse_after(1));
}

#[test]
fn timeout_is_sticky_and_keeps_frames_quarantined() {
    let address_space = TlbAddressSpace::new(0xb000, 9).unwrap();
    let mut coordinator = TlbShootdownCoordinator::new();
    coordinator
        .begin(
            cpu(0),
            mask(&[0, 1, 2]),
            mask(&[0, 1, 2]),
            address_space,
            TlbFlushScope::whole_address_space(),
        )
        .unwrap();
    coordinator.acknowledge(cpu(1), 1).unwrap();
    assert_eq!(coordinator.mark_timed_out(1), Ok(mask(&[2])));
    assert_eq!(
        coordinator.acknowledge(cpu(2), 1),
        Err(TlbShootdownError::RequestTimedOut { generation: 1 })
    );
    assert_eq!(
        coordinator.finish(1),
        Err(TlbShootdownError::RequestTimedOut { generation: 1 })
    );
    assert!(!coordinator.can_reuse_after(1));
    assert_eq!(
        coordinator.begin(
            cpu(0),
            mask(&[0]),
            mask(&[0]),
            address_space,
            TlbFlushScope::page(0x1000).unwrap(),
        ),
        Err(TlbShootdownError::RequestActive { generation: 1 })
    );
}

#[test]
fn generation_exhaustion_never_wraps_to_zero() {
    let mut coordinator = TlbShootdownCoordinator::from_completed_generation(u64::MAX);
    let address_space = TlbAddressSpace::new(0xc000, 1).unwrap();
    assert_eq!(
        coordinator.begin(
            cpu(0),
            mask(&[0]),
            mask(&[0]),
            address_space,
            TlbFlushScope::whole_address_space(),
        ),
        Err(TlbShootdownError::GenerationExhausted)
    );
    assert_eq!(coordinator.last_issued_generation(), u64::MAX);
    assert!(coordinator.active_request().is_none());
}
