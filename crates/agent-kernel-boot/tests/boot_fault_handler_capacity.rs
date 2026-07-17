use agent_kernel_boot::{BootConfig, BootedKernel};
use agent_kernel_core::{AgentId, FaultKind, FaultPolicyAction};

type FaultHandlerBoot = BootedKernel<2, 1, 1, 12, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1>;

#[test]
fn booted_kernel_forwards_fault_handler_and_policy_capacities() {
    let mut booted = FaultHandlerBoot::boot(BootConfig::default()).unwrap();
    let report = *booted.report();
    let handler = AgentId::new(2);
    let kernel = booted.kernel_mut();
    kernel.sys_register_agent(handler).unwrap();

    let binding = kernel
        .sys_install_fault_handler(
            report.bootstrap_agent,
            report.bootstrap_capability,
            report.bootstrap_resource,
            FaultKind::ExecutionTrap,
            handler,
        )
        .unwrap();
    let policy = kernel
        .sys_install_fault_policy(
            report.bootstrap_agent,
            report.bootstrap_capability,
            report.bootstrap_resource,
            FaultKind::ExecutionTrap,
            FaultPolicyAction::RouteToHandler,
        )
        .unwrap();

    assert_eq!(binding.raw(), 1);
    assert_eq!(policy.raw(), 1);
    assert_eq!(kernel.fault_handlers().len(), 1);
    assert_eq!(kernel.fault_policies().len(), 1);
}
