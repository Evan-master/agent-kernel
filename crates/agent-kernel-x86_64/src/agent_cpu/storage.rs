//! Single-core evidence storage shared by ring-3 Agent contexts.
//!
//! Entry, PIT, Agent-call, and exception assembly share one saved host context plus fixed
//! per-dispatch atomic mailboxes. Rust reads them only after assembly has
//! returned at CPL0 with IF clear.

use core::{
    arch::asm,
    cell::UnsafeCell,
    sync::atomic::{AtomicU64, AtomicU8, Ordering},
};

use agent_kernel_x86_64::{
    address_space::AddressSpaceRoots,
    native_runtime::{NativeRunBoundary, NativeRunBoundaryEvidence},
};

pub(super) const RFLAGS_INTERRUPT_ENABLE: u64 = 1 << 9;

#[repr(C, align(8))]
pub(super) struct ContextSlot {
    rsp: UnsafeCell<u64>,
}

impl ContextSlot {
    const fn new() -> Self {
        Self {
            rsp: UnsafeCell::new(0),
        }
    }

    pub(super) fn pointer(&self) -> *mut u64 {
        self.rsp.get()
    }

    pub(super) fn load(&self) -> u64 {
        // SAFETY: reads occur after the assembly writer returned with IF clear.
        unsafe { self.rsp.get().read_volatile() }
    }

    fn store(&self, rsp: u64) {
        // SAFETY: reset occurs before ring-3 entry with IF clear.
        unsafe {
            self.rsp.get().write_volatile(rsp);
        }
    }
}

// SAFETY: one boot CPU owns the slot and every transition keeps IF controlled.
unsafe impl Sync for ContextSlot {}

#[no_mangle]
#[used]
pub(super) static AGENT_KERNEL_HOST_CONTEXT_RSP: ContextSlot = ContextSlot::new();

#[no_mangle]
#[used]
pub(super) static AGENT_KERNEL_AGENT_INTERRUPT_RSP: AtomicU64 = AtomicU64::new(0);

#[no_mangle]
#[used]
pub(super) static AGENT_KERNEL_AGENT_INTERRUPT_RIP: AtomicU64 = AtomicU64::new(0);

#[no_mangle]
#[used]
pub(super) static AGENT_KERNEL_AGENT_IRQ_COUNT: AtomicU8 = AtomicU8::new(0);

#[no_mangle]
#[used]
pub(super) static AGENT_KERNEL_AGENT_IRQ_SEEN: AtomicU8 = AtomicU8::new(0);

#[no_mangle]
#[used]
pub(super) static AGENT_KERNEL_AGENT_PREEMPTED: AtomicU8 = AtomicU8::new(0);

#[no_mangle]
#[used]
pub(super) static AGENT_KERNEL_KERNEL_CR3: AtomicU64 = AtomicU64::new(0);

#[no_mangle]
#[used]
pub(super) static AGENT_KERNEL_AGENT_INTERRUPT_CR3: AtomicU64 = AtomicU64::new(0);

#[no_mangle]
#[used]
pub(super) static AGENT_KERNEL_AGENT_CALL_RSP: AtomicU64 = AtomicU64::new(0);

#[no_mangle]
#[used]
pub(super) static AGENT_KERNEL_AGENT_CALL_RIP: AtomicU64 = AtomicU64::new(0);

#[no_mangle]
#[used]
pub(super) static AGENT_KERNEL_AGENT_CALL_COUNT: AtomicU8 = AtomicU8::new(0);

#[no_mangle]
#[used]
pub(super) static AGENT_KERNEL_AGENT_CALL_SEEN: AtomicU8 = AtomicU8::new(0);

#[no_mangle]
#[used]
pub(super) static AGENT_KERNEL_AGENT_CALL_CR3: AtomicU64 = AtomicU64::new(0);

#[no_mangle]
#[used]
pub(super) static AGENT_KERNEL_AGENT_FAULT_RSP: AtomicU64 = AtomicU64::new(0);

#[no_mangle]
#[used]
pub(super) static AGENT_KERNEL_AGENT_FAULT_RIP: AtomicU64 = AtomicU64::new(0);

#[no_mangle]
#[used]
pub(super) static AGENT_KERNEL_AGENT_FAULT_CR3: AtomicU64 = AtomicU64::new(0);

#[no_mangle]
#[used]
pub(super) static AGENT_KERNEL_AGENT_FAULT_COUNT: AtomicU8 = AtomicU8::new(0);

#[no_mangle]
#[used]
pub(super) static AGENT_KERNEL_AGENT_FAULT_SEEN: AtomicU8 = AtomicU8::new(0);

#[no_mangle]
#[used]
pub(super) static AGENT_KERNEL_AGENT_FAULT_VECTOR: AtomicU8 = AtomicU8::new(0);

#[no_mangle]
#[used]
pub(super) static AGENT_KERNEL_AGENT_FAULT_ERROR_CODE: AtomicU64 = AtomicU64::new(0);

