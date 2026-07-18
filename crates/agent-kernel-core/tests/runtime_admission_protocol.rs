#[path = "runtime_admission_protocol/support.rs"]
mod support;

use agent_kernel_core::{
    EventKind, KernelError, Operation, OperationSet, RuntimeAdmissionFailure,
    RuntimeAdmissionStatus,
};
use support::prepared;

#[test]
fn request_requires_supervisor_identity_and_delegate_authority() {
    let (mut core, fixture) = prepared::<40>();
    let observe_only = core
        .derive_capability(
            fixture.supervisor,
            fixture.authority,
            fixture.supervisor,
            OperationSet::only(Operation::Observe),
        )
        .expect("attenuated authority derives");
    let event_count = core.events().len();

    assert_eq!(
        core.request_runtime_admission(
            fixture.supervisor,
            observe_only,
            fixture.target,
            fixture.task,
        ),
        Err(KernelError::OperationDenied)
    );
    assert_eq!(
        core.request_runtime_admission(
            fixture.target,
            fixture.authority,
            fixture.target,
            fixture.task,
        ),
        Err(KernelError::AgentEntryKindMismatch)
    );
    assert!(core.runtime_admissions().is_empty());
    assert_eq!(core.events().len(), event_count);
    assert!(core.run_queue().is_empty());
}

#[test]
fn supervisor_request_binds_authority_target_task_image_and_event() {
    let (mut core, fixture) = prepared::<40>();
    let before = core.events().len();

    let admission = core
        .request_runtime_admission(
            fixture.supervisor,
            fixture.authority,
            fixture.target,
            fixture.task,
        )
        .expect("request succeeds");
    let record = core
        .runtime_admission(admission)
        .expect("record remains queryable");
    let event = core.events().last().expect("request emits event");

    assert_eq!(admission.raw(), 1);
    assert_eq!(core.runtime_admissions(), [record]);
    assert_eq!(record.requester, fixture.supervisor);
    assert_eq!(record.authority, fixture.authority);
    assert_eq!(record.target, fixture.target);
    assert_eq!(record.task, fixture.task);
    assert_eq!(record.image, fixture.image);
    assert_eq!(record.resource, fixture.resource);
    assert_eq!(record.status, RuntimeAdmissionStatus::Requested);
    assert_eq!(record.failure, None);
    assert_eq!(core.events().len(), before + 1);
    assert_eq!(event.kind, EventKind::RuntimeAdmissionRequested);
    assert_eq!(event.runtime_admission, Some(admission));
    assert_eq!(event.target_agent, Some(fixture.target));
    assert!(core.run_queue().is_empty());
}

#[test]
fn permit_is_read_only_and_commit_atomically_admits_and_queues() {
    let (mut core, fixture) = prepared::<40>();
    let admission = core
        .request_runtime_admission(
            fixture.supervisor,
            fixture.authority,
            fixture.target,
            fixture.task,
        )
        .expect("request succeeds");
    let before = core.events().len();
    let permit = core
        .prepare_next_runtime_admission()
        .expect("pending request prepares");

    assert_eq!(permit.admission(), admission);
    assert_eq!(permit.target(), fixture.target);
    assert_eq!(permit.task(), fixture.task);
    assert_eq!(permit.image(), fixture.image);
    assert_eq!(core.events().len(), before);
    assert!(core.run_queue().is_empty());

    let record = core
        .commit_runtime_admission(permit)
        .expect("prepared request commits");
    assert_eq!(record.status, RuntimeAdmissionStatus::Admitted);
    assert_eq!(core.events().len(), before + 2);
    assert_eq!(
        core.events()[before].kind,
        EventKind::RuntimeAdmissionAdmitted
    );
    assert_eq!(core.events()[before + 1].kind, EventKind::TaskQueued);
    assert_eq!(core.run_queue()[0].agent, fixture.target);
    assert_eq!(core.run_queue()[0].task, fixture.task);
    assert_eq!(
        core.commit_runtime_admission(permit),
        Err(KernelError::RuntimeAdmissionPermitStale)
    );
}

#[test]
fn duplicate_and_commit_capacity_failures_are_atomic() {
    let (mut core, fixture) = prepared::<18>();
    let admission = core
        .request_runtime_admission(
            fixture.supervisor,
            fixture.authority,
            fixture.target,
            fixture.task,
        )
        .expect("request succeeds");
    let event_count = core.events().len();
    assert_eq!(
        core.request_runtime_admission(
            fixture.supervisor,
            fixture.authority,
            fixture.target,
            fixture.task,
        ),
        Err(KernelError::RuntimeAdmissionDuplicate)
    );
    assert_eq!(core.events().len(), event_count);
    assert_eq!(core.runtime_admissions().len(), 1);

    while core.events().len() < 17 {
        core.observe(fixture.supervisor, fixture.authority, fixture.resource)
            .expect("filler observation fits");
    }
    let permit = core
        .prepare_next_runtime_admission()
        .expect_err("two event slots are required");
    assert_eq!(permit, KernelError::EventLogFull);
    assert_eq!(
        core.runtime_admission(admission)
            .expect("record exists")
            .status,
        RuntimeAdmissionStatus::Requested
    );
    assert!(core.run_queue().is_empty());
}

#[test]
fn prepared_request_can_record_bounded_physical_rejection() {
    let (mut core, fixture) = prepared::<40>();
    let admission = core
        .request_runtime_admission(
            fixture.supervisor,
            fixture.authority,
            fixture.target,
            fixture.task,
        )
        .expect("request succeeds");
    let permit = core
        .prepare_next_runtime_admission()
        .expect("request prepares");

    let record = core
        .reject_runtime_admission(permit, RuntimeAdmissionFailure::AllocationUnavailable)
        .expect("rejection commits");

    assert_eq!(record.status, RuntimeAdmissionStatus::Rejected);
    assert_eq!(
        record.failure,
        Some(RuntimeAdmissionFailure::AllocationUnavailable)
    );
    assert_eq!(
        core.runtime_admission(admission).expect("record exists"),
        record
    );
    assert_eq!(
        core.events().last().expect("rejection event").kind,
        EventKind::RuntimeAdmissionRejected
    );
    assert!(core.run_queue().is_empty());
}
