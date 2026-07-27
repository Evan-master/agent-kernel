use agent_kernel_core::{
    AgentId, DriverEndpointDescriptor, DriverResourceTreeSpec, EventKind, KernelCore, KernelError,
    Operation, OperationSet, ResourceId, ResourceKind,
};

type TreeCore<const RESOURCES: usize, const CAPS: usize, const EVENTS: usize> =
    KernelCore<1, RESOURCES, CAPS, EVENTS, 0, 0, 0, 0, 0, 0>;

const OWNER: AgentId = AgentId::new(1);

#[test]
fn driver_resource_tree_commits_root_regions_capabilities_and_endpoints_in_order() {
    let (mut core, parent, authority) = parented_core::<8, 8, 16>();
    let spec = DriverResourceTreeSpec::new(
        ResourceKind::Network,
        [
            Some(DriverEndpointDescriptor::mmio(0x8000_0000, 0x1000)),
            Some(DriverEndpointDescriptor::port(0xc000, 0x100)),
            None,
            None,
            None,
            None,
        ],
    );
    let event_start = core.events().len();

    let tree = core
        .create_driver_resource_tree(OWNER, Some((parent, authority)), owner_operations(), spec)
        .unwrap();

    assert_eq!(tree.root().resource, ResourceId::new(2));
    assert_eq!(tree.region_count(), 2);
    let mmio = tree.region(0).unwrap();
    let port = tree.region(1).unwrap();
    assert_eq!(mmio.slot(), 0);
    assert_eq!(mmio.resource(), ResourceId::new(3));
    assert_eq!(
        mmio.descriptor(),
        DriverEndpointDescriptor::mmio(0x8000_0000, 0x1000)
    );
    assert_eq!(port.slot(), 1);
    assert_eq!(port.resource(), ResourceId::new(4));
    assert_eq!(
        port.descriptor(),
        DriverEndpointDescriptor::port(0xc000, 0x100)
    );

    let resources = core.resources();
    assert_eq!(resources.len(), 4);
    assert_eq!(resources[1].kind, ResourceKind::Network);
    assert_eq!(resources[1].parent, Some(parent));
    assert_eq!(resources[1].owner, Some(OWNER));
    assert_eq!(resources[2].parent, Some(tree.root().resource));
    assert_eq!(resources[3].parent, Some(tree.root().resource));
    assert_eq!(core.driver_endpoints().len(), 2);
    assert_eq!(core.driver_endpoints()[0].resource, mmio.resource());
    assert_eq!(core.driver_endpoints()[1].resource, port.resource());
    for capability in [tree.root().capability, mmio.capability(), port.capability()] {
        let record = core.capability(capability).unwrap();
        assert_eq!(record.agent, OWNER);
        assert_eq!(record.operations, owner_operations());
        assert!(!record.revoked);
    }

    assert_eq!(
        core.events()[event_start..]
            .iter()
            .map(|event| event.kind)
            .collect::<Vec<_>>(),
        [
            EventKind::ResourceCreated,
            EventKind::CapabilityGranted,
            EventKind::ResourceCreated,
            EventKind::CapabilityGranted,
            EventKind::DriverEndpointRegistered,
            EventKind::ResourceCreated,
            EventKind::CapabilityGranted,
            EventKind::DriverEndpointRegistered,
        ]
    );
}

#[test]
fn driver_resource_tree_rejects_sibling_overlap_without_allocating_ids_or_events() {
    let (mut core, parent, authority) = parented_core::<8, 8, 16>();
    let invalid = DriverResourceTreeSpec::new(
        ResourceKind::Device,
        [
            Some(DriverEndpointDescriptor::mmio(0x4000, 0x100)),
            Some(DriverEndpointDescriptor::mmio(0x4080, 0x20)),
            None,
            None,
            None,
            None,
        ],
    );
    let resources_before = core.resources().len();
    let capabilities_before = core.capability_count();
    let events_before = core.events().len();

    assert_eq!(
        core.create_driver_resource_tree(
            OWNER,
            Some((parent, authority)),
            owner_operations(),
            invalid,
        ),
        Err(KernelError::DriverEndpointOverlap)
    );
    assert_eq!(core.resources().len(), resources_before);
    assert_eq!(core.capability_count(), capabilities_before);
    assert!(core.driver_endpoints().is_empty());
    assert_eq!(core.events().len(), events_before);

    let valid = DriverResourceTreeSpec::new(
        ResourceKind::Device,
        [
            Some(DriverEndpointDescriptor::mmio(0x4000, 0x100)),
            Some(DriverEndpointDescriptor::mmio(0x5000, 0x20)),
            None,
            None,
            None,
            None,
        ],
    );
    let tree = core
        .create_driver_resource_tree(OWNER, Some((parent, authority)), owner_operations(), valid)
        .unwrap();
    assert_eq!(tree.root().resource, ResourceId::new(2));
}

