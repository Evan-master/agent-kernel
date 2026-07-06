use agent_kernel_boot::{BootConfig, BootPhase, BootedKernel};
use agent_kernel_core::{ActionId, AgentImageId, EventKind};

#[test]
fn boot_records_phase_sequence() {
    let booted = BootedKernel::<2, 8, 8, 16, 4, 4, 4, 0, 4, 4>::boot(BootConfig::default())
        .expect("boot flow should fit fixed stores");

    assert_eq!(
        booted.report().phases,
        [
            BootPhase::EnteredKernel,
            BootPhase::KernelInitialized,
            BootPhase::SupervisorHandoffReady,
        ]
    );
    assert_eq!(booted.report().bootstrap_image, AgentImageId::new(1));
}

#[test]
fn boot_records_observe_action_and_verify_events() {
    let config = BootConfig::default().with_boot_action(ActionId::new(99));
    let booted = BootedKernel::<2, 8, 8, 16, 4, 4, 4, 0, 4, 4>::boot(config)
        .expect("boot flow should fit fixed stores");

    let events = booted.kernel().events();
    assert_eq!(events.len(), 7);
    assert_eq!(events[0].kind, EventKind::AgentRegistered);
    assert_eq!(events[1].kind, EventKind::CapabilityGranted);
    assert_eq!(events[2].kind, EventKind::AgentImageRegistered);
    assert_eq!(events[3].kind, EventKind::AgentLaunched);
    assert_eq!(events[4].kind, EventKind::Observation);
    assert_eq!(events[5].kind, EventKind::ActionExecuted);
    assert_eq!(events[6].kind, EventKind::VerificationRequested);
    assert_eq!(events[5].action, Some(ActionId::new(99)));
    assert_eq!(events[6].action, Some(ActionId::new(99)));
}
