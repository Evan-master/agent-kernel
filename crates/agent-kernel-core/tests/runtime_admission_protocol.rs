#[path = "runtime_admission_protocol/support.rs"]
mod support;

use agent_kernel_core::{
    AgentEntryKind, AgentId, AgentImageDigest, AgentImageKind, CapabilityId, EventKind,
    KernelError, Operation, OperationSet, RuntimeAdmissionFailure, RuntimeAdmissionId,
    RuntimeAdmissionStatus, TaskId,
};
use support::{prepared, prepared_pair, TestCore};

fn admit_and_verify<const EVENTS: usize>(
    core: &mut TestCore<EVENTS>,
    supervisor: AgentId,
    authority: CapabilityId,
    target: AgentId,
    task: TaskId,
    task_capability: CapabilityId,
) -> RuntimeAdmissionId {
    let admission = core
        .request_runtime_admission(supervisor, authority, target, task)
        .expect("request succeeds");
    let permit = core
        .prepare_next_runtime_admission()
        .expect("request prepares");
    core.commit_runtime_admission(permit)
        .expect("request commits");
    core.dispatch_next(target).expect("target dispatches");
    core.complete_task(target, task_capability, task)
        .expect("target completes");
    core.verify_task(supervisor, authority, task)
        .expect("target verifies");
    admission
}

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

