mod sector_device;

pub use sector_device::SectorDevice;

use agent_kernel::AgentKernel;
use agent_kernel_core::{
    AgentEntryKind, AgentId, AgentImageDigest, AgentImageId, AgentImageKind, CapabilityId,
    IntentKind, Operation, OperationSet, ResourceId, ResourceKind, SignalKey, TaskId,
    VerificationRequirement,
};

pub type ArchiveKernel = AgentKernel<1, 2, 4, 32, 0, 0, 0, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 1>;

pub struct ArchiveFixture {
    pub kernel: ArchiveKernel,
    pub actor: AgentId,
    pub task: TaskId,
    pub image: AgentImageId,
    pub root: ResourceId,
    pub storage: ResourceId,
    pub archive_authority: CapabilityId,
    pub storage_authority: CapabilityId,
}

pub fn launched_archive_kernel() -> ArchiveFixture {
    let mut kernel = ArchiveKernel::new();
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
            AgentImageKind::Supervisor,
            AgentImageDigest::new([0x95; 32]),
            1,
            1,
        )
        .unwrap();
    kernel
        .sys_verify_agent_image(actor, archive_authority, image)
        .unwrap();
    kernel
        .sys_launch_task_agent(
            actor,
            task_authority,
            task,
            image,
            AgentEntryKind::Supervisor,
        )
        .unwrap();
    kernel.sys_accept_task(actor, task).unwrap();
    kernel
        .sys_emit_signal(actor, archive_authority, root, SignalKey::new(95))
        .unwrap();

    ArchiveFixture {
        kernel,
        actor,
        task,
        image,
        root,
        storage,
        archive_authority,
        storage_authority,
    }
}
