#[path = "runtime_admission_protocol/support.rs"]
mod support;

use agent_kernel_core::{
    AgentId, CapabilityId, EventKind, KernelError, Operation, OperationSet,
    RuntimeAdmissionFailure, RuntimeAdmissionId, RuntimeAdmissionStatus, TaskId,
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
fn terminal_prefix_compaction_preserves_fifo_and_reuses_the_vacated_slot() {
    let (mut core, fixture) = prepared_pair::<100>();
    let first = core
        .request_runtime_admission(
            fixture.first.supervisor,
            fixture.first.authority,
            fixture.first.target,
            fixture.first.task,
        )
        .expect("first request succeeds");
    let permit = core
        .prepare_next_runtime_admission()
        .expect("first request prepares");
    core.reject_runtime_admission(permit, RuntimeAdmissionFailure::AllocationUnavailable)
        .expect("first request rejects");
    let second = core
        .request_runtime_admission(
            fixture.first.supervisor,
            fixture.first.authority,
            fixture.second.target,
            fixture.second.task,
        )
        .expect("second request succeeds");
    let stale = core
        .prepare_next_runtime_admission()
        .expect("second request prepares");
    let before = core.events().len();

    let receipt = core
        .compact_runtime_admission_prefix(fixture.first.supervisor, fixture.first.authority, first)
        .expect("terminal prefix compacts");

    assert_eq!(receipt.first(), first);
    assert_eq!(receipt.through(), first);
    assert_eq!(receipt.count(), 1);
    assert_eq!(core.runtime_admissions().len(), 1);
    assert_eq!(core.runtime_admissions()[0].id, second);
    assert_eq!(
        core.runtime_admissions()[0].status,
        RuntimeAdmissionStatus::Requested
    );
    assert_eq!(
        core.runtime_admission(first),
        Err(KernelError::RuntimeAdmissionNotFound)
    );
    assert_eq!(core.events().len(), before + 1);
    let event = core.events().last().expect("compaction emits one event");
    assert_eq!(event.kind, EventKind::RuntimeAdmissionCompacted);
    assert_eq!(event.agent, fixture.first.supervisor);
    assert_eq!(event.capability, Some(fixture.first.authority));
    assert_eq!(event.operation, Some(Operation::Delegate));
    assert_eq!(event.runtime_admission, Some(first));
    assert_eq!(event.target_agent, Some(fixture.first.target));
    assert_eq!(event.agent_image, Some(fixture.first.image));
    assert_eq!(
        core.commit_runtime_admission(stale),
        Err(KernelError::RuntimeAdmissionPermitStale)
    );

    let retry = core
        .request_runtime_admission(
            fixture.first.supervisor,
            fixture.first.authority,
            fixture.first.target,
            fixture.first.task,
        )
        .expect("vacated slot accepts a retry");
    assert_eq!(retry.raw(), 3);
    assert_eq!(
        [
            core.runtime_admissions()[0].id.raw(),
            core.runtime_admissions()[1].id.raw(),
        ],
        [2, 3]
    );
}

#[test]
fn nonterminal_and_unauthorized_compaction_fail_atomically() {
    let (mut core, fixture) = prepared_pair::<100>();
    let first = core
        .request_runtime_admission(
            fixture.first.supervisor,
            fixture.first.authority,
            fixture.first.target,
            fixture.first.task,
        )
        .expect("first request succeeds");
    let permit = core
        .prepare_next_runtime_admission()
        .expect("first request prepares");
    core.reject_runtime_admission(permit, RuntimeAdmissionFailure::AllocationUnavailable)
        .expect("first request rejects");
    let second = core
        .request_runtime_admission(
            fixture.first.supervisor,
            fixture.first.authority,
            fixture.second.target,
            fixture.second.task,
        )
        .expect("second request succeeds");
    let records = [core.runtime_admissions()[0], core.runtime_admissions()[1]];
    let event_count = core.events().len();

    let error = core.compact_runtime_admission_prefix(
        fixture.first.supervisor,
        fixture.first.authority,
        RuntimeAdmissionId::new(0),
    );
    assert_eq!(error, Err(KernelError::RuntimeAdmissionNotFound));
    assert_eq!(core.runtime_admissions(), records);
    assert_eq!(core.events().len(), event_count);
    assert_eq!(
        core.compact_runtime_admission_prefix(
            fixture.first.supervisor,
            fixture.first.authority,
            second,
        ),
        Err(KernelError::RuntimeAdmissionCompactionNotReady)
    );
    assert_eq!(core.runtime_admissions(), records);
    assert_eq!(core.events().len(), event_count);

    let observe_only = core
        .derive_capability(
            fixture.first.supervisor,
            fixture.first.authority,
            fixture.first.supervisor,
            OperationSet::only(Operation::Observe),
        )
        .expect("attenuated capability derives");
    let event_count = core.events().len();
    assert_eq!(
        core.compact_runtime_admission_prefix(fixture.first.supervisor, observe_only, first),
        Err(KernelError::OperationDenied)
    );
    assert_eq!(
        core.compact_runtime_admission_prefix(
            fixture.first.target,
            fixture.first.authority,
            first,
        ),
        Err(KernelError::AgentEntryKindMismatch)
    );
    assert_eq!(core.runtime_admissions(), records);
    assert_eq!(core.events().len(), event_count);
}

#[test]
fn compaction_preflights_event_capacity() {
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
    core.reject_runtime_admission(permit, RuntimeAdmissionFailure::MemoryBuild)
        .expect("request rejects");
    while core.events().len() < 40 {
        core.observe(fixture.supervisor, fixture.authority, fixture.resource)
            .expect("filler event fits");
    }
    let record = core.runtime_admissions()[0];

    assert_eq!(
        core.compact_runtime_admission_prefix(fixture.supervisor, fixture.authority, admission),
        Err(KernelError::EventLogFull)
    );
    assert_eq!(core.runtime_admissions(), [record]);
    assert_eq!(core.events().len(), 40);
}

#[test]
fn compaction_invalidates_a_release_permit_for_a_retained_record() {
    let (mut core, fixture) = prepared_pair::<100>();
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
    let first_release = core
        .prepare_runtime_admission_release_batch([first])
        .expect("first release prepares");
    core.commit_runtime_admission_release_batch(first_release)
        .expect("first release commits");
    let stale = core
        .prepare_runtime_admission_release_batch([second])
        .expect("second release prepares");

    core.compact_runtime_admission_prefix(fixture.first.supervisor, fixture.first.authority, first)
        .expect("released prefix compacts");

    assert_eq!(core.runtime_admissions().len(), 1);
    assert_eq!(core.runtime_admissions()[0].id, second);
    assert_eq!(
        core.commit_runtime_admission_release_batch(stale),
        Err(KernelError::RuntimeAdmissionReleasePermitStale)
    );
}
