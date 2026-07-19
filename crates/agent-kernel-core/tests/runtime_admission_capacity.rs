#[allow(dead_code)]
#[path = "runtime_admission_protocol/support.rs"]
mod support;

use agent_kernel_core::{
    KernelError, RuntimeAdmissionFailure, RuntimeAdmissionId, RuntimeAdmissionStatus,
};
use support::{prepared, prepared_with_capacity};

#[test]
fn default_capacity_tracks_the_task_store() {
    let (core, _) = prepared::<100>();

    assert_eq!(core.runtime_admission_capacity(), 2);
}

#[test]
fn zero_capacity_rejects_a_valid_request_without_mutation() {
    let (mut core, fixture) = prepared_with_capacity::<100, 0>();
    let event_count = core.events().len();

    assert_eq!(
        core.request_runtime_admission(
            fixture.supervisor,
            fixture.authority,
            fixture.target,
            fixture.task,
        ),
        Err(KernelError::RuntimeAdmissionStoreFull)
    );
    assert_eq!(core.runtime_admission_capacity(), 0);
    assert!(core.runtime_admissions().is_empty());
    assert_eq!(core.events().len(), event_count);
}

#[test]
fn independent_capacity_retains_three_rejections_for_one_task_and_reuses_prefix_slots() {
    let (mut core, fixture) = prepared_with_capacity::<100, 3>();
    let mut admissions = [RuntimeAdmissionId::new(0); 3];

    for (index, admission) in admissions.iter_mut().enumerate() {
        *admission = core
            .request_runtime_admission(
                fixture.supervisor,
                fixture.authority,
                fixture.target,
                fixture.task,
            )
            .expect("terminal attempts can be retained");
        assert_eq!(admission.raw(), index as u64 + 1);
        let permit = core
            .prepare_next_runtime_admission()
            .expect("retry prepares");
        core.reject_runtime_admission(permit, RuntimeAdmissionFailure::AllocationUnavailable)
            .expect("retry rejection records");
    }

    assert_eq!(core.runtime_admission_capacity(), 3);
    assert_eq!(core.tasks().len(), 1);
    assert_eq!(core.runtime_admissions().len(), 3);
    assert!(core
        .runtime_admissions()
        .iter()
        .all(|record| record.status == RuntimeAdmissionStatus::Rejected));
    assert_eq!(
        core.request_runtime_admission(
            fixture.supervisor,
            fixture.authority,
            fixture.target,
            fixture.task,
        ),
        Err(KernelError::RuntimeAdmissionStoreFull)
    );
    assert_eq!(
        core.prepare_runtime_admission_release_batch(admissions),
        Err(KernelError::RuntimeAdmissionReleaseNotReady)
    );
    assert_eq!(
        core.prepare_runtime_admission_release_batch([
            admissions[0],
            admissions[1],
            admissions[2],
            admissions[0],
        ]),
        Err(KernelError::RuntimeAdmissionReleaseBatchTooLarge)
    );

    core.compact_runtime_admission_prefix(fixture.supervisor, fixture.authority, admissions[1])
        .expect("terminal prefix compacts");
    let fourth = core
        .request_runtime_admission(
            fixture.supervisor,
            fixture.authority,
            fixture.target,
            fixture.task,
        )
        .expect("vacated capacity accepts the next retry");

    assert_eq!(fourth.raw(), 4);
    assert_eq!(core.runtime_admissions().len(), 2);
    assert_eq!(
        [
            core.runtime_admissions()[0].id.raw(),
            core.runtime_admissions()[1].id.raw(),
        ],
        [3, 4]
    );
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
    assert_eq!(core.runtime_admissions().len(), 2);
    assert_eq!(core.events().len(), event_count);
}
