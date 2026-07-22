use agent_kernel_x86_64::{
    apic::{
        ApicBaseMsr, ApicVector, CpuidApicIdentity, IcrCommand, IcrError, IoApicPolarity,
        IoApicRedirectionEntry, IoApicRedirectionIndex, IoApicTrigger, IoApicVersion,
        LocalApicBase, LocalApicRegister, StartupVector, APIC_RESCHEDULE_VECTOR,
        APIC_SPURIOUS_VECTOR, APIC_STARTUP_ERROR_VECTOR, APIC_TIMER_VECTOR,
        APIC_TLB_SHOOTDOWN_VECTOR,
    },
    cpu::ApicId,
};

#[test]
fn apic_vectors_are_valid_and_pairwise_distinct() {
    let vectors = [
        APIC_TIMER_VECTOR,
        APIC_RESCHEDULE_VECTOR,
        APIC_TLB_SHOOTDOWN_VECTOR,
        APIC_STARTUP_ERROR_VECTOR,
        APIC_SPURIOUS_VECTOR,
    ];
    for (index, vector) in vectors.iter().enumerate() {
        assert!(vector.get() >= 32);
        assert!(!vectors[..index].contains(vector));
    }
    assert_eq!(APIC_TIMER_VECTOR.get(), 0xe0);
    assert_eq!(APIC_TLB_SHOOTDOWN_VECTOR.get(), 0xe2);
    assert_eq!(APIC_SPURIOUS_VECTOR.get(), 0xff);
    assert_eq!(ApicVector::new(31), None);
    assert_eq!(ApicVector::new(32).unwrap().get(), 32);
}

#[test]
fn startup_vector_accepts_only_low_aligned_trampoline_pages() {
    let vector = StartupVector::from_trampoline_address(0x8000).unwrap();
    assert_eq!(vector.get(), 8);
    assert_eq!(vector.address(), 0x8000);
    assert!(StartupVector::from_trampoline_address(0).is_none());
    assert!(StartupVector::from_trampoline_address(0x8100).is_none());
    assert!(StartupVector::from_trampoline_address(0x10_0000).is_none());
}

#[test]
fn xapic_icr_encodes_init_sipi_and_fixed_ipi_commands() {
    let destination = ApicId::new(3);
    let init_assert = IcrCommand::init_assert(destination).unwrap();
    assert_eq!(init_assert.high(), 3 << 24);
    assert_eq!(init_assert.low(), (0b101 << 8) | (1 << 14) | (1 << 15));

    let init_deassert = IcrCommand::init_deassert(destination).unwrap();
    assert_eq!(init_deassert.high(), 3 << 24);
    assert_eq!(init_deassert.low(), (0b101 << 8) | (1 << 15));

    let sipi = IcrCommand::startup(
        destination,
        StartupVector::from_trampoline_address(0x8000).unwrap(),
    )
    .unwrap();
    assert_eq!(sipi.high(), 3 << 24);
    assert_eq!(sipi.low(), (0b110 << 8) | 8);

    let tlb = IcrCommand::fixed(destination, APIC_TLB_SHOOTDOWN_VECTOR).unwrap();
    assert_eq!(tlb.high(), 3 << 24);
    assert_eq!(tlb.low(), 0xe2);
    assert!(!IcrCommand::delivery_pending(tlb.low()));
    assert!(IcrCommand::delivery_pending(tlb.low() | (1 << 12)));

    assert_eq!(
        IcrCommand::fixed(ApicId::new(0x1ff), APIC_TLB_SHOOTDOWN_VECTOR),
        Err(IcrError::DestinationRequiresX2Apic(ApicId::new(0x1ff)))
    );
}

#[test]
fn io_apic_redirection_entry_has_canonical_fixed_delivery_bits() {
    let entry = IoApicRedirectionEntry::fixed(
        APIC_TIMER_VECTOR,
        7,
        IoApicPolarity::ActiveLow,
        IoApicTrigger::Level,
        false,
    );
    let expected = (7u64 << 56) | (1 << 13) | (1 << 15) | 0xe0;
    assert_eq!(entry.raw(), expected);
    assert_eq!(entry.low(), expected as u32);
    assert_eq!(entry.high(), (expected >> 32) as u32);
    assert_eq!(entry.vector(), APIC_TIMER_VECTOR);
    assert_eq!(entry.destination(), 7);
    assert_eq!(entry.polarity(), IoApicPolarity::ActiveLow);
    assert_eq!(entry.trigger(), IoApicTrigger::Level);
    assert!(!entry.masked());
    assert_eq!(IoApicRedirectionEntry::from_raw(expected), Some(entry));

    let masked = entry.with_masked(true);
    assert!(masked.masked());
    assert_eq!(masked.raw(), expected | (1 << 16));
    assert!(IoApicRedirectionEntry::from_raw(expected | (1 << 11)).is_none());
}

#[test]
fn io_apic_version_bounds_redirection_register_pairs() {
    let version = IoApicVersion::from_raw(0x11 | (23 << 16));
    assert_eq!(version.version(), 0x11);
    assert_eq!(version.redirection_count(), 24);

    let first = IoApicRedirectionIndex::new(0, version).unwrap();
    assert_eq!(first.low_register(), 0x10);
    assert_eq!(first.high_register(), 0x11);
    let last = IoApicRedirectionIndex::new(23, version).unwrap();
    assert_eq!(last.low_register(), 0x3e);
    assert_eq!(last.high_register(), 0x3f);
    assert!(IoApicRedirectionIndex::new(24, version).is_none());
}

#[test]
fn local_apic_base_and_register_offsets_are_checked() {
    let base = LocalApicBase::new(0xfee0_0000).unwrap();
    assert_eq!(base.physical(), 0xfee0_0000);
    assert_eq!(
        base.virtual_address(0xffff_8000_0000_0000),
        Some(0xffff_8000_fee0_0000)
    );
    assert!(LocalApicBase::new(0).is_none());
    assert!(LocalApicBase::new(0xfee0_0001).is_none());

    assert_eq!(LocalApicRegister::Id.offset(), 0x20);
    assert_eq!(LocalApicRegister::EndOfInterrupt.offset(), 0xb0);
    assert_eq!(LocalApicRegister::Spurious.offset(), 0xf0);
    assert_eq!(LocalApicRegister::InterruptCommandLow.offset(), 0x300);
    assert_eq!(LocalApicRegister::InterruptCommandHigh.offset(), 0x310);
}

#[test]
fn cpuid_and_apic_base_msr_bind_the_boot_processor() {
    let cpuid = CpuidApicIdentity::from_leaf1(3 << 24, 1 << 9).unwrap();
    assert_eq!(cpuid.initial_apic_id(), ApicId::new(3));
    assert!(CpuidApicIdentity::from_leaf1(3 << 24, 0).is_none());

    let raw = 0xfee0_0000 | (1 << 11) | (1 << 10) | (1 << 8);
    let msr = ApicBaseMsr::from_raw(raw).unwrap();
    assert_eq!(msr.base(), LocalApicBase::new(0xfee0_0000).unwrap());
    assert!(msr.enabled());
    assert!(msr.x2apic_enabled());
    assert!(msr.boot_processor());
    assert!(ApicBaseMsr::from_raw(0xfee0_0000 | (1 << 8)).is_none());
    assert!(ApicBaseMsr::from_raw(0xfee0_0001 | (1 << 11)).is_none());
}
