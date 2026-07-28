use agent_kernel::AgentKernel;
use agent_kernel_core::{
    AgentId, InterruptMode, InterruptRouteStatus, InterruptTarget, Operation, OperationSet,
    ResourceKind,
};

type Kernel = AgentKernel<1, 8, 12, 32, 0, 0, 0, 0, 0, 0>;

#[test]
fn interrupt_route_authority_is_exposed_through_the_kernel_facade() {
    let mut kernel = Kernel::new();
    let agent = AgentId::new(1);
    kernel.sys_register_agent(agent).unwrap();
    let operations = OperationSet::only(Operation::Act)
        .with(Operation::Rollback)
        .with(Operation::Observe);
    let device = kernel
        .sys_register_resource(ResourceKind::Device, None)
        .unwrap();
    let device_capability = kernel.sys_grant(agent, device, operations).unwrap();

    let route = kernel
        .sys_create_interrupt_route(
            agent,
            device_capability,
            device,
            InterruptMode::Msi,
            InterruptTarget::new(0, 0xd0).unwrap(),
            operations,
        )
        .unwrap();
    kernel
        .sys_activate_interrupt_route(agent, route.capability, route.resource)
        .unwrap();
    assert_eq!(
        kernel.interrupt_route(route.resource).unwrap().status(),
        InterruptRouteStatus::Active
    );
    kernel
        .sys_begin_interrupt_route_revoke(agent, route.capability, route.resource)
        .unwrap();
    kernel
        .sys_complete_interrupt_route_revoke(agent, route.capability, route.resource)
        .unwrap();
    assert_eq!(
        kernel.interrupt_route(route.resource).unwrap().status(),
        InterruptRouteStatus::Released
    );
}
