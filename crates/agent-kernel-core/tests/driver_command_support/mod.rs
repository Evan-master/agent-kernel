use agent_kernel_core::{
    AgentId, CapabilityId, DeviceEventId, DriverCommandId, DriverCommandKind, DriverCommandPayload,
    KernelCore, KernelError, OperationSet, ResourceId, ResourceKind,
};

pub type EventKernel<const EVENTS: usize, const DRIVER_COMMANDS: usize> =
    KernelCore<4, 4, 8, EVENTS, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 2, DRIVER_COMMANDS>;

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
