use agent_kernel_core::{
    agent_image_signer_id, AgentId, AgentImageKind, AgentImageKindScope, AgentImageSignerStatus,
    EventKind, KernelCore, KernelError, Operation, OperationSet, ResourceKind,
};

type SignerCore = KernelCore<2, 2, 4, 24, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2>;

const RESOURCE_MANAGER_PUBLIC_KEY: [u8; 32] = [
    0x59, 0x49, 0x87, 0xda, 0x8e, 0x91, 0xf7, 0x62, 0x4e, 0x12, 0x89, 0x16, 0x0e, 0x34, 0x79, 0x76,
    0xf5, 0x78, 0xeb, 0xe1, 0x82, 0x4d, 0xb0, 0xce, 0x0b, 0x0e, 0xd4, 0x7d, 0x04, 0x54, 0x36, 0xcb,
];
const RESOURCE_MANAGER_SIGNER_ID: [u8; 32] = [
    0x62, 0xe6, 0xf2, 0xe3, 0x61, 0x5d, 0x45, 0x5f, 0x0e, 0x26, 0x0b, 0x73, 0x80, 0x40, 0x14, 0xed,
    0x6b, 0xbd, 0x75, 0x98, 0x4a, 0x11, 0x6b, 0x5c, 0x6b, 0x72, 0x5c, 0x55, 0x66, 0x0b, 0x15, 0x8c,
];

fn prepare(
    core: &mut SignerCore,
    operations: OperationSet,
) -> (
    AgentId,
    agent_kernel_core::CapabilityId,
    agent_kernel_core::ResourceId,
) {
    let actor = AgentId::new(1);
    core.register_agent(actor).expect("actor should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("policy resource should register");
    let authority = core
        .grant_capability(actor, resource, operations)
        .expect("policy authority should fit");
    (actor, authority, resource)
}

fn rotation_authority() -> OperationSet {
    OperationSet::only(Operation::Verify).with(Operation::Rollback)
}

#[test]
fn signer_id_derivation_matches_the_v10_trust_anchor() {
    assert_eq!(
        agent_image_signer_id(RESOURCE_MANAGER_PUBLIC_KEY).bytes(),
        RESOURCE_MANAGER_SIGNER_ID
    );
}

#[test]
fn initial_trust_is_kernel_owned_and_replayable() {
    let mut core = SignerCore::new();
    let (actor, authority, resource) = prepare(&mut core, rotation_authority());
    let scope = AgentImageKindScope::only(AgentImageKind::Supervisor);

    let record = core
        .trust_agent_image_signer(
            actor,
            authority,
            resource,
            RESOURCE_MANAGER_PUBLIC_KEY,
            scope,
            1,
            1,
        )
        .expect("initial signer should become trusted");

    assert_eq!(record.signer_id.bytes(), RESOURCE_MANAGER_SIGNER_ID);
    assert_eq!(record.resource, resource);
    assert_eq!(record.public_key, RESOURCE_MANAGER_PUBLIC_KEY);
    assert_eq!(record.image_kinds, scope);
    assert_eq!(record.minimum_abi, 1);
    assert_eq!(record.maximum_abi, 1);
    assert_eq!(record.status, AgentImageSignerStatus::Active);
    assert_eq!(record.generation, 1);
    assert_eq!(core.agent_image_signer_policy_generation(), 1);
    assert_eq!(core.agent_image_signers(), [record]);

    let event = core.events().last().expect("trust should append one Event");
    let evidence = event
        .agent_image_signer
        .expect("trust Event should retain signer evidence");
    assert_eq!(event.kind, EventKind::AgentImageSignerTrusted);
    assert_eq!(event.agent, actor);
    assert_eq!(event.resource, Some(resource));
    assert_eq!(event.capability, Some(authority));
    assert_eq!(event.operation, Some(Operation::Verify));
    assert_eq!(evidence.signer_id, record.signer_id);
    assert_eq!(evidence.peer_signer_id, None);
    assert_eq!(evidence.public_key, RESOURCE_MANAGER_PUBLIC_KEY);
    assert_eq!(evidence.image_kinds, scope);
    assert_eq!(evidence.status, AgentImageSignerStatus::Active);
    assert_eq!(evidence.policy_generation, 1);
}

#[test]
fn rotation_activates_replacement_then_revokes_previous_at_one_generation() {
    let mut core = SignerCore::new();
    let (actor, authority, resource) = prepare(&mut core, rotation_authority());
    let initial = core
        .trust_agent_image_signer(
            actor,
            authority,
            resource,
            RESOURCE_MANAGER_PUBLIC_KEY,
            AgentImageKindScope::only(AgentImageKind::Supervisor),
            1,
            1,
        )
        .expect("initial signer should become trusted");
    let replacement_key = [0x35; 32];
    let replacement_id = agent_image_signer_id(replacement_key);
    let event_start = core.events().len();
    let next_sequence = core.next_event_sequence();

    let rotation = core
        .rotate_agent_image_signer(
            actor,
            authority,
            resource,
            1,
            initial.signer_id,
            replacement_key,
            AgentImageKindScope::only(AgentImageKind::Worker),
            1,
            2,
        )
        .expect("rotation should commit atomically");

    assert_eq!(rotation.generation(), 2);
    assert_eq!(rotation.previous().signer_id, initial.signer_id);
    assert_eq!(rotation.previous().status, AgentImageSignerStatus::Revoked);
    assert_eq!(rotation.previous().generation, 2);
    assert_eq!(rotation.replacement().signer_id, replacement_id);
    assert_eq!(
        rotation.replacement().status,
        AgentImageSignerStatus::Active
    );
    assert_eq!(rotation.replacement().generation, 2);
    assert_eq!(core.agent_image_signer_policy_generation(), 2);
    assert_eq!(core.agent_image_signers().len(), 2);

    let events = &core.events()[event_start..];
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].sequence, next_sequence);
    assert_eq!(events[0].kind, EventKind::AgentImageSignerTrusted);
    assert_eq!(events[0].operation, Some(Operation::Verify));
    assert_eq!(events[1].sequence, next_sequence + 1);
    assert_eq!(events[1].kind, EventKind::AgentImageSignerRevoked);
    assert_eq!(events[1].operation, Some(Operation::Rollback));
    let trusted = events[0]
        .agent_image_signer
        .expect("replacement Event should retain evidence");
    let revoked = events[1]
        .agent_image_signer
        .expect("revocation Event should retain evidence");
    assert_eq!(trusted.signer_id, replacement_id);
    assert_eq!(trusted.peer_signer_id, Some(initial.signer_id));
    assert_eq!(trusted.status, AgentImageSignerStatus::Active);
    assert_eq!(trusted.policy_generation, 2);
    assert_eq!(revoked.signer_id, initial.signer_id);
    assert_eq!(revoked.peer_signer_id, Some(replacement_id));
    assert_eq!(revoked.status, AgentImageSignerStatus::Revoked);
    assert_eq!(revoked.policy_generation, 2);
}

