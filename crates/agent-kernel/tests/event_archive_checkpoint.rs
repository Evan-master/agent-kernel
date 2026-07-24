use agent_kernel::AgentKernel;
use agent_kernel_core::{
    AgentEntryKind, AgentId, AgentImageDigest, AgentImageKind, DurableArchiveAnchor,
    DurableArchiveReceipt, DurableArchiveVerificationError, DurableArchiveVerificationRequest,
    DurableArchiveVerifier, DurableSlot, DurableStateDigest, IntentKind, KernelError, Operation,
    OperationSet, ResourceKind, SignalKey, VerificationRequirement,
};

type TestKernel = AgentKernel<1, 1, 4, 24, 0, 0, 0, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 1>;

struct AcceptVerifier;

impl DurableArchiveVerifier for AcceptVerifier {
    fn verify(
        &mut self,
        _request: DurableArchiveVerificationRequest,
    ) -> Result<(), DurableArchiveVerificationError> {
        Ok(())
    }
}

#[test]
fn facade_prepares_and_commits_event_archive_checkpoint() {
    let mut kernel = TestKernel::new();
    let actor = AgentId::new(1);
    kernel.sys_register_agent(actor).unwrap();
    let root = kernel
        .sys_register_resource(ResourceKind::Workspace, None)
        .unwrap();
    let operations = OperationSet::only(Operation::Act)
        .with(Operation::Verify)
        .with(Operation::Checkpoint)
        .with(Operation::Rollback)
        .with(Operation::Delegate);
    let authority = kernel.sys_grant(actor, root, operations).unwrap();
    let intent = kernel
        .sys_declare_intent(
            actor,
            authority,
            root,
            IntentKind::Act,
            VerificationRequirement::Required,
        )
        .unwrap();
    let task = kernel.sys_create_task(actor, authority, intent).unwrap();
    kernel
        .sys_delegate_task(actor, authority, task, actor)
        .unwrap();
    let task_authority = kernel.tasks()[0].delegated_capability.unwrap();
    let image = kernel
        .sys_register_agent_image(
            actor,
            authority,
            root,
            AgentImageKind::StateSigner,
            AgentImageDigest::new([0x40; 32]),
            1,
            1,
        )
        .unwrap();
    kernel
        .sys_verify_agent_image(actor, authority, image)
        .unwrap();
    kernel
        .sys_launch_task_agent(
            actor,
            task_authority,
            task,
            image,
            AgentEntryKind::StateSigner,
        )
        .unwrap();
    kernel.sys_accept_task(actor, task).unwrap();
    kernel
        .sys_emit_signal(actor, authority, root, SignalKey::new(40))
        .unwrap();
    let through = kernel.events()[3].sequence;
    let retained = kernel.events()[4..].to_vec();

    let proposal = kernel.sys_prepare_event_archive(through).unwrap();
    let receipt = DurableArchiveReceipt::new(
        DurableSlot::A,
        root,
        proposal.generation(),
        proposal.digest(),
        DurableStateDigest::new([0x71; 32]),
        DurableStateDigest::new([0x72; 32]),
        1,
        DurableArchiveAnchor::unanchored(),
    )
    .unwrap();
    assert_eq!(
        kernel.sys_commit_event_archive(actor, authority, proposal),
        Err(KernelError::EventArchiveDurabilityRequired)
    );
    assert_eq!(kernel.events().len(), retained.len() + proposal.count());

    let checkpoint = kernel
        .commit_verified_event_archive(
            actor,
            authority,
            authority,
            proposal,
            receipt,
            &mut AcceptVerifier,
        )
        .unwrap();

    assert_eq!(checkpoint.proposal(), proposal);
    assert_eq!(kernel.event_archive_checkpoint(), Some(checkpoint));
    assert_eq!(kernel.durable_archive_receipt(), Some(receipt));
    assert_eq!(kernel.events(), retained.as_slice());
}
