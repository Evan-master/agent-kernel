use core::mem::{align_of, size_of};

use agent_kernel_x86_64::{
    native_runtime::NativeRunBoundary,
    per_cpu::{
        CpuTransitionError, CpuTransitionStorage, PER_CPU_CALL_COUNT_OFFSET,
        PER_CPU_CALL_CR3_OFFSET, PER_CPU_CALL_RIP_OFFSET, PER_CPU_CALL_RSP_OFFSET,
        PER_CPU_CALL_SEEN_OFFSET, PER_CPU_FAULT_ADDRESS_OFFSET, PER_CPU_FAULT_COUNT_OFFSET,
        PER_CPU_FAULT_CR3_OFFSET, PER_CPU_FAULT_ERROR_CODE_OFFSET, PER_CPU_FAULT_RIP_OFFSET,
        PER_CPU_FAULT_RSP_OFFSET, PER_CPU_FAULT_SEEN_OFFSET, PER_CPU_FAULT_VECTOR_OFFSET,
        PER_CPU_HOST_RSP_OFFSET, PER_CPU_INTERRUPT_CR3_OFFSET, PER_CPU_INTERRUPT_RIP_OFFSET,
        PER_CPU_INTERRUPT_RSP_OFFSET, PER_CPU_IRQ_COUNT_OFFSET, PER_CPU_IRQ_SEEN_OFFSET,
        PER_CPU_KERNEL_CR3_OFFSET, PER_CPU_PREEMPTED_OFFSET,
    },
};

unsafe fn write_u64(storage: &CpuTransitionStorage, offset: usize, value: u64) {
    // SAFETY: contract offsets name aligned u64 fields in this exclusively
    // owned test slot.
    unsafe {
        storage
            .as_ptr()
            .cast::<u8>()
            .add(offset)
            .cast::<u64>()
            .write_volatile(value);
    }
}

unsafe fn write_u8(storage: &CpuTransitionStorage, offset: usize, value: u8) {
    // SAFETY: contract offsets name u8 fields in this exclusively owned slot.
    unsafe {
        storage
            .as_ptr()
            .cast::<u8>()
            .add(offset)
            .write_volatile(value);
    }
}

#[test]
fn per_cpu_transition_layout_is_stable_for_gs_assembly() {
    assert_eq!(align_of::<CpuTransitionStorage>(), 64);
    assert_eq!(size_of::<CpuTransitionStorage>(), 128);
    assert_eq!(PER_CPU_HOST_RSP_OFFSET, 0);
    assert_eq!(PER_CPU_KERNEL_CR3_OFFSET, 8);
    assert_eq!(PER_CPU_INTERRUPT_RSP_OFFSET, 16);
    assert_eq!(PER_CPU_INTERRUPT_RIP_OFFSET, 24);
    assert_eq!(PER_CPU_INTERRUPT_CR3_OFFSET, 32);
    assert_eq!(PER_CPU_CALL_RSP_OFFSET, 40);
    assert_eq!(PER_CPU_CALL_RIP_OFFSET, 48);
    assert_eq!(PER_CPU_CALL_CR3_OFFSET, 56);
    assert_eq!(PER_CPU_FAULT_RSP_OFFSET, 64);
    assert_eq!(PER_CPU_FAULT_RIP_OFFSET, 72);
    assert_eq!(PER_CPU_FAULT_CR3_OFFSET, 80);
    assert_eq!(PER_CPU_FAULT_ERROR_CODE_OFFSET, 88);
    assert_eq!(PER_CPU_FAULT_ADDRESS_OFFSET, 96);
    assert_eq!(PER_CPU_IRQ_COUNT_OFFSET, 104);
    assert_eq!(PER_CPU_IRQ_SEEN_OFFSET, 105);
    assert_eq!(PER_CPU_PREEMPTED_OFFSET, 106);
    assert_eq!(PER_CPU_CALL_COUNT_OFFSET, 107);
    assert_eq!(PER_CPU_CALL_SEEN_OFFSET, 108);
    assert_eq!(PER_CPU_FAULT_COUNT_OFFSET, 109);
    assert_eq!(PER_CPU_FAULT_SEEN_OFFSET, 110);
    assert_eq!(PER_CPU_FAULT_VECTOR_OFFSET, 111);
}

