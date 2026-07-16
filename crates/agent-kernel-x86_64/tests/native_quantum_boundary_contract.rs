use agent_kernel_x86_64::native_runtime::{
    NativeAgentFault, NativeRunBoundary, NativeRunBoundaryError, NativeRunBoundaryEvidence,
};

#[test]
fn one_agent_call_is_the_only_valid_call_boundary() {
    let evidence = NativeRunBoundaryEvidence::new(1, 0, 0, true, false, false, false, 0);

    assert_eq!(evidence.classify(), Ok(NativeRunBoundary::AgentCall));
}

#[test]
fn one_timer_irq_is_the_only_valid_quantum_boundary() {
    let evidence = NativeRunBoundaryEvidence::new(0, 1, 0, false, true, true, false, 0);

    assert_eq!(evidence.classify(), Ok(NativeRunBoundary::QuantumExpired));
}

#[test]
fn one_invalid_opcode_is_the_only_valid_agent_fault_boundary() {
    let evidence = NativeRunBoundaryEvidence::new(0, 0, 1, false, false, false, true, 6);

    assert_eq!(
        evidence.classify(),
        Ok(NativeRunBoundary::AgentFault(
            NativeAgentFault::InvalidOpcode
        ))
    );
}

#[test]
fn empty_mixed_repeated_inconsistent_and_unsupported_evidence_is_rejected() {
    let invalid = [
        NativeRunBoundaryEvidence::new(0, 0, 0, false, false, false, false, 0),
        NativeRunBoundaryEvidence::new(1, 1, 0, true, true, true, false, 0),
        NativeRunBoundaryEvidence::new(1, 0, 1, true, false, false, true, 6),
        NativeRunBoundaryEvidence::new(0, 1, 1, false, true, true, true, 6),
        NativeRunBoundaryEvidence::new(2, 0, 0, true, false, false, false, 0),
        NativeRunBoundaryEvidence::new(0, 2, 0, false, true, true, false, 0),
        NativeRunBoundaryEvidence::new(0, 0, 2, false, false, false, true, 6),
        NativeRunBoundaryEvidence::new(1, 0, 0, false, false, false, false, 0),
        NativeRunBoundaryEvidence::new(0, 1, 0, false, false, true, false, 0),
        NativeRunBoundaryEvidence::new(0, 1, 0, false, true, false, false, 0),
        NativeRunBoundaryEvidence::new(0, 0, 1, false, false, false, false, 6),
        NativeRunBoundaryEvidence::new(0, 0, 1, false, false, false, true, 0),
        NativeRunBoundaryEvidence::new(0, 0, 1, false, false, false, true, 13),
    ];

    for evidence in invalid {
        assert_eq!(
            evidence.classify(),
            Err(NativeRunBoundaryError::InvalidEvidence)
        );
    }
}
