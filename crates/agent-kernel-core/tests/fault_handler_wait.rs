#[path = "fault_handler_wait/support.rs"]
mod fault_handler_wait_support;

use agent_kernel_core::{EventKind, FaultPolicyAction, KernelError, TaskStatus};
use fault_handler_wait_support::{assert_woken_handler, waiting_fault_route, WaitRouteCore};

#[test]
fn direct_fault_route_wakes_blocked_handler_before_recording_route() {
    let mut core = WaitRouteCore::new();
    let route = waiting_fault_route(&mut core);

    let message = core
        .route_fault_to_handler(route.owner, route.owner_capability, route.fault)
        .expect("fault route should wake its blocked handler");

    assert_woken_handler(&core, route, message);
    let events = core.events();
    assert_eq!(events[events.len() - 3].kind, EventKind::MessageSent);
    assert_eq!(events[events.len() - 2].kind, EventKind::MessageWaitWoken);
    assert_eq!(events[events.len() - 1].kind, EventKind::FaultRouted);
}

#[test]
fn route_policy_wakes_blocked_handler_before_route_and_policy_events() {
    let mut core = WaitRouteCore::new();
    let route = waiting_fault_route(&mut core);

    let outcome = core
        .apply_fault_policy(route.owner, route.owner_capability, route.fault)
        .expect("route policy should wake its blocked handler");
    let message = outcome.message.expect("route policy should send a message");

    assert_eq!(outcome.action, FaultPolicyAction::RouteToHandler);
    assert_woken_handler(&core, route, message);
    let events = core.events();
    assert_eq!(events[events.len() - 4].kind, EventKind::MessageSent);
    assert_eq!(events[events.len() - 3].kind, EventKind::MessageWaitWoken);
    assert_eq!(events[events.len() - 2].kind, EventKind::FaultRouted);
    assert_eq!(events[events.len() - 1].kind, EventKind::FaultPolicyApplied);
}

#[test]
fn route_policy_run_queue_failure_leaves_waiter_and_fault_delivery_unchanged() {
    let mut core = WaitRouteCore::new();
    let route = waiting_fault_route(&mut core);
    core.enqueue_task(route.spare, route.spare_task)
        .expect("spare task should occupy the only run queue slot");
    let event_len = core.events().len();

    assert_eq!(
        core.apply_fault_policy(route.owner, route.owner_capability, route.fault),
        Err(KernelError::RunQueueFull)
    );

    assert_eq!(core.events().len(), event_len);
    assert!(core.messages().is_empty());
    assert_eq!(core.tasks()[0].status, TaskStatus::Waiting);
    assert_eq!(core.tasks()[1].status, TaskStatus::Faulted);
    assert!(core.waiters()[0].active);
    assert_eq!(core.run_queue().len(), 1);
    assert_eq!(core.run_queue()[0].task, route.spare_task);
}

#[test]
fn direct_route_run_queue_failure_leaves_waiter_and_fault_delivery_unchanged() {
    let mut core = WaitRouteCore::new();
    let route = waiting_fault_route(&mut core);
    core.enqueue_task(route.spare, route.spare_task)
        .expect("spare task should occupy the only run queue slot");
    let event_len = core.events().len();

    assert_eq!(
        core.route_fault_to_handler(route.owner, route.owner_capability, route.fault),
        Err(KernelError::RunQueueFull)
    );

    assert_eq!(core.events().len(), event_len);
    assert!(core.messages().is_empty());
    assert_eq!(core.tasks()[0].status, TaskStatus::Waiting);
    assert_eq!(core.tasks()[1].status, TaskStatus::Faulted);
    assert!(core.waiters()[0].active);
    assert_eq!(core.run_queue().len(), 1);
    assert_eq!(core.run_queue()[0].task, route.spare_task);
}
