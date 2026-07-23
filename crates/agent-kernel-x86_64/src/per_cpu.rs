//! CPU-local Ring 3 transition evidence with a stable assembly layout.
//!
//! The x86_64 architecture layer owns one slot per logical CPU. Assembly uses
//! GS-relative byte offsets while Rust reads the owning CPU's slot only after
//! returning to CPL0 with interrupts clear. Cross-CPU mutation is forbidden.

use core::{cell::UnsafeCell, mem::offset_of};

use crate::cpu::CpuIndex;
use crate::native_runtime::{NativeRunBoundary, NativeRunBoundaryEvidence};

#[repr(transparent)]
struct CpuLocalU64(UnsafeCell<u64>);

impl CpuLocalU64 {
    const fn new() -> Self {
        Self(UnsafeCell::new(0))
    }

    fn load(&self) -> u64 {
        // SAFETY: only the CPU owning the enclosing slot accesses this field.
        unsafe { self.0.get().read_volatile() }
    }

    fn store(&self, value: u64) {
        // SAFETY: only the CPU owning the enclosing slot accesses this field.
        unsafe { self.0.get().write_volatile(value) };
    }

    fn pointer(&self) -> *mut u64 {
        self.0.get()
    }
}

#[repr(transparent)]
struct CpuLocalU8(UnsafeCell<u8>);

impl CpuLocalU8 {
    const fn new() -> Self {
        Self(UnsafeCell::new(0))
    }

    fn load(&self) -> u8 {
        // SAFETY: only the CPU owning the enclosing slot accesses this field.
        unsafe { self.0.get().read_volatile() }
    }

    fn store(&self, value: u8) {
        // SAFETY: only the CPU owning the enclosing slot accesses this field.
        unsafe { self.0.get().write_volatile(value) };
    }
}

#[repr(transparent)]
struct CpuLocalU16(UnsafeCell<u16>);

impl CpuLocalU16 {
    const fn new() -> Self {
        Self(UnsafeCell::new(0))
    }

    fn load(&self) -> u16 {
        // SAFETY: only the CPU owning the enclosing slot accesses this field.
        unsafe { self.0.get().read_volatile() }
    }