static AGENT_KERNEL_AGENT_RUNTIME_READY: AtomicU8 = AtomicU8::new(0);

pub(super) fn install(roots: AddressSpaceRoots) -> Option<()> {
    // SAFETY: this runtime is initialized only on the ring-0 boot CPU.
    unsafe {
        asm!("cli", options(nomem, nostack));
    }
    if current_raw_cr3() != roots.kernel_cr3()
        || AGENT_KERNEL_AGENT_RUNTIME_READY
            .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
    {
        return None;
    }
    AGENT_KERNEL_KERNEL_CR3.store(roots.kernel_cr3(), Ordering::Release);
    reset_mailbox();
    Some(())
}

pub(super) fn begin_dispatch(roots: AddressSpaceRoots) -> Option<()> {
    // SAFETY: each dispatch begins in single-core kernel context.
    unsafe {
        asm!("cli", options(nomem, nostack));
    }
    if AGENT_KERNEL_AGENT_RUNTIME_READY.load(Ordering::Acquire) != 1
        || current_raw_cr3() != roots.kernel_cr3()
        || AGENT_KERNEL_KERNEL_CR3.load(Ordering::Acquire) != roots.kernel_cr3()
    {
        return None;
    }
    reset_mailbox();
    Some(())
}

pub(super) fn current_raw_cr3() -> u64 {
    let raw: u64;
    // SAFETY: reading CR3 does not modify address-space state.
    unsafe {
        asm!("mov {}, cr3", out(reg) raw, options(nomem, nostack, preserves_flags));
    }
    raw
}

pub(super) fn interrupts_are_clear() -> bool {
    let rflags: u64;
    // SAFETY: pushfq/pop only inspect flags on the active kernel stack.
    unsafe {
        asm!("pushfq", "pop {}", out(reg) rflags, options(nomem, preserves_flags));
    }
    rflags & RFLAGS_INTERRUPT_ENABLE == 0
}

pub(super) fn run_boundary() -> Option<NativeRunBoundary> {
    NativeRunBoundaryEvidence::new(
        AGENT_KERNEL_AGENT_CALL_COUNT.load(Ordering::Acquire),
        AGENT_KERNEL_AGENT_IRQ_COUNT.load(Ordering::Acquire),
        AGENT_KERNEL_AGENT_FAULT_COUNT.load(Ordering::Acquire),
        AGENT_KERNEL_AGENT_CALL_SEEN.load(Ordering::Acquire) == 1,
        AGENT_KERNEL_AGENT_IRQ_SEEN.load(Ordering::Acquire) == 1,
        AGENT_KERNEL_AGENT_PREEMPTED.load(Ordering::Acquire) == 1,
        AGENT_KERNEL_AGENT_FAULT_SEEN.load(Ordering::Acquire) == 1,
        AGENT_KERNEL_AGENT_FAULT_VECTOR.load(Ordering::Acquire),
        AGENT_KERNEL_AGENT_FAULT_ERROR_CODE.load(Ordering::Acquire),
    )
    .classify()
    .ok()
}

fn reset_mailbox() {
    AGENT_KERNEL_HOST_CONTEXT_RSP.store(0);
    AGENT_KERNEL_AGENT_INTERRUPT_RSP.store(0, Ordering::Release);
    AGENT_KERNEL_AGENT_INTERRUPT_RIP.store(0, Ordering::Release);
    AGENT_KERNEL_AGENT_INTERRUPT_CR3.store(0, Ordering::Release);
    AGENT_KERNEL_AGENT_IRQ_COUNT.store(0, Ordering::Release);
    AGENT_KERNEL_AGENT_IRQ_SEEN.store(0, Ordering::Release);
    AGENT_KERNEL_AGENT_PREEMPTED.store(0, Ordering::Release);
    AGENT_KERNEL_AGENT_CALL_RSP.store(0, Ordering::Release);
    AGENT_KERNEL_AGENT_CALL_RIP.store(0, Ordering::Release);
    AGENT_KERNEL_AGENT_CALL_CR3.store(0, Ordering::Release);
    AGENT_KERNEL_AGENT_CALL_COUNT.store(0, Ordering::Release);
    AGENT_KERNEL_AGENT_CALL_SEEN.store(0, Ordering::Release);
    AGENT_KERNEL_AGENT_FAULT_RSP.store(0, Ordering::Release);
    AGENT_KERNEL_AGENT_FAULT_RIP.store(0, Ordering::Release);
    AGENT_KERNEL_AGENT_FAULT_CR3.store(0, Ordering::Release);
    AGENT_KERNEL_AGENT_FAULT_COUNT.store(0, Ordering::Release);
    AGENT_KERNEL_AGENT_FAULT_SEEN.store(0, Ordering::Release);
    AGENT_KERNEL_AGENT_FAULT_VECTOR.store(0, Ordering::Release);
    AGENT_KERNEL_AGENT_FAULT_ERROR_CODE.store(0, Ordering::Release);
}
