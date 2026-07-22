use agent_kernel_x86_64::cpu::{
    ApicId, CpuIndex, CpuLifecycleState, CpuMask, CpuRegistry, CpuRegistryError,
    CpuTopologyBuilder, FirmwareCpuFlags, FirmwareProcessor, ProcessorSource, TopologyError,
    TopologyInsert, MAX_CPU_COUNT,
};

fn processor(
    uid: u32,
    apic_id: u32,
    source: ProcessorSource,
    flags: FirmwareCpuFlags,
) -> FirmwareProcessor {
    FirmwareProcessor::new(uid, ApicId::new(apic_id), source, flags)
}

#[test]
fn cpu_mask_covers_every_supported_boundary() {
    let zero = CpuIndex::new(0).unwrap();
    let sixty_three = CpuIndex::new(63).unwrap();
    let sixty_four = CpuIndex::new(64).unwrap();
    let last = CpuIndex::new(255).unwrap();

    let mut mask = CpuMask::empty();
    for index in [zero, sixty_three, sixty_four, last] {
        assert!(mask.insert(index));
        assert!(!mask.insert(index));
        assert!(mask.contains(index));
    }

    assert_eq!(mask.count(), 4);
    assert_eq!(mask.first(), Some(zero));
    assert_eq!(CpuIndex::new(MAX_CPU_COUNT as u16), None);

    assert!(mask.remove(sixty_three));
    assert!(!mask.remove(sixty_three));
    assert_eq!(mask.count(), 3);
    assert_eq!(mask.intersection(CpuMask::singleton(sixty_four)).count(), 1);
    assert_eq!(mask.difference(CpuMask::singleton(sixty_four)).count(), 2);
}

#[test]
fn topology_freeze_assigns_bsp_zero_and_preserves_ap_order() {
    let enabled = FirmwareCpuFlags::new(true, false);
    let online_capable = FirmwareCpuFlags::new(false, true);
    let disabled = FirmwareCpuFlags::new(false, false);
    let mut builder = CpuTopologyBuilder::<4>::new();

    assert_eq!(
        builder.insert(processor(7, 9, ProcessorSource::LocalApic, enabled)),
        Ok(TopologyInsert::Accepted)
    );
    assert_eq!(
        builder.insert(processor(
            8,
            0x1ff,
            ProcessorSource::LocalX2Apic,
            online_capable,
        )),
        Ok(TopologyInsert::Accepted)
    );
    assert_eq!(
        builder.insert(processor(99, 44, ProcessorSource::LocalApic, disabled)),
        Ok(TopologyInsert::IgnoredDisabled)
    );
    assert_eq!(
        builder.insert(processor(5, 2, ProcessorSource::LocalApic, enabled)),
        Ok(TopologyInsert::Accepted)
    );

    let topology = builder.freeze(ApicId::new(2)).unwrap();
    assert_eq!(topology.len(), 3);
    assert_eq!(topology.bsp().index(), CpuIndex::BSP);
    assert_eq!(topology.bsp().processor().apic_id(), ApicId::new(2));
    assert_eq!(
        topology
            .get(CpuIndex::new(1).unwrap())
            .unwrap()
            .processor()
            .uid(),
        7
    );
    assert_eq!(
        topology
            .get(CpuIndex::new(2).unwrap())
            .unwrap()
            .processor()
            .uid(),
        8
    );
    assert_eq!(
        topology.index_for_apic_id(ApicId::new(0x1ff)),
        Some(CpuIndex::new(2).unwrap())
    );
    assert_eq!(topology.present_mask().count(), 3);
}

#[test]
fn topology_rejects_duplicate_identities_and_capacity_atomically() {
    let enabled = FirmwareCpuFlags::new(true, false);
    let mut builder = CpuTopologyBuilder::<2>::new();
    assert_eq!(
        builder.insert(processor(1, 4, ProcessorSource::LocalApic, enabled)),
        Ok(TopologyInsert::Accepted)
    );

    assert_eq!(
        builder.insert(processor(2, 4, ProcessorSource::LocalApic, enabled)),
        Err(TopologyError::DuplicateApicId(ApicId::new(4)))
    );
    assert_eq!(builder.len(), 1);
    assert_eq!(
        builder.insert(processor(1, 5, ProcessorSource::LocalX2Apic, enabled)),
        Err(TopologyError::DuplicateProcessorUid(1))
    );
    assert_eq!(builder.len(), 1);
    assert_eq!(
        builder.insert(processor(2, 5, ProcessorSource::LocalApic, enabled)),
        Ok(TopologyInsert::Accepted)
    );
    assert_eq!(
        builder.insert(processor(3, 6, ProcessorSource::LocalApic, enabled)),
        Err(TopologyError::CapacityExceeded)
    );
    assert_eq!(builder.len(), 2);
}

