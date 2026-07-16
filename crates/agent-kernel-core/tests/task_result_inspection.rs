mod task_result_inspection_support;

use agent_kernel_core::{
    AgentEntryKind, AgentExecutionState, AgentImageKind, EventKind, IntentStatus, KernelError,
    TaskStatus,
};
use task_result_inspection_support::{setup, TestCore, RESULT};

#[test]
fn running_verifier_inspects_completed_result_without_changing_scheduler_state() {
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

    let event = core
        .inspect_task_result(fixture.verifier, fixture.verify_capability, fixture.target)
        .unwrap();

    assert_eq!(event.kind, EventKind::TaskResultInspected);
    assert_eq!(event.agent, fixture.verifier);
    assert_eq!(event.capability, Some(fixture.verify_capability));
    assert_eq!(event.task, Some(fixture.target));
    assert_eq!(event.task_result, Some(RESULT));
    assert_eq!(core.events().len(), events_before + 1);
    assert_eq!(core.tasks()[0].status, TaskStatus::Completed);
    assert_eq!(core.tasks()[0].result, Some(RESULT));
    assert_eq!(core.tasks()[1].status, TaskStatus::Running);
    assert_eq!(core.tasks()[1].result, None);
    assert_eq!(
        core.execution_context(fixture.verifier).unwrap().state,
        AgentExecutionState::Running
    );
    assert!(core.run_queue().is_empty());

    core.verify_task(fixture.verifier, fixture.verify_capability, fixture.target)
        .unwrap();
    assert_eq!(core.tasks()[0].status, TaskStatus::Verified);
    assert_eq!(core.tasks()[0].result, Some(RESULT));
    assert_eq!(core.intents()[0].status, IntentStatus::Fulfilled);
}

#[test]
fn full_event_log_does_not_return_or_mutate_result() {
    let mut core = TestCore::<29>::new();
    let fixture = setup(
        &mut core,
        true,
        true,
        AgentImageKind::Verifier,
        AgentEntryKind::Verifier,
        true,
    );
    assert_eq!(core.events().len(), 29);

    let result =
        core.inspect_task_result(fixture.verifier, fixture.verify_capability, fixture.target);

    assert_eq!(result, Err(KernelError::EventLogFull));
    assert_eq!(core.events().len(), 29);
    assert_eq!(core.tasks()[0].status, TaskStatus::Completed);
    assert_eq!(core.tasks()[0].result, Some(RESULT));
    assert_eq!(core.tasks()[1].status, TaskStatus::Running);
}