#[test]
fn prepared_request_can_record_image_verification_rejection() {
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
        .reject_runtime_admission(permit, RuntimeAdmissionFailure::ImageVerification)
        .expect("verification rejection commits");

    assert_eq!(record.status, RuntimeAdmissionStatus::Rejected);
    assert_eq!(
        record.failure,
        Some(RuntimeAdmissionFailure::ImageVerification)
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

#[test]
fn release_requires_verified_idle_target_and_is_read_only_during_preparation() {
    let (mut core, fixture) = prepared::<60>();
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
    core.commit_runtime_admission(permit)
        .expect("request commits");
    let admitted = core
        .runtime_admission(admission)
        .expect("admission remains queryable");
    let event_count = core.events().len();

    assert_eq!(
        core.prepare_runtime_admission_release_batch([admission]),
        Err(KernelError::RuntimeAdmissionReleaseNotReady)
    );
    assert_eq!(core.runtime_admission(admission), Ok(admitted));
    assert_eq!(core.events().len(), event_count);

    core.dispatch_next(fixture.target)
        .expect("target dispatches");
    assert_eq!(
        core.prepare_runtime_admission_release_batch([admission]),
        Err(KernelError::RuntimeAdmissionReleaseNotReady)
    );
    core.complete_task(fixture.target, fixture.task_capability, fixture.task)
        .expect("target completes");
    assert_eq!(
        core.prepare_runtime_admission_release_batch([admission]),
        Err(KernelError::RuntimeAdmissionReleaseNotReady)
    );
    core.verify_task(fixture.supervisor, fixture.authority, fixture.task)
        .expect("target verifies");

    let before_prepare = core.events().len();
    let release = core
        .prepare_runtime_admission_release_batch([admission])
        .expect("verified target prepares for release");
    assert_eq!(release.len(), 1);
    assert_eq!(release.records()[0].id, admission);
    assert_eq!(core.events().len(), before_prepare);
    assert_eq!(
        core.runtime_admission(admission)
            .expect("record remains admitted")
            .status,
        RuntimeAdmissionStatus::Admitted
    );
}

#[test]
fn release_batch_commits_two_records_and_events_in_permit_order() {
    let (mut core, fixture) = prepared_pair::<90>();
    let first = admit_and_verify(
        &mut core,
        fixture.first.supervisor,
        fixture.first.authority,
        fixture.first.target,
        fixture.first.task,
        fixture.first.task_capability,
    );
    let second = admit_and_verify(
        &mut core,
        fixture.first.supervisor,
        fixture.first.authority,
        fixture.second.target,
        fixture.second.task,
        fixture.second.task_capability,
    );
    let before = core.events().len();
    let permit = core
        .prepare_runtime_admission_release_batch([first, second])
        .expect("release batch prepares");

    assert_eq!(permit.records()[0].id, first);
    assert_eq!(permit.records()[1].id, second);
    let released = core
        .commit_runtime_admission_release_batch(permit)
        .expect("release batch commits");

    assert!(released
        .iter()
        .all(|record| record.status == RuntimeAdmissionStatus::Released));
    assert_eq!(core.events().len(), before + 2);
    assert_eq!(
        core.events()[before].kind,
        EventKind::RuntimeAdmissionReleased
    );
    assert_eq!(core.events()[before].runtime_admission, Some(first));
    assert_eq!(
        core.events()[before + 1].kind,
        EventKind::RuntimeAdmissionReleased
    );
    assert_eq!(core.events()[before + 1].runtime_admission, Some(second));
    assert_eq!(
        core.commit_runtime_admission_release_batch(permit),
        Err(KernelError::RuntimeAdmissionReleasePermitStale)
    );
}

#[test]
fn empty_duplicate_and_capacity_failures_leave_release_batch_unchanged() {
    let (mut core, fixture) = prepared_pair::<90>();
    let first = admit_and_verify(
        &mut core,
        fixture.first.supervisor,
        fixture.first.authority,
        fixture.first.target,
        fixture.first.task,
        fixture.first.task_capability,
    );
    let second = admit_and_verify(
        &mut core,
        fixture.first.supervisor,
        fixture.first.authority,
        fixture.second.target,
        fixture.second.task,
        fixture.second.task_capability,
    );
    let before = core.events().len();

    assert_eq!(
        core.prepare_runtime_admission_release_batch([]),
        Err(KernelError::RuntimeAdmissionReleaseBatchEmpty)
    );
    assert_eq!(
        core.prepare_runtime_admission_release_batch([first, first]),
        Err(KernelError::RuntimeAdmissionReleaseDuplicate)
    );
    assert_eq!(core.events().len(), before);
    assert!(core
        .runtime_admissions()
        .iter()
        .all(|record| record.status == RuntimeAdmissionStatus::Admitted));

    while core.events().len() < 89 {
        core.observe(
            fixture.first.supervisor,
            fixture.first.authority,
            fixture.first.resource,
        )
        .expect("filler observation fits");
    }
    assert_eq!(
        core.prepare_runtime_admission_release_batch([first, second]),
        Err(KernelError::EventLogFull)
    );
    assert!(core
        .runtime_admissions()
        .iter()
        .all(|record| record.status == RuntimeAdmissionStatus::Admitted));
}

#[test]
fn semantic_transition_makes_an_older_release_batch_stale_atomically() {
    let (mut core, fixture) = prepared_pair::<90>();
    let first = admit_and_verify(
        &mut core,
        fixture.first.supervisor,
        fixture.first.authority,
        fixture.first.target,
        fixture.first.task,
        fixture.first.task_capability,
    );
    let second = admit_and_verify(
        &mut core,
        fixture.first.supervisor,
        fixture.first.authority,
        fixture.second.target,
        fixture.second.task,
        fixture.second.task_capability,
    );
    let older = core
        .prepare_runtime_admission_release_batch([first])
        .expect("first release prepares");
    let newer = core
        .prepare_runtime_admission_release_batch([second])
        .expect("second release prepares");
    core.commit_runtime_admission_release_batch(newer)
        .expect("second release commits");
    let before_stale = core.events().len();

    assert_eq!(
        core.commit_runtime_admission_release_batch(older),
        Err(KernelError::RuntimeAdmissionReleasePermitStale)
    );
    assert_eq!(core.events().len(), before_stale);
    assert_eq!(
        core.runtime_admission(first)
            .expect("first remains queryable")
            .status,
        RuntimeAdmissionStatus::Admitted
    );
    assert_eq!(
        core.runtime_admission(second)
            .expect("second remains queryable")
            .status,
        RuntimeAdmissionStatus::Released
    );
}

#[test]
fn authorized_requester_can_release_a_task_owned_by_another_supervisor() {
    let (mut core, fixture) = prepared::<70>();
    let requester = AgentId::new(3);
    core.register_agent(requester)
        .expect("requesting supervisor registers");
    let requester_authority = core
        .grant_capability(
            requester,
            fixture.resource,
            OperationSet::only(Operation::Act)
                .with(Operation::Delegate)
                .with(Operation::Verify),
        )
        .expect("requesting supervisor receives authority");
    let requester_image = core
        .register_agent_image(
            requester,
            requester_authority,
            fixture.resource,
            AgentImageKind::Supervisor,
            AgentImageDigest::new([4; 32]),
            1,
            1,
        )
        .expect("requesting supervisor image registers");
    core.verify_agent_image(requester, requester_authority, requester_image)
        .expect("requesting supervisor image verifies");
    core.launch_agent(
        requester,
        requester_authority,
        fixture.resource,
        requester_image,
        AgentEntryKind::Supervisor,
        None,
    )
    .expect("requesting supervisor launches");

    let admission = core
        .request_runtime_admission(requester, requester_authority, fixture.target, fixture.task)
        .expect("alternate requester admits owner task");
    let permit = core
        .prepare_next_runtime_admission()
        .expect("alternate request prepares");
    core.commit_runtime_admission(permit)
        .expect("alternate request commits");
    core.dispatch_next(fixture.target)
        .expect("owned task dispatches");
    core.complete_task(fixture.target, fixture.task_capability, fixture.task)
        .expect("owned task completes");
    core.verify_task(fixture.supervisor, fixture.authority, fixture.task)
        .expect("task owner verifies");

    let release = core
        .prepare_runtime_admission_release_batch([admission])
        .expect("alternate requester release prepares");
    let [released] = core
        .commit_runtime_admission_release_batch(release)
        .expect("alternate requester release commits");

    assert_eq!(released.requester, requester);
    assert_eq!(released.status, RuntimeAdmissionStatus::Released);
    assert_eq!(core.tasks()[0].owner, fixture.supervisor);
}