#[test]
fn driver_resource_tree_rejects_overlap_with_an_existing_endpoint_atomically() {
    let (mut core, parent, authority) = parented_core::<8, 8, 16>();
    let existing = core.register_resource(ResourceKind::Device, None).unwrap();
    let existing_capability = core
        .grant_capability(OWNER, existing, OperationSet::only(Operation::Delegate))
        .unwrap();
    core.register_driver_endpoint(
        OWNER,
        existing_capability,
        existing,
        DriverEndpointDescriptor::mmio(0x8000, 0x1000),
    )
    .unwrap();
    let spec = DriverResourceTreeSpec::new(
        ResourceKind::Device,
        [
            Some(DriverEndpointDescriptor::mmio(0x8800, 0x100)),
            None,
            None,
            None,
            None,
            None,
        ],
    );

    assert_unchanged_after(
        &mut core,
        |core| {
            core.create_driver_resource_tree(
                OWNER,
                Some((parent, authority)),
                owner_operations(),
                spec,
            )
        },
        KernelError::DriverEndpointOverlap,
    );
}

#[test]
fn driver_resource_tree_preflights_authority_shape_and_every_store_capacity() {
    let empty =
        DriverResourceTreeSpec::new(ResourceKind::Device, [None, None, None, None, None, None]);
    let one_region = DriverResourceTreeSpec::new(
        ResourceKind::Device,
        [
            Some(DriverEndpointDescriptor::mmio(0x1000, 0x100)),
            None,
            None,
            None,
            None,
            None,
        ],
    );

    let (mut core, parent, authority) = parented_core::<4, 4, 8>();
    assert_unchanged_after(
        &mut core,
        |core| {
            core.create_driver_resource_tree(
                OWNER,
                Some((parent, authority)),
                owner_operations(),
                empty,
            )
        },
        KernelError::DriverResourceTreeEmpty,
    );
    assert_unchanged_after(
        &mut core,
        |core| {
            core.create_driver_resource_tree(
                OWNER,
                Some((parent, authority)),
                OperationSet::only(Operation::Act),
                one_region,
            )
        },
        KernelError::OperationDenied,
    );

    let (mut resources_full, parent, authority) = parented_core::<2, 4, 8>();
    assert_unchanged_after(
        &mut resources_full,
        |core| {
            core.create_driver_resource_tree(
                OWNER,
                Some((parent, authority)),
                owner_operations(),
                one_region,
            )
        },
        KernelError::ResourceStoreFull,
    );

    let (mut capabilities_full, parent, authority) = parented_core::<4, 2, 8>();
    assert_unchanged_after(
        &mut capabilities_full,
        |core| {
            core.create_driver_resource_tree(
                OWNER,
                Some((parent, authority)),
                owner_operations(),
                one_region,
            )
        },
        KernelError::CapabilityStoreFull,
    );

    let (mut events_full, parent, authority) = parented_core::<4, 4, 3>();
    assert_unchanged_after(
        &mut events_full,
        |core| {
            core.create_driver_resource_tree(
                OWNER,
                Some((parent, authority)),
                owner_operations(),
                one_region,
            )
        },
        KernelError::EventLogFull,
    );
}

fn parented_core<const RESOURCES: usize, const CAPS: usize, const EVENTS: usize>() -> (
    TreeCore<RESOURCES, CAPS, EVENTS>,
    ResourceId,
    agent_kernel_core::CapabilityId,
) {
    let mut core = TreeCore::new();
    core.register_agent(OWNER).unwrap();
    let parent = core
        .register_resource(ResourceKind::Workspace, None)
        .unwrap();
    let authority = core
        .grant_capability(OWNER, parent, owner_operations())
        .unwrap();
    (core, parent, authority)
}

fn assert_unchanged_after<const RESOURCES: usize, const CAPS: usize, const EVENTS: usize, F>(
    core: &mut TreeCore<RESOURCES, CAPS, EVENTS>,
    operation: F,
    expected: KernelError,
) where
    F: FnOnce(
        &mut TreeCore<RESOURCES, CAPS, EVENTS>,
    ) -> Result<agent_kernel_core::DriverResourceTree, KernelError>,
{
    let resource_count = core.resources().len();
    let capability_count = core.capability_count();
    let endpoint_count = core.driver_endpoints().len();
    let event_count = core.events().len();
    let sequence = core.next_event_sequence();

    assert_eq!(operation(core), Err(expected));
    assert_eq!(core.resources().len(), resource_count);
    assert_eq!(core.capability_count(), capability_count);
    assert_eq!(core.driver_endpoints().len(), endpoint_count);
    assert_eq!(core.events().len(), event_count);
    assert_eq!(core.next_event_sequence(), sequence);
}

const fn owner_operations() -> OperationSet {
    OperationSet::only(Operation::Observe)
        .with(Operation::Act)
        .with(Operation::Rollback)
        .with(Operation::Delegate)
}
