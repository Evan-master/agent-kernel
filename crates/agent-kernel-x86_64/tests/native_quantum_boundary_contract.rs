use agent_kernel_x86_64::native_runtime::{
    NativeRunBoundary, NativeRunBoundaryError, NativeRunBoundaryEvidence,
};

#[test]
fn one_agent_call_is_the_only_valid_call_boundary() {
    let evidence = NativeRunBoundaryEvidence::new(1, 0, true, false, false);

    assert_eq!(evidence.classify(), Ok(NativeRunBoundary::AgentCall));
}

#[test]
fn one_timer_irq_is_the_only_valid_quantum_boundary() {
    let evidence = NativeRunBoundaryEvidence::new(0, 1, false, true, true);

    assert_eq!(evidence.classify(), Ok(NativeRunBoundary::QuantumExpired));
}

#[test]
fn empty_mixed_repeated_and_inconsistent_evidence_is_rejected() {
    let invalid = [
        NativeRunBoundaryEvidence::new(0, 0, false, false, false),
        NativeRunBoundaryEvidence::new(1, 1, true, true, true),
        NativeRunBoundaryEvidence::new(2, 0, true, false, false),
        NativeRunBoundaryEvidence::new(0, 2, false, true, true),
        NativeRunBoundaryEvidence::new(1, 0, false, false, false),
        NativeRunBoundaryEvidence::new(0, 1, false, false, true),
        NativeRunBoundaryEvidence::new(0, 1, false, true, false),
    ];

    for evidence in invalid {
        assert_eq!(
            evidence.classify(),
            Err(NativeRunBoundaryError::InvalidEvidence)
        );
    }
}
