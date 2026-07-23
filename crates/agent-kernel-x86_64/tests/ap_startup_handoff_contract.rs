use agent_kernel_x86_64::cpu::{
    ApStartupDescriptor, ApStartupEvidence, ApStartupHandoff, ApStartupHandoffError,
    ApStartupStatus, ApicId, CpuIndex, AP_HANDOFF_APIC_ID_OFFSET, AP_HANDOFF_CPU_INDEX_OFFSET,
    AP_HANDOFF_CR3_OFFSET, AP_HANDOFF_ENTRY_OFFSET, AP_HANDOFF_GENERATION_OFFSET,
    AP_HANDOFF_LOCAL_APIC_BASE_OFFSET, AP_HANDOFF_PHYSICAL_OFFSET_OFFSET,
    AP_HANDOFF_STACK_TOP_OFFSET, AP_HANDOFF_STATUS_OFFSET,
};

const PHYSICAL_OFFSET: u64 = 0xffff_8000_0000_0000;

fn descriptor(cpu: u16, generation: u64) -> ApStartupDescriptor {
    ApStartupDescriptor::new(
        CpuIndex::new(cpu).unwrap(),
        ApicId::new(cpu as u32 + 4),
        generation,
        0x20_0000,
        0xffff_ffff_8100_0000,
        0xffff_ffff_8000_1000,
        0xfee0_0000,
        PHYSICAL_OFFSET,
    )
    .unwrap()
}

#[test]
fn fixed_layout_matches_the_trampoline_abi() {
    assert_eq!(AP_HANDOFF_STATUS_OFFSET, 8);
    assert_eq!(AP_HANDOFF_CPU_INDEX_OFFSET, 12);
    assert_eq!(AP_HANDOFF_APIC_ID_OFFSET, 16);
    assert_eq!(AP_HANDOFF_GENERATION_OFFSET, 24);
    assert_eq!(AP_HANDOFF_CR3_OFFSET, 32);
    assert_eq!(AP_HANDOFF_STACK_TOP_OFFSET, 40);
    assert_eq!(AP_HANDOFF_ENTRY_OFFSET, 48);
    assert_eq!(AP_HANDOFF_LOCAL_APIC_BASE_OFFSET, 56);
    assert_eq!(AP_HANDOFF_PHYSICAL_OFFSET_OFFSET, 64);
    assert_eq!(core::mem::align_of::<ApStartupHandoff>(), 64);
    assert_eq!(core::mem::size_of::<ApStartupHandoff>(), 128);
}

#[test]
fn descriptors_reject_unsafe_machine_state() {
    let cpu = CpuIndex::new(1).unwrap();
    let apic = ApicId::new(5);
    assert_eq!(
        ApStartupDescriptor::new(
            CpuIndex::BSP,
            apic,
            1,
            0x20_0000,
            0xffff_ffff_8100_0000,
            0xffff_ffff_8000_1000,
            0xfee0_0000,
            PHYSICAL_OFFSET,
        ),
        Err(ApStartupHandoffError::InvalidCpu)
    );
    assert_eq!(
        ApStartupDescriptor::new(
            cpu,
            apic,
            0,
            0x20_0000,
            0xffff_ffff_8100_0000,
            0xffff_ffff_8000_1000,
            0xfee0_0000,
            PHYSICAL_OFFSET,
        ),
        Err(ApStartupHandoffError::InvalidGeneration)
    );
    assert_eq!(
        ApStartupDescriptor::new(
            cpu,
            apic,
            1,
            0x1_0000_0000,
            0xffff_ffff_8100_0000,
            0xffff_ffff_8000_1000,
            0xfee0_0000,
            PHYSICAL_OFFSET,
        ),
        Err(ApStartupHandoffError::InvalidCr3)
    );
}

#[test]
fn release_publishes_one_exact_startup_request() {
    let handoff = ApStartupHandoff::new();
    let expected = descriptor(1, 7);
    handoff.prepare(expected).unwrap();
    assert_eq!(handoff.status(), Ok(ApStartupStatus::Prepared));
    assert_eq!(handoff.descriptor(), Ok(expected));
    assert_eq!(
        handoff.prepare(descriptor(2, 8)),
        Err(ApStartupHandoffError::InvalidState)
    );
}

#[test]
fn online_acknowledgement_requires_exact_identity_and_generation() {
    let handoff = ApStartupHandoff::new();
    handoff.prepare(descriptor(1, 7)).unwrap();
    let wrong = ApStartupEvidence {
        cpu: CpuIndex::new(2).unwrap(),
        apic_id: ApicId::new(5),
        generation: 7,
        privileged_stack_start: 0x1000,
        privileged_stack_end: 0x2000,
        transition_slot: 0x3000,
    };
    assert_eq!(
        handoff.acknowledge_online(wrong),
        Err(ApStartupHandoffError::IdentityMismatch)
    );
    assert_eq!(handoff.status(), Ok(ApStartupStatus::Prepared));

    let evidence = ApStartupEvidence {
        cpu: CpuIndex::new(1).unwrap(),
        apic_id: ApicId::new(5),
        generation: 7,
        privileged_stack_start: 0xffff_ffff_8200_0000,
        privileged_stack_end: 0xffff_ffff_8200_8000,
        transition_slot: 0xffff_ffff_8300_0000,
    };
    handoff.acknowledge_online(evidence).unwrap();
    assert_eq!(handoff.status(), Ok(ApStartupStatus::Online));
    assert_eq!(handoff.evidence(), Ok(evidence));
}

#[test]
fn terminal_state_must_be_observed_before_handoff_reuse() {
    let handoff = ApStartupHandoff::new();
    handoff.prepare(descriptor(1, 7)).unwrap();
    assert_eq!(
        handoff.reset_terminal(),
        Err(ApStartupHandoffError::InvalidState)
    );
    handoff.fail(CpuIndex::new(1).unwrap(), 7).unwrap();
    assert_eq!(handoff.status(), Ok(ApStartupStatus::Failed));
    handoff.reset_terminal().unwrap();
    handoff.prepare(descriptor(2, 8)).unwrap();
    assert_eq!(handoff.descriptor(), Ok(descriptor(2, 8)));
}
