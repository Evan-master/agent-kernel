use agent_kernel_boot::{BootConfig, BootedKernel};
use agent_kernel_core::{
    ActionId, AgentId, KernelError, NamespaceEntryId, NamespaceKey, NamespaceObject, ResourceKind,
};

type NamespaceBoot =
    BootedKernel<1, 1, 1, 16, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1>;

#[test]
fn boot_profile_provisions_and_reuses_namespace_capacity() {
    let mut booted = NamespaceBoot::boot(BootConfig::new(
        AgentId::new(1),
        ResourceKind::Workspace,
        ActionId::new(1),
    ))
    .unwrap();
    let report = *booted.report();
    let first = booted
        .kernel_mut()
        .sys_bind_namespace_entry(
            report.bootstrap_agent,
            report.bootstrap_capability,
            report.bootstrap_resource,
            NamespaceKey::new(1),
            NamespaceObject::Resource(report.bootstrap_resource),
        )
        .unwrap();
    assert_eq!(first, NamespaceEntryId::new(1));
    assert_eq!(
        booted.kernel_mut().sys_bind_namespace_entry(
            report.bootstrap_agent,
            report.bootstrap_capability,
            report.bootstrap_resource,
            NamespaceKey::new(2),
            NamespaceObject::Agent(report.bootstrap_agent),
        ),
        Err(KernelError::NamespaceEntryStoreFull)
    );

    booted
        .kernel_mut()
        .sys_retire_namespace_entry(report.bootstrap_agent, report.bootstrap_capability, first)
        .unwrap();
    let fresh = booted
        .kernel_mut()
        .sys_bind_namespace_entry(
            report.bootstrap_agent,
            report.bootstrap_capability,
            report.bootstrap_resource,
            NamespaceKey::new(2),
            NamespaceObject::Agent(report.bootstrap_agent),
        )
        .unwrap();

    assert_eq!(fresh, NamespaceEntryId::new(2));
    assert_eq!(booted.kernel().namespace_entries().len(), 1);
}