    fn store(&self, value: u16) {
        // SAFETY: installation publishes this immutable CPU identity once.
        unsafe { self.0.get().write_volatile(value) };
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CpuTransitionError {
    AlreadyInstalled,
    NotInstalled,
    KernelCr3Mismatch,
}

#[repr(C, align(64))]
pub struct CpuTransitionStorage {
    host_rsp: CpuLocalU64,
    kernel_cr3: CpuLocalU64,
    interrupt_rsp: CpuLocalU64,
    interrupt_rip: CpuLocalU64,
    interrupt_cr3: CpuLocalU64,
    call_rsp: CpuLocalU64,
    call_rip: CpuLocalU64,
    call_cr3: CpuLocalU64,
    fault_rsp: CpuLocalU64,
    fault_rip: CpuLocalU64,
    fault_cr3: CpuLocalU64,
    fault_error_code: CpuLocalU64,
    fault_address: CpuLocalU64,
    irq_count: CpuLocalU8,
    irq_seen: CpuLocalU8,
    preempted: CpuLocalU8,
    call_count: CpuLocalU8,
    call_seen: CpuLocalU8,
    fault_count: CpuLocalU8,
    fault_seen: CpuLocalU8,
    fault_vector: CpuLocalU8,
    cpu_index: CpuLocalU16,
}

impl CpuTransitionStorage {
    pub const fn new() -> Self {
        Self {
            host_rsp: CpuLocalU64::new(),
            kernel_cr3: CpuLocalU64::new(),
            interrupt_rsp: CpuLocalU64::new(),
            interrupt_rip: CpuLocalU64::new(),
            interrupt_cr3: CpuLocalU64::new(),
            call_rsp: CpuLocalU64::new(),
            call_rip: CpuLocalU64::new(),
            call_cr3: CpuLocalU64::new(),
            fault_rsp: CpuLocalU64::new(),
            fault_rip: CpuLocalU64::new(),
            fault_cr3: CpuLocalU64::new(),
            fault_error_code: CpuLocalU64::new(),
            fault_address: CpuLocalU64::new(),
            irq_count: CpuLocalU8::new(),
            irq_seen: CpuLocalU8::new(),
            preempted: CpuLocalU8::new(),
            call_count: CpuLocalU8::new(),
            call_seen: CpuLocalU8::new(),
            fault_count: CpuLocalU8::new(),
            fault_seen: CpuLocalU8::new(),
            fault_vector: CpuLocalU8::new(),
            cpu_index: CpuLocalU16::new(),
        }
    }

    pub fn install(&self, kernel_cr3: u64) -> Result<(), CpuTransitionError> {
        self.install_for_cpu(kernel_cr3, CpuIndex::BSP)
    }

    pub fn install_for_cpu(
        &self,
        kernel_cr3: u64,
        cpu: CpuIndex,
    ) -> Result<(), CpuTransitionError> {
        if self.kernel_cr3.load() != 0 {
            return Err(CpuTransitionError::AlreadyInstalled);
        }
        self.reset_evidence();
        self.cpu_index.store(cpu.get());
        self.kernel_cr3.store(kernel_cr3);
        Ok(())
    }

    pub fn begin_dispatch(&self, kernel_cr3: u64) -> Result<(), CpuTransitionError> {
        let installed = self.kernel_cr3.load();
        if installed == 0 {
            return Err(CpuTransitionError::NotInstalled);
        }
        if installed != kernel_cr3 {
            return Err(CpuTransitionError::KernelCr3Mismatch);
        }
        self.reset_evidence();
        Ok(())
    }

    pub fn run_boundary(&self) -> Option<NativeRunBoundary> {
        NativeRunBoundaryEvidence::new(
            self.call_count.load(),
            self.irq_count.load(),
            self.fault_count.load(),
            self.call_seen.load() == 1,
            self.irq_seen.load() == 1,
            self.preempted.load() == 1,
            self.fault_seen.load() == 1,
            self.fault_vector.load(),
            self.fault_error_code.load(),
            self.fault_address.load(),
        )
        .classify()
        .ok()
    }

    pub const fn as_ptr(&self) -> *mut Self {
        self as *const Self as *mut Self
    }

    pub fn host_rsp_pointer(&self) -> *mut u64 {
        self.host_rsp.pointer()
    }

    pub fn host_rsp(&self) -> u64 {
        self.host_rsp.load()
    }

    pub fn kernel_cr3(&self) -> u64 {
        self.kernel_cr3.load()
    }

    pub fn cpu_index(&self) -> Option<CpuIndex> {
        CpuIndex::new(self.cpu_index.load())
    }

    pub fn interrupt_rsp(&self) -> u64 {
        self.interrupt_rsp.load()
    }

    pub fn interrupt_rip(&self) -> u64 {
        self.interrupt_rip.load()
    }

    pub fn interrupt_cr3(&self) -> u64 {
        self.interrupt_cr3.load()
    }

    pub fn call_rsp(&self) -> u64 {
        self.call_rsp.load()
    }

    pub fn call_rip(&self) -> u64 {
        self.call_rip.load()
    }

    pub fn call_cr3(&self) -> u64 {
        self.call_cr3.load()
    }

    pub fn fault_rsp(&self) -> u64 {
        self.fault_rsp.load()
    }

    pub fn fault_rip(&self) -> u64 {
        self.fault_rip.load()
    }

    pub fn fault_cr3(&self) -> u64 {
        self.fault_cr3.load()
    }

    pub fn fault_error_code(&self) -> u64 {
        self.fault_error_code.load()
    }

    pub fn fault_address(&self) -> u64 {
        self.fault_address.load()
    }

    fn reset_evidence(&self) {
        self.host_rsp.store(0);
        self.interrupt_rsp.store(0);
        self.interrupt_rip.store(0);
        self.interrupt_cr3.store(0);
        self.call_rsp.store(0);
        self.call_rip.store(0);
        self.call_cr3.store(0);
        self.fault_rsp.store(0);
        self.fault_rip.store(0);
        self.fault_cr3.store(0);
        self.fault_error_code.store(0);
        self.fault_address.store(0);
        self.irq_count.store(0);
        self.irq_seen.store(0);
        self.preempted.store(0);
        self.call_count.store(0);
        self.call_seen.store(0);
        self.fault_count.store(0);
        self.fault_seen.store(0);
        self.fault_vector.store(0);
    }
}

impl Default for CpuTransitionStorage {
    fn default() -> Self {
        Self::new()
    }
}

// SAFETY: the kernel assigns each slot to one CPU before online publication;
// all mutation and reads remain on that owner with IF controlled.
unsafe impl Sync for CpuTransitionStorage {}

pub const PER_CPU_HOST_RSP_OFFSET: usize = offset_of!(CpuTransitionStorage, host_rsp);
pub const PER_CPU_KERNEL_CR3_OFFSET: usize = offset_of!(CpuTransitionStorage, kernel_cr3);
pub const PER_CPU_INTERRUPT_RSP_OFFSET: usize = offset_of!(CpuTransitionStorage, interrupt_rsp);
pub const PER_CPU_INTERRUPT_RIP_OFFSET: usize = offset_of!(CpuTransitionStorage, interrupt_rip);
pub const PER_CPU_INTERRUPT_CR3_OFFSET: usize = offset_of!(CpuTransitionStorage, interrupt_cr3);
pub const PER_CPU_CALL_RSP_OFFSET: usize = offset_of!(CpuTransitionStorage, call_rsp);
pub const PER_CPU_CALL_RIP_OFFSET: usize = offset_of!(CpuTransitionStorage, call_rip);
pub const PER_CPU_CALL_CR3_OFFSET: usize = offset_of!(CpuTransitionStorage, call_cr3);
pub const PER_CPU_FAULT_RSP_OFFSET: usize = offset_of!(CpuTransitionStorage, fault_rsp);
pub const PER_CPU_FAULT_RIP_OFFSET: usize = offset_of!(CpuTransitionStorage, fault_rip);
pub const PER_CPU_FAULT_CR3_OFFSET: usize = offset_of!(CpuTransitionStorage, fault_cr3);
pub const PER_CPU_FAULT_ERROR_CODE_OFFSET: usize =
    offset_of!(CpuTransitionStorage, fault_error_code);
pub const PER_CPU_FAULT_ADDRESS_OFFSET: usize = offset_of!(CpuTransitionStorage, fault_address);
pub const PER_CPU_IRQ_COUNT_OFFSET: usize = offset_of!(CpuTransitionStorage, irq_count);
pub const PER_CPU_IRQ_SEEN_OFFSET: usize = offset_of!(CpuTransitionStorage, irq_seen);
pub const PER_CPU_PREEMPTED_OFFSET: usize = offset_of!(CpuTransitionStorage, preempted);
pub const PER_CPU_CALL_COUNT_OFFSET: usize = offset_of!(CpuTransitionStorage, call_count);
pub const PER_CPU_CALL_SEEN_OFFSET: usize = offset_of!(CpuTransitionStorage, call_seen);
pub const PER_CPU_FAULT_COUNT_OFFSET: usize = offset_of!(CpuTransitionStorage, fault_count);
pub const PER_CPU_FAULT_SEEN_OFFSET: usize = offset_of!(CpuTransitionStorage, fault_seen);
pub const PER_CPU_FAULT_VECTOR_OFFSET: usize = offset_of!(CpuTransitionStorage, fault_vector);
pub const PER_CPU_CPU_INDEX_OFFSET: usize = offset_of!(CpuTransitionStorage, cpu_index);

const _: () = assert!(core::mem::size_of::<CpuTransitionStorage>() == 128);
const _: () = assert!(core::mem::align_of::<CpuTransitionStorage>() == 64);
