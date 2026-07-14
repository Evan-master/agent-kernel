#![allow(dead_code)]

use agent_kernel_core::{
    AgentEntryKind, AgentId, AgentImageDigest, AgentImageKind, CapabilityId, DeviceEventId,
    DeviceEventKind, DeviceEventPayload, KernelCore, Operation, OperationSet, ResourceId,
    ResourceKind,
};

pub type RuntimeKernel<const EVENTS: usize, const DRIVER_INVOCATIONS: usize> = KernelCore<
    4,
    4,
    12,
    EVENTS,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    2,
    2,
    3,
    2,
    DRIVER_INVOCATIONS,
>;

#[derive(Copy, Clone)]
pub struct PreparedDriver {
    pub owner: AgentId,
    pub driver: AgentId,
    pub device: ResourceId,
    pub owner_capability: CapabilityId,
    pub driver_capability: CapabilityId,
    pub entry_capability: Option<CapabilityId>,
}

pub fn prepare_bound_driver<const EVENTS: usize, const DRIVER_INVOCATIONS: usize>(
    core: &mut RuntimeKernel<EVENTS, DRIVER_INVOCATIONS>,
) -> PreparedDriver {
    let owner = AgentId::new(1);
    let driver = AgentId::new(2);
    core.register_agent(owner).unwrap();
    core.register_agent(driver).unwrap();
    let device = core.register_resource(ResourceKind::Device, None).unwrap();
    let owner_capability = core
        .grant_capability(
            owner,
            device,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Delegate)
                .with(Operation::Verify)
                .with(Operation::Rollback),
        )
        .unwrap();
    let driver_capability = core
        .grant_capability(
            driver,
            device,
            OperationSet::empty()
                .with(Operation::Observe)
                .with(Operation::Act),
        )
        .unwrap();
    core.bind_driver(owner, owner_capability, device, driver)
        .unwrap();
    PreparedDriver {
        owner,
        driver,
        device,
        owner_capability,
        driver_capability,
        entry_capability: None,
    }
}

pub fn launch_entry<const EVENTS: usize, const DRIVER_INVOCATIONS: usize>(
    core: &mut RuntimeKernel<EVENTS, DRIVER_INVOCATIONS>,
    prepared: &mut PreparedDriver,
    image_kind: AgentImageKind,
    entry_kind: AgentEntryKind,
) {
    let entry_capability = core
        .grant_capability(
            prepared.driver,
            prepared.device,
            OperationSet::empty()
                .with(Operation::Observe)
                .with(Operation::Act),
        )
        .unwrap();
    let image = core
        .register_agent_image(
            prepared.owner,
            prepared.owner_capability,
            prepared.device,
            image_kind,
            AgentImageDigest::new([3; 32]),
            1,
            1,
        )
        .unwrap();
    core.verify_agent_image(prepared.owner, prepared.owner_capability, image)
        .unwrap();
    core.launch_agent(
        prepared.driver,
        entry_capability,
        prepared.device,
        image,
        entry_kind,
        None,
    )
    .unwrap();
    prepared.entry_capability = Some(entry_capability);
}

pub fn prepare_driver<const EVENTS: usize, const DRIVER_INVOCATIONS: usize>(
    core: &mut RuntimeKernel<EVENTS, DRIVER_INVOCATIONS>,
) -> PreparedDriver {
    let mut prepared = prepare_bound_driver(core);
    launch_entry(
        core,
        &mut prepared,
        AgentImageKind::Driver,
        AgentEntryKind::Driver,
    );
    prepared
}

pub fn raise_event<const EVENTS: usize, const DRIVER_INVOCATIONS: usize>(
    core: &mut RuntimeKernel<EVENTS, DRIVER_INVOCATIONS>,
    prepared: PreparedDriver,
    code: u16,
) -> DeviceEventId {
    core.raise_device_event(
        prepared.owner,
        prepared.owner_capability,
        prepared.device,
        DeviceEventKind::Interrupt,
        DeviceEventPayload {
            code,
            value: u64::from(code),
        },
    )
    .unwrap()
}
