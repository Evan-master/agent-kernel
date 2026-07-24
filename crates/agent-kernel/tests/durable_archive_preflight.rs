use agent_kernel::AgentKernel;
use agent_kernel_core::{
    AgentEntryKind, AgentId, AgentImageDigest, AgentImageKind, IntentKind, KernelError, Operation,
    OperationSet, ResourceKind, SignalKey, VerificationRequirement,
};

type TestKernel = AgentKernel<1, 2, 4, 32, 0, 0, 0, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 1>;

#[test]
fn durable_preflight_binds_actor_authorities_resources_and_proposal() {
    let (kernel, actor, root, storage, archive_authority, storage_authority) = launched_kernel();
    let through = kernel.events()[3].sequence;
    let proposal = kernel.sys_prepare_event_archive(through).unwrap();
    let event_len = kernel.events().len();
    let next_sequence = kernel.next_event_sequence();

    let preflight = kernel
        .preflight_durable_event_archive(
            actor,
            archive_authority,
            storage_authority,
            storage,
            proposal,
        )
        .expect("durable preflight");

    assert_eq!(preflight.actor(), actor);
    assert_eq!(preflight.archive_authority(), archive_authority);
    assert_eq!(preflight.storage_authority(), storage_authority);
    assert_eq!(preflight.root(), root);
    assert_eq!(preflight.storage(), storage);
    assert_eq!(preflight.proposal(), proposal);
    assert_eq!(kernel.events().len(), event_len);
    assert_eq!(kernel.next_event_sequence(), next_sequence);
    assert_eq!(kernel.event_archive_checkpoint(), None);
    assert_eq!(kernel.durable_archive_receipt(), None);
}

#[test]
fn durable_preflight_rejects_wrong_storage_authority_without_mutation() {
    let (kernel, actor, _, storage, archive_authority, _) = launched_kernel();
    let through = kernel.events()[3].sequence;
    let proposal = kernel.sys_prepare_event_archive(through).unwrap();
    let events = kernel.events().to_vec();
    let next_sequence = kernel.next_event_sequence();

    assert_eq!(
        kernel.preflight_durable_event_archive(
            actor,
            archive_authority,
            archive_authority,
            storage,
            proposal,
        ),
        Err(KernelError::ResourceMismatch)
    );
    assert_eq!(kernel.events(), events.as_slice());
    assert_eq!(kernel.next_event_sequence(), next_sequence);
    assert_eq!(kernel.event_archive_checkpoint(), None);
    assert_eq!(kernel.durable_archive_receipt(), None);
}

#[test]
fn durable_preflight_rejects_supervisor_identity_without_mutation() {
    let (kernel, actor, _, storage, archive_authority, storage_authority) =
        launched_kernel_with_kind(AgentImageKind::Supervisor, AgentEntryKind::Supervisor);
    let through = kernel.events()[3].sequence;
    let proposal = kernel.sys_prepare_event_archive(through).unwrap();
    let events = kernel.events().to_vec();

    assert_eq!(
        kernel.preflight_durable_event_archive(
            actor,
            archive_authority,
            storage_authority,
            storage,
            proposal,
        ),
        Err(KernelError::AgentEntryKindMismatch)
    );
    assert_eq!(kernel.events(), events.as_slice());
    assert_eq!(kernel.event_archive_checkpoint(), None);
    assert_eq!(kernel.durable_archive_receipt(), None);
}

fn launched_kernel() -> (
    TestKernel,
    AgentId,
    agent_kernel_core::ResourceId,
    agent_kernel_core::ResourceId,
    agent_kernel_core::CapabilityId,
    agent_kernel_core::CapabilityId,
) {
    launched_kernel_with_kind(AgentImageKind::StateSigner, AgentEntryKind::StateSigner)
}

fn launched_kernel_with_kind(
    image_kind: AgentImageKind,
    entry_kind: AgentEntryKind,
) -> (
    TestKernel,
    AgentId,
    agent_kernel_core::ResourceId,
    agent_kernel_core::ResourceId,
    agent_kernel_core::CapabilityId,
    agent_kernel_core::CapabilityId,
) {
    let mut kernel = TestKernel::new();
    let actor = AgentId::new(1);
    kernel.sys_register_agent(actor).unwrap();
    let root = kernel
        .sys_register_resource(ResourceKind::Workspace, None)
        .unwrap();
    let storage = kernel
        .sys_register_resource(ResourceKind::Device, Some(root))
        .unwrap();
    let archive_authority = kernel
        .sys_grant(
            actor,
            root,
            OperationSet::only(Operation::Act)
                .with(Operation::Verify)
                .with(Operation::Rollback)
                .with(Operation::Delegate),
        )
        .unwrap();
    let storage_authority = kernel
        .sys_grant(actor, storage, OperationSet::only(Operation::Checkpoint))
        .unwrap();
    let intent = kernel
        .sys_declare_intent(
            actor,
            archive_authority,
            root,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .unwrap();
    let task = kernel
        .sys_create_task(actor, archive_authority, intent)
        .unwrap();
    kernel
        .sys_delegate_task(actor, archive_authority, task, actor)
        .unwrap();
    let task_authority = kernel.tasks()[0].delegated_capability.unwrap();
    let image = kernel
        .sys_register_agent_image(
            actor,
            archive_authority,
            root,
            image_kind,
            AgentImageDigest::new([0x81; 32]),
            1,
            1,
        )
        .unwrap();
    kernel
        .sys_verify_agent_image(actor, archive_authority, image)
        .unwrap();
    kernel
        .sys_launch_task_agent(actor, task_authority, task, image, entry_kind)
        .unwrap();
    kernel.sys_accept_task(actor, task).unwrap();
    kernel
        .sys_emit_signal(actor, archive_authority, root, SignalKey::new(81))
        .unwrap();

    (
        kernel,
        actor,
        root,
        storage,
        archive_authority,
        storage_authority,
    )
}
