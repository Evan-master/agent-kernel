use agent_kernel_boot::{BootConfig, BootedKernel};
use agent_kernel_core::{MemoryValue, Operation, OperationSet, ResourceKind};

type MemoryBoot = BootedKernel<1, 2, 2, 20, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1>;

#[test]
fn boot_profile_can_provision_memory_cell_capacity() {
    let mut booted = MemoryBoot::boot(BootConfig::default()).unwrap();
    let report = *booted.report();
    let outcome = booted
        .kernel_mut()
        .sys_create_resource(
            report.bootstrap_agent,
            ResourceKind::Memory,
            Some((report.bootstrap_resource, report.bootstrap_capability)),
            OperationSet::only(Operation::Observe).with(Operation::Act),
        )
        .unwrap();
    let value = MemoryValue::new([0x4000, 4096, 3, 1]);

    let cell = booted
        .kernel_mut()
        .sys_create_memory_cell(
            report.bootstrap_agent,
            outcome.capability,
            outcome.resource,
            value,
        )
        .unwrap();

    assert_eq!(cell.raw(), 1);
    assert_eq!(booted.kernel().memory_cells().len(), 1);
    assert_eq!(booted.kernel().memory_cells()[0].value, value);
}