#[test]
fn stale_generation_and_missing_rollback_authority_leave_policy_unchanged() {
    let mut core = SignerCore::new();
    let actor = AgentId::new(1);
    core.register_agent(actor).expect("actor should register");
    let resource = core
        .register_resource(ResourceKind::Workspace, None)
        .expect("policy resource should register");
    let full_authority = core
        .grant_capability(actor, resource, rotation_authority())
        .expect("full authority should fit");
    let verify_only = core
        .grant_capability(actor, resource, OperationSet::only(Operation::Verify))
        .expect("verify authority should fit");
    let initial = core
        .trust_agent_image_signer(
            actor,
            full_authority,
            resource,
            RESOURCE_MANAGER_PUBLIC_KEY,
            AgentImageKindScope::only(AgentImageKind::Supervisor),
            1,
            1,
        )
        .expect("initial signer should become trusted");
    let records_before = core.agent_image_signers().to_owned();
    let events_before = core.events().len();
    let sequence_before = core.next_event_sequence();

    let stale = core.rotate_agent_image_signer(
        actor,
        full_authority,
        resource,
        0,
        initial.signer_id,
        [0x41; 32],
        AgentImageKindScope::only(AgentImageKind::Worker),
        1,
        1,
    );
    let denied = core.rotate_agent_image_signer(
        actor,
        verify_only,
        resource,
        1,
        initial.signer_id,
        [0x42; 32],
        AgentImageKindScope::only(AgentImageKind::Worker),
        1,
        1,
    );

    assert_eq!(stale, Err(KernelError::AgentImageSignerGenerationStale));
    assert_eq!(denied, Err(KernelError::OperationDenied));
    assert_eq!(core.agent_image_signers(), records_before);
    assert_eq!(core.agent_image_signer_policy_generation(), 1);
    assert_eq!(core.events().len(), events_before);
    assert_eq!(core.next_event_sequence(), sequence_before);
}

