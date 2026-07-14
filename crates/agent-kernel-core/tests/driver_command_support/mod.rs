use agent_kernel_core::{
    AgentEntryKind, AgentId, AgentImageDigest, AgentImageKind, CapabilityId, DeviceEventId,
    DriverCommandId, DriverCommandKind, DriverCommandPayload, DriverEndpointDescriptor, KernelCore,
    KernelError, Operation, OperationSet, ResourceId, ResourceKind,
};

pub type EventKernel<const EVENTS: usize, const DRIVER_COMMANDS: usize> =
    KernelCore<4, 4, 8, EVENTS, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 2, DRIVER_COMMANDS, 2>;

pub fn prepare_bound_device<const EVENTS: usize, const DRIVER_COMMANDS: usize>(
    owner_operations: OperationSet,
    driver_operations: OperationSet,
) -> (
    EventKernel<EVENTS, DRIVER_COMMANDS>,
    AgentId,
    AgentId,
    ResourceId,
    CapabilityId,
    CapabilityId,
) {
    let mut core = EventKernel::<EVENTS, DRIVER_COMMANDS>::new();
    let owner = AgentId::new(1);
    let driver = AgentId::new(2);
    core.register_agent(owner).unwrap();
    core.register_agent(driver).unwrap();
    let device = core.register_resource(ResourceKind::Device, None).unwrap();
    let owner_capability = core
        .grant_capability(owner, device, owner_operations)
        .unwrap();
    let driver_capability = core
        .grant_capability(driver, device, driver_operations)
        .unwrap();
    core.bind_driver(owner, owner_capability, device, driver)
        .unwrap();

    (
        core,
        owner,
        driver,
        device,
        owner_capability,
        driver_capability,
    )
}

#[allow(dead_code)]
pub fn submit<const EVENTS: usize, const DRIVER_COMMANDS: usize>(
    core: &mut EventKernel<EVENTS, DRIVER_COMMANDS>,
    driver: AgentId,
    capability: CapabilityId,
    resource: ResourceId,
    cause: Option<DeviceEventId>,
) -> Result<DriverCommandId, KernelError> {
    core.submit_driver_command(
        driver,
        capability,
        resource,
        cause,
        DriverCommandKind::Write,
        DriverCommandPayload {
            opcode: 3,
            value: 11,
        },
    )
}

#[allow(dead_code)]
pub fn admit_driver<const EVENTS: usize, const DRIVER_COMMANDS: usize>(
    core: &mut EventKernel<EVENTS, DRIVER_COMMANDS>,
    owner: AgentId,
    driver: AgentId,
    device: ResourceId,
) {
    let image_capability = core
        .grant_capability(
            owner,
            device,
            OperationSet::empty()
                .with(Operation::Act)
                .with(Operation::Verify),
        )
        .unwrap();
    let entry_capability = core
        .grant_capability(
            driver,
            device,
            OperationSet::empty()
                .with(Operation::Observe)
                .with(Operation::Act),
        )
        .unwrap();
    let image = core
        .register_agent_image(
            owner,
            image_capability,
            device,
            AgentImageKind::Driver,
            AgentImageDigest::new([3; 32]),
            1,
            1,
        )
        .unwrap();
    core.verify_agent_image(owner, image_capability, image)
        .unwrap();
    core.launch_agent(
        driver,
        entry_capability,
        device,
        image,
        AgentEntryKind::Driver,
        None,
    )
    .unwrap();
}

#[allow(dead_code)]
pub fn register_virtual_endpoint<const EVENTS: usize, const DRIVER_COMMANDS: usize>(
    core: &mut EventKernel<EVENTS, DRIVER_COMMANDS>,
    installer: AgentId,
    capability: CapabilityId,
    device: ResourceId,
) {
    core.register_driver_endpoint(
        installer,
        capability,
        device,
        DriverEndpointDescriptor::virtual_channel(device.raw()),
    )
    .unwrap();
}
