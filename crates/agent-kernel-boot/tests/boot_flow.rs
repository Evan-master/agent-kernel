use agent_kernel_boot::{BootConfig, BootPhase, BootedKernel};
use agent_kernel_core::{ActionId, EventKind};

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
}

#[test]
fn boot_records_observe_action_and_verify_events() {
    let config = BootConfig::default().with_boot_action(ActionId::new(99));
    let booted = BootedKernel::<2, 8, 8, 16, 4, 4, 4, 0, 4, 4>::boot(config)
        .expect("boot flow should fit fixed stores");

    let events = booted.kernel().events();
    assert_eq!(events.len(), 5);
    assert_eq!(events[0].kind, EventKind::AgentRegistered);
    assert_eq!(events[1].kind, EventKind::CapabilityGranted);
    assert_eq!(events[2].kind, EventKind::Observation);
    assert_eq!(events[3].kind, EventKind::ActionExecuted);
    assert_eq!(events[4].kind, EventKind::VerificationRequested);
    assert_eq!(events[3].action, Some(ActionId::new(99)));
    assert_eq!(events[4].action, Some(ActionId::new(99)));
}