#[test]
fn rotation_preflights_signer_and_event_capacity_without_partial_mutation() {
    type OneSignerCore = KernelCore<1, 1, 1, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1>;
    let mut full = OneSignerCore::new();
    let actor = AgentId::new(1);
    full.register_agent(actor).expect("actor should register");
    let resource = full
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should register");
    let authority = full
        .grant_capability(actor, resource, rotation_authority())
        .expect("authority should fit");
    let initial = full
        .trust_agent_image_signer(
            actor,
            authority,
            resource,
            RESOURCE_MANAGER_PUBLIC_KEY,
            AgentImageKindScope::only(AgentImageKind::Supervisor),
            1,
            1,
        )
        .expect("initial signer should fit");
    let events_before = full.events().len();

    assert_eq!(
        full.rotate_agent_image_signer(
            actor,
            authority,
            resource,
            1,
            initial.signer_id,
            [0x51; 32],
            AgentImageKindScope::only(AgentImageKind::Worker),
            1,
            1,
        ),
        Err(KernelError::AgentImageSignerStoreFull)
    );
    assert_eq!(full.agent_image_signers(), [initial]);
    assert_eq!(full.agent_image_signer_policy_generation(), 1);
    assert_eq!(full.events().len(), events_before);

    type TightEventCore = KernelCore<2, 2, 2, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2>;
    let mut tight = TightEventCore::new();
    tight.register_agent(actor).expect("actor should register");
    let resource = tight
        .register_resource(ResourceKind::Workspace, None)
        .expect("resource should register");
    let authority = tight
        .grant_capability(actor, resource, rotation_authority())
        .expect("authority should fit");
    let initial = tight
        .trust_agent_image_signer(
            actor,
            authority,
            resource,
            RESOURCE_MANAGER_PUBLIC_KEY,
            AgentImageKindScope::only(AgentImageKind::Supervisor),
            1,
            1,
        )
        .expect("initial signer should fit");

    assert_eq!(
        tight.rotate_agent_image_signer(
            actor,
            authority,
            resource,
            1,
            initial.signer_id,
            [0x52; 32],
            AgentImageKindScope::only(AgentImageKind::Worker),
            1,
            1,
        ),
        Err(KernelError::EventLogFull)
    );
    assert_eq!(tight.agent_image_signers(), [initial]);
    assert_eq!(tight.agent_image_signer_policy_generation(), 1);
    assert_eq!(tight.events().len(), 3);
}

#[test]
fn invalid_abi_and_duplicate_signer_are_rejected_without_mutation() {
    let mut core = SignerCore::new();
    let (actor, authority, resource) = prepare(&mut core, rotation_authority());

    assert_eq!(
        core.trust_agent_image_signer(
            actor,
            authority,
            resource,
            RESOURCE_MANAGER_PUBLIC_KEY,
            AgentImageKindScope::only(AgentImageKind::Supervisor),
            0,
            1,
        ),
        Err(KernelError::AgentImageSignerPolicyInvalid)
    );
    let initial = core
        .trust_agent_image_signer(
            actor,
            authority,
            resource,
            RESOURCE_MANAGER_PUBLIC_KEY,
            AgentImageKindScope::only(AgentImageKind::Supervisor),
            1,
            1,
        )
        .expect("valid signer should fit");
    let events_before = core.events().len();

    assert_eq!(
        core.trust_agent_image_signer(
            actor,
            authority,
            resource,
            RESOURCE_MANAGER_PUBLIC_KEY,
            AgentImageKindScope::only(AgentImageKind::Worker),
            1,
            1,
        ),
        Err(KernelError::AgentImageSignerAlreadyExists)
    );
    assert_eq!(core.agent_image_signers(), [initial]);
    assert_eq!(core.agent_image_signer_policy_generation(), 1);
    assert_eq!(core.events().len(), events_before);
}
