use agent_kernel::AgentKernel;
use agent_kernel_core::{
    AgentId, EventKind, MessageId, MessageKind, MessagePayload, MessageStatus, Operation,
    OperationSet, ResourceKind,
};

type TestKernel = AgentKernel<3, 1, 1, 16, 0, 0, 0, 0, 0, 0, 1>;

#[test]
fn facade_retires_orphaned_pending_message_and_reuses_capacity() {
    let mut kernel = TestKernel::new();
    let manager = AgentId::new(1);
    let target = AgentId::new(9);
    kernel
        .sys_register_agent(manager)
        .expect("manager should register");
    let resource = kernel
        .sys_register_resource(ResourceKind::Workspace, None)
        .expect("management resource should fit");
    let authority = kernel
        .sys_grant(manager, resource, OperationSet::only(Operation::Delegate))
        .expect("management authority should fit");
    kernel
        .sys_register_managed_agent(manager, authority, resource, target)
        .expect("managed target should register");
    let orphaned = kernel
        .sys_send_message(
            manager,
            target,
            MessageKind::Request,
            MessagePayload::empty(),
        )
        .expect("pending message should fit");
    kernel
        .sys_retire_managed_agent(manager, authority, target)
        .expect("managed target should retire");

    let retirement = kernel
        .sys_retire_orphaned_message(manager, authority, orphaned)
        .expect("manager should retire orphaned message");
    let replacement = AgentId::new(10);
    kernel
        .sys_register_managed_agent(manager, authority, resource, replacement)
        .expect("replacement target should register");
    let reused = kernel
        .sys_send_message(
            manager,
            replacement,
            MessageKind::Notify,
            MessagePayload::empty(),
        )
        .expect("retired message slot should be reusable");

    assert_eq!(retirement.message(), orphaned);
    assert_eq!(retirement.record().status, MessageStatus::Pending);
    assert_eq!(retirement.actor(), manager);
    assert_eq!(retirement.authority(), authority);
    assert_eq!(retirement.management_resource(), resource);
    assert_eq!(kernel.messages().len(), 1);
    assert_eq!(kernel.messages()[0].id, reused);
    assert_eq!(
        kernel
            .events()
            .iter()
            .find(|event| {
                event.message == Some(orphaned) && event.kind == EventKind::OrphanedMessageRetired
            })
            .map(|event| event.kind),
        Some(EventKind::OrphanedMessageRetired)
    );

    assert_eq!(orphaned, MessageId::new(1));
    assert_eq!(reused, MessageId::new(2));
}