#[test]
fn topology_requires_an_enabled_boot_processor() {
    let enabled = FirmwareCpuFlags::new(true, false);
    let mut missing = CpuTopologyBuilder::<2>::new();
    missing
        .insert(processor(1, 4, ProcessorSource::LocalApic, enabled))
        .unwrap();
    assert_eq!(
        missing.freeze(ApicId::new(7)),
        Err(TopologyError::BootProcessorMissing(ApicId::new(7)))
    );

    let mut disabled = CpuTopologyBuilder::<2>::new();
    disabled
        .insert(processor(
            1,
            7,
            ProcessorSource::LocalApic,
            FirmwareCpuFlags::new(false, true),
        ))
        .unwrap();
    assert_eq!(
        disabled.freeze(ApicId::new(7)),
        Err(TopologyError::BootProcessorDisabled(ApicId::new(7)))
    );
}

#[test]
fn registry_binds_ap_startup_to_cpu_and_generation() {
    let enabled = FirmwareCpuFlags::new(true, false);
    let mut builder = CpuTopologyBuilder::<4>::new();
    for (uid, apic) in [(1, 2), (2, 3), (3, 4)] {
        builder
            .insert(processor(uid, apic, ProcessorSource::LocalApic, enabled))
            .unwrap();
    }
    let topology = builder.freeze(ApicId::new(2)).unwrap();
    let mut registry = CpuRegistry::new(topology);
    let ap = CpuIndex::new(1).unwrap();
    let other_ap = CpuIndex::new(2).unwrap();

    assert_eq!(
        registry.state(CpuIndex::BSP),
        Some(CpuLifecycleState::Online)
    );
    assert_eq!(registry.state(ap), Some(CpuLifecycleState::Discovered));
    assert_eq!(registry.online_mask(), CpuMask::singleton(CpuIndex::BSP));
    assert_eq!(
        registry.request_startup(ap, 0),
        Err(CpuRegistryError::InvalidStartupGeneration)
    );
    assert_eq!(registry.request_startup(ap, 41), Ok(()));
    assert_eq!(
        registry.state(ap),
        Some(CpuLifecycleState::StartupRequested)
    );
    assert_eq!(registry.startup_generation(ap), Some(41));

    assert_eq!(
        registry.acknowledge_online(other_ap, 41),
        Err(CpuRegistryError::InvalidState {
            cpu: other_ap,
            state: CpuLifecycleState::Discovered,
        })
    );
    assert_eq!(
        registry.acknowledge_online(ap, 40),
        Err(CpuRegistryError::StaleStartupGeneration {
            expected: 41,
            actual: 40,
        })
    );
    assert_eq!(
        registry.state(ap),
        Some(CpuLifecycleState::StartupRequested)
    );
    assert_eq!(registry.online_mask(), CpuMask::singleton(CpuIndex::BSP));

    assert_eq!(registry.acknowledge_online(ap, 41), Ok(()));
    assert_eq!(registry.state(ap), Some(CpuLifecycleState::Online));
    assert!(registry.online_mask().contains(ap));
    assert_eq!(
        registry.acknowledge_online(ap, 41),
        Err(CpuRegistryError::InvalidState {
            cpu: ap,
            state: CpuLifecycleState::Online,
        })
    );
}

#[test]
fn registry_records_failure_and_guards_offline_transition() {
    let enabled = FirmwareCpuFlags::new(true, false);
    let mut builder = CpuTopologyBuilder::<3>::new();
    for (uid, apic) in [(1, 2), (2, 3), (3, 4)] {
        builder
            .insert(processor(uid, apic, ProcessorSource::LocalApic, enabled))
            .unwrap();
    }
    let mut registry = CpuRegistry::new(builder.freeze(ApicId::new(2)).unwrap());
    let failed_ap = CpuIndex::new(1).unwrap();
    let online_ap = CpuIndex::new(2).unwrap();

    registry.request_startup(failed_ap, 10).unwrap();
    assert_eq!(
        registry.fail_startup(failed_ap, 9),
        Err(CpuRegistryError::StaleStartupGeneration {
            expected: 10,
            actual: 9
        })
    );
    registry.fail_startup(failed_ap, 10).unwrap();
    assert_eq!(registry.state(failed_ap), Some(CpuLifecycleState::Failed));
    assert!(!registry.online_mask().contains(failed_ap));

    registry.request_startup(online_ap, 11).unwrap();
    registry.acknowledge_online(online_ap, 11).unwrap();
    assert_eq!(
        registry.begin_quiesce(online_ap, CpuMask::singleton(online_ap), CpuMask::empty()),
        Err(CpuRegistryError::CpuOwnsRunContext(online_ap))
    );
    assert_eq!(
        registry.begin_quiesce(online_ap, CpuMask::empty(), CpuMask::singleton(online_ap)),
        Err(CpuRegistryError::CpuTargetsActiveShootdown(online_ap))
    );
    assert_eq!(
        registry.begin_quiesce(online_ap, CpuMask::empty(), CpuMask::empty()),
        Ok(())
    );
    assert_eq!(
        registry.state(online_ap),
        Some(CpuLifecycleState::Quiescing)
    );
    assert!(!registry.online_mask().contains(online_ap));
    assert_eq!(registry.mark_offline(online_ap), Ok(()));
    assert_eq!(registry.state(online_ap), Some(CpuLifecycleState::Offline));

    assert_eq!(
        registry.begin_quiesce(CpuIndex::BSP, CpuMask::empty(), CpuMask::empty()),
        Err(CpuRegistryError::BootProcessorCannotOffline)
    );
}
