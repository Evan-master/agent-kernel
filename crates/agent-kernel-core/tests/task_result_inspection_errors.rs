mod task_result_inspection_support;

use agent_kernel_core::{AgentEntryKind, AgentImageKind, KernelError};
use task_result_inspection_support::{setup, TestCore};

#[test]
fn inspection_requires_resource_verify_capability() {
    let mut core = TestCore::<40>::new();
    let fixture = setup(
        &mut core,
        true,
        true,
        AgentImageKind::Verifier,
        AgentEntryKind::Verifier,
        true,
    );
    let events_before = core.events().len();

    let result = core.inspect_task_result(
        fixture.verifier,
        fixture.verifier_task_capability,
        fixture.target,
    );

    assert_eq!(result, Err(KernelError::OperationDenied));
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn inspection_requires_running_verifier_entry() {
    let mut core = TestCore::<40>::new();
    let fixture = setup(
        &mut core,
        true,
        true,
        AgentImageKind::Verifier,
        AgentEntryKind::Verifier,
        false,
    );
    let events_before = core.events().len();

    let result =
        core.inspect_task_result(fixture.verifier, fixture.verify_capability, fixture.target);

    assert_eq!(result, Err(KernelError::TaskStatusMismatch));
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn inspection_rejects_non_verifier_entry() {
    let mut core = TestCore::<40>::new();
    let fixture = setup(
        &mut core,
        true,
        true,
        AgentImageKind::Worker,
        AgentEntryKind::Worker,
        true,
    );
    let events_before = core.events().len();

    let result =
        core.inspect_task_result(fixture.verifier, fixture.verify_capability, fixture.target);

    assert_eq!(result, Err(KernelError::AgentEntryKindMismatch));
    assert_eq!(core.events().len(), events_before);
}

#[test]
fn inspection_requires_completed_target_with_result() {
    let mut missing_core = TestCore::<40>::new();
    let missing = setup(
        &mut missing_core,
        false,
        true,
        AgentImageKind::Verifier,
        AgentEntryKind::Verifier,
        true,
    );
    assert_eq!(
        missing_core.inspect_task_result(
            missing.verifier,
            missing.verify_capability,
            missing.target,
        ),
        Err(KernelError::TaskResultMissing)
    );

    let mut running_core = TestCore::<40>::new();
    let running = setup(
        &mut running_core,
        true,
        false,
        AgentImageKind::Verifier,
        AgentEntryKind::Verifier,
        true,
    );
    assert_eq!(
        running_core.inspect_task_result(
            running.verifier,
            running.verify_capability,
            running.target,
        ),
        Err(KernelError::TaskStatusMismatch)
    );
}
