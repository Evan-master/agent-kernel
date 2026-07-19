//! Exhaustive structural coverage for the Resource reference preflight.

use crate::{
    AgentId, Capability, CapabilityId, Event, EventArchiveCheckpoint, EventArchiveProposal,
    KernelCore, KernelError, NamespaceObject, OperationSet, ResourceId,
};

type TestCore = KernelCore<2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2>;

const TARGET: ResourceId = ResourceId::new(7);
const ACTOR: AgentId = AgentId::new(1);
const AUTHORITY: CapabilityId = CapabilityId::new(1);

#[test]
fn every_persistent_non_event_resource_reference_is_rejected() {
    for case in 0..25 {
        let mut core = core_with_target();
        install_reference(&mut core, case);
        assert_eq!(
            core.ensure_resource_record_unreferenced(TARGET),
            Err(KernelError::ResourceRecordRetirementReferenced),
            "reference case {case} escaped the preflight"
        );
    }
}

#[test]
fn historical_event_resource_ids_do_not_block_retirement() {
    let mut core = core_with_target();
    core.events[0].sequence = 1;
    core.events[0].resource = Some(TARGET);
    core.event_len = 1;

    assert_eq!(core.ensure_resource_record_unreferenced(TARGET), Ok(()));
}

fn core_with_target() -> TestCore {
    let mut core = TestCore::new();
    core.resources[0].id = TARGET;
    core.resource_len = 1;
    core
}

fn install_reference(core: &mut TestCore, case: usize) {
    match case {
        0 => {
            core.resources[1].id = ResourceId::new(8);
            core.resources[1].parent = Some(TARGET);
            core.resource_len = 2;
        }
        1 => {
            core.capabilities[0] = Some(Capability {
                id: AUTHORITY,
                agent: ACTOR,
                resource: TARGET,
                operations: OperationSet::empty(),
                revoked: true,
                task: None,
                parent: None,
            });
        }
        2 => {
            core.agents[0].management_resource = Some(TARGET);
            core.agent_len = 1;
        }
        3 => {
            core.agent_entries[0].resource = TARGET;
            core.agent_entry_len = 1;
        }
        4 => {
            core.agent_images[0].resource = TARGET;
            core.agent_image_len = 1;
        }
        5 => {
            core.intents[0].resource = TARGET;
            core.intent_len = 1;
        }
        6 => {
            core.tasks[0].resource = TARGET;
            core.task_len = 1;
        }
        7 => {
            core.runtime_admissions[0].resource = TARGET;
            core.runtime_admission_len = 1;
        }
        8 => {
            core.actions[0].resource = TARGET;
            core.action_len = 1;
        }
        9 => {
            core.observations[0].resource = TARGET;
            core.observation_len = 1;
        }
        10 => {
            core.checkpoints[0].resource = TARGET;
            core.checkpoint_len = 1;
        }
        11 => {
            core.messages[0].payload.resource = Some(TARGET);
            core.message_len = 1;
        }
        12 => {
            core.memory_cells[0].resource = TARGET;
            core.memory_cell_len = 1;
        }
        13 => {
            core.namespace_entries[0].namespace = TARGET;
            core.namespace_entry_len = 1;
        }
        14 => {
            core.namespace_entries[0].object = NamespaceObject::Resource(TARGET);
            core.namespace_entry_len = 1;
        }
        15 => {
            core.faults[0].resource = TARGET;
            core.fault_len = 1;
        }
        16 => {
            core.fault_handlers[0].resource = TARGET;
            core.fault_handler_len = 1;
        }
        17 => {
            core.fault_policies[0].resource = TARGET;
            core.fault_policy_len = 1;
        }
        18 => {
            core.waiters[0].resource = TARGET;
            core.waiter_len = 1;
        }
        19 => {
            core.driver_endpoints[0].resource = TARGET;
            core.driver_endpoint_len = 1;
        }
        20 => {
            core.driver_bindings[0].resource = TARGET;
            core.driver_binding_len = 1;
        }
        21 => {
            core.device_events[0].resource = TARGET;
            core.device_event_len = 1;
        }
        22 => {
            core.driver_commands[0].resource = TARGET;
            core.driver_command_len = 1;
        }
        23 => {
            core.driver_invocations[0].resource = TARGET;
            core.driver_invocation_len = 1;
        }
        24 => {
            let mut event = Event::empty();
            event.sequence = 1;
            let proposal = EventArchiveProposal::from_segment(None, &[event]).unwrap();
            core.event_archive_checkpoint = Some(EventArchiveCheckpoint::new(
                proposal, ACTOR, AUTHORITY, TARGET,
            ));
        }
        _ => unreachable!(),
    }
}