#[test]
fn install_and_dispatch_reset_preserve_only_kernel_identity() {
    let storage = CpuTransitionStorage::new();
    assert_eq!(storage.install(0x4003), Ok(()));
    assert_eq!(storage.kernel_cr3(), 0x4003);
    assert_eq!(
        storage.install(0x4003),
        Err(CpuTransitionError::AlreadyInstalled)
    );

    unsafe {
        write_u64(&storage, PER_CPU_HOST_RSP_OFFSET, 0x9000);
        write_u64(&storage, PER_CPU_CALL_RSP_OFFSET, 0xa000);
        write_u8(&storage, PER_CPU_CALL_COUNT_OFFSET, 1);
        write_u8(&storage, PER_CPU_CALL_SEEN_OFFSET, 1);
    }
    assert_eq!(storage.run_boundary(), Some(NativeRunBoundary::AgentCall));
    assert_eq!(storage.begin_dispatch(0x4003), Ok(()));
    assert_eq!(storage.kernel_cr3(), 0x4003);
    assert_eq!(storage.host_rsp(), 0);
    assert_eq!(storage.call_rsp(), 0);
    assert_eq!(storage.run_boundary(), None);
    assert_eq!(
        storage.begin_dispatch(0x5003),
        Err(CpuTransitionError::KernelCr3Mismatch)
    );
}

#[test]
fn distinct_cpu_slots_never_share_transition_evidence() {
    let cpu0 = CpuTransitionStorage::new();
    let cpu1 = CpuTransitionStorage::new();
    cpu0.install(0x4003).unwrap();
    cpu1.install(0x4003).unwrap();
    cpu0.begin_dispatch(0x4003).unwrap();
    cpu1.begin_dispatch(0x4003).unwrap();

    unsafe {
        write_u64(&cpu1, PER_CPU_INTERRUPT_RSP_OFFSET, 0x8000);
        write_u64(&cpu1, PER_CPU_INTERRUPT_RIP_OFFSET, 0x4100);
        write_u64(&cpu1, PER_CPU_INTERRUPT_CR3_OFFSET, 0x9003);
        write_u8(&cpu1, PER_CPU_IRQ_COUNT_OFFSET, 1);
        write_u8(&cpu1, PER_CPU_IRQ_SEEN_OFFSET, 1);
        write_u8(&cpu1, PER_CPU_PREEMPTED_OFFSET, 1);
    }
    assert_eq!(cpu0.run_boundary(), None);
    assert_eq!(cpu1.run_boundary(), Some(NativeRunBoundary::QuantumExpired));
    assert_eq!(cpu1.interrupt_rsp(), 0x8000);
    assert_eq!(cpu1.interrupt_rip(), 0x4100);
    assert_eq!(cpu1.interrupt_cr3(), 0x9003);
}

#[test]
fn call_and_fault_mailboxes_classify_exactly_one_boundary() {
    let storage = CpuTransitionStorage::new();
    storage.install(0x4003).unwrap();
    storage.begin_dispatch(0x4003).unwrap();
    unsafe {
        write_u64(&storage, PER_CPU_CALL_RSP_OFFSET, 0x8100);
        write_u64(&storage, PER_CPU_CALL_RIP_OFFSET, 0x4200);
        write_u64(&storage, PER_CPU_CALL_CR3_OFFSET, 0x9003);
        write_u8(&storage, PER_CPU_CALL_COUNT_OFFSET, 1);
        write_u8(&storage, PER_CPU_CALL_SEEN_OFFSET, 1);
    }
    assert_eq!(storage.run_boundary(), Some(NativeRunBoundary::AgentCall));
    assert_eq!(storage.call_rip(), 0x4200);
    assert_eq!(storage.call_cr3(), 0x9003);

    storage.begin_dispatch(0x4003).unwrap();
    unsafe {
        write_u64(&storage, PER_CPU_FAULT_RSP_OFFSET, 0x8200);
        write_u64(&storage, PER_CPU_FAULT_RIP_OFFSET, 0x4300);
        write_u64(&storage, PER_CPU_FAULT_CR3_OFFSET, 0x9003);
        write_u64(&storage, PER_CPU_FAULT_ERROR_CODE_OFFSET, 5);
        write_u64(&storage, PER_CPU_FAULT_ADDRESS_OFFSET, 0xdead_0000);
        write_u8(&storage, PER_CPU_FAULT_COUNT_OFFSET, 1);
        write_u8(&storage, PER_CPU_FAULT_SEEN_OFFSET, 1);
        write_u8(&storage, PER_CPU_FAULT_VECTOR_OFFSET, 14);
    }
    assert!(matches!(
        storage.run_boundary(),
        Some(NativeRunBoundary::AgentFault(_))
    ));
    assert_eq!(storage.fault_rsp(), 0x8200);
    assert_eq!(storage.fault_rip(), 0x4300);
    assert_eq!(storage.fault_cr3(), 0x9003);
    assert_eq!(storage.fault_error_code(), 5);
    assert_eq!(storage.fault_address(), 0xdead_0000);
}
