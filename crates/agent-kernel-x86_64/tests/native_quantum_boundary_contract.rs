use agent_kernel_x86_64::native_runtime::{
    NativeAgentFault, NativeRunBoundary, NativeRunBoundaryError, NativeRunBoundaryEvidence,
    PAGE_FAULT_VECTOR,
};
use agent_kernel_x86_64::user_memory::UserMemoryLayout;

#[test]
fn one_agent_call_is_the_only_valid_call_boundary() {
    let evidence = NativeRunBoundaryEvidence::new(1, 0, 0, true, false, false, false, 0, 0, 0);

    assert_eq!(evidence.classify(), Ok(NativeRunBoundary::AgentCall));
}

#[test]
fn one_timer_irq_is_the_only_valid_quantum_boundary() {
    let evidence = NativeRunBoundaryEvidence::new(0, 1, 0, false, true, true, false, 0, 0, 0);

    assert_eq!(evidence.classify(), Ok(NativeRunBoundary::QuantumExpired));
}

#[test]
fn one_invalid_opcode_is_the_only_valid_agent_fault_boundary() {
    let evidence = NativeRunBoundaryEvidence::new(0, 0, 1, false, false, false, true, 6, 0, 0);

    assert_eq!(
        evidence.classify(),
        Ok(NativeRunBoundary::AgentFault(
            NativeAgentFault::InvalidOpcode
        ))
    );
}

#[test]
fn general_protection_preserves_its_cpu_error_code_and_fault_detail() {
    let zero = NativeRunBoundaryEvidence::new(0, 0, 1, false, false, false, true, 13, 0, 0);
    let selector =
        NativeRunBoundaryEvidence::new(0, 0, 1, false, false, false, true, 13, 0x1234, 0);

    assert_eq!(
        zero.classify(),
        Ok(NativeRunBoundary::AgentFault(
            NativeAgentFault::GeneralProtection { error_code: 0 }
        ))
    );
    let fault = NativeAgentFault::GeneralProtection { error_code: 0x1234 };
    assert_eq!(
        selector.classify(),
        Ok(NativeRunBoundary::AgentFault(fault))
    );
    assert_eq!(fault.vector(), 13);
    assert_eq!(fault.error_code(), 0x1234);
    assert_eq!(fault.fault_address(), None);
    assert_eq!(fault.detail(), 13 | (0x1234 << 8));
    assert_eq!(NativeAgentFault::InvalidOpcode.detail(), 6);
}

#[test]
fn page_fault_preserves_error_address_and_packed_semantic_detail() {
    let address = UserMemoryLayout::fixed().signal_start();
    let fault = NativeAgentFault::PageFault {
        error_code: 7,
        address,
    };
    let evidence = NativeRunBoundaryEvidence::new(
        0,
        0,
        1,
        false,
        false,
        false,
        true,
        PAGE_FAULT_VECTOR,
        7,
        address,
    );

    assert_eq!(
        evidence.classify(),
        Ok(NativeRunBoundary::AgentFault(fault))
    );
    assert_eq!(fault.vector(), 14);
    assert_eq!(fault.error_code(), 7);
    assert_eq!(fault.fault_address(), Some(address));
    assert_eq!(fault.detail(), 0xe007_4000_0002_0000);
    assert_eq!(NativeAgentFault::InvalidOpcode.fault_address(), None);
}

#[test]
fn not_present_lazy_write_has_distinct_error_address_and_detail() {
    let address = UserMemoryLayout::fixed().lazy_data_start();
    let fault = NativeAgentFault::PageFault {
        error_code: 6,
        address,
    };
    let evidence = NativeRunBoundaryEvidence::new(
        0,
        0,
        1,
        false,
        false,
        false,
        true,
        PAGE_FAULT_VECTOR,
        6,
        address,
    );

    assert_eq!(
        evidence.classify(),
        Ok(NativeRunBoundary::AgentFault(fault))
    );
    assert_eq!(fault.detail(), 0xe006_4000_0002_6000);
    assert_ne!(fault.detail(), 0xe007_4000_0002_0000);
}

#[test]
fn empty_mixed_repeated_inconsistent_and_unsupported_evidence_is_rejected() {
    let invalid = [
        NativeRunBoundaryEvidence::new(0, 0, 0, false, false, false, false, 0, 0, 0),
        NativeRunBoundaryEvidence::new(1, 1, 0, true, true, true, false, 0, 0, 0),
        NativeRunBoundaryEvidence::new(1, 0, 1, true, false, false, true, 6, 0, 0),
        NativeRunBoundaryEvidence::new(0, 1, 1, false, true, true, true, 6, 0, 0),
        NativeRunBoundaryEvidence::new(2, 0, 0, true, false, false, false, 0, 0, 0),
        NativeRunBoundaryEvidence::new(0, 2, 0, false, true, true, false, 0, 0, 0),
        NativeRunBoundaryEvidence::new(0, 0, 2, false, false, false, true, 6, 0, 0),
        NativeRunBoundaryEvidence::new(1, 0, 0, false, false, false, false, 0, 0, 0),
        NativeRunBoundaryEvidence::new(0, 1, 0, false, false, true, false, 0, 0, 0),
        NativeRunBoundaryEvidence::new(0, 1, 0, false, true, false, false, 0, 0, 0),
        NativeRunBoundaryEvidence::new(0, 0, 1, false, false, false, false, 6, 0, 0),
        NativeRunBoundaryEvidence::new(0, 0, 1, false, false, false, true, 0, 0, 0),
        NativeRunBoundaryEvidence::new(0, 0, 1, false, false, false, true, 7, 0, 0),
        NativeRunBoundaryEvidence::new(0, 0, 1, false, false, false, true, 6, 1, 0),
        NativeRunBoundaryEvidence::new(0, 0, 1, false, false, false, true, 6, 0, 1),
        NativeRunBoundaryEvidence::new(0, 0, 1, false, false, false, true, 13, 0, 1),
        NativeRunBoundaryEvidence::new(
            0,
            0,
            1,
            false,
            false,
            false,
            true,
            13,
            u64::from(u32::MAX) + 1,
            0,
        ),
        NativeRunBoundaryEvidence::new(1, 0, 0, true, false, false, false, 0, 0, 1),
        NativeRunBoundaryEvidence::new(0, 1, 0, false, true, true, false, 0, 0, 1),
        NativeRunBoundaryEvidence::new(0, 0, 1, false, false, false, true, 14, 0x1000, 0),
        NativeRunBoundaryEvidence::new(
            0,
            0,
            1,
            false,
            false,
            false,
            true,
            14,
            7,
            0x0000_8000_0000_0000,
        ),
    ];

    for evidence in invalid {
        assert_eq!(
            evidence.classify(),
            Err(NativeRunBoundaryError::InvalidEvidence)
        );
    }
}
