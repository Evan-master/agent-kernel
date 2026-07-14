//! Single-core evidence storage for the ring-3 Agent runtime.
//!
//! Entry, PIT, and Agent-call assembly share one saved host context plus fixed
//! atomic mailboxes. Rust reads them only after assembly has returned at CPL0
//! with IF clear.

use core::{
    arch::asm,
    cell::UnsafeCell,
    sync::atomic::{AtomicU64, AtomicU8, Ordering},
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
pub(super) static AGENT_KERNEL_AGENT_YIELDED: AtomicU8 = AtomicU8::new(0);

static AGENT_KERNEL_AGENT_RUNTIME_READY: AtomicU8 = AtomicU8::new(0);

pub(super) fn initialize() -> Option<()> {
    // SAFETY: this runtime is initialized only on the ring-0 boot CPU.
    unsafe {
        asm!("cli", options(nomem, nostack));
    }
    if AGENT_KERNEL_AGENT_RUNTIME_READY
        .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return None;
    }
    reset_mailbox();
    Some(())
}

pub(super) fn interrupts_are_clear() -> bool {
    let rflags: u64;
    // SAFETY: pushfq/pop only inspect flags on the active kernel stack.
    unsafe {
        asm!("pushfq", "pop {}", out(reg) rflags, options(nomem, preserves_flags));
    }
    rflags & RFLAGS_INTERRUPT_ENABLE == 0
}

fn reset_mailbox() {
    AGENT_KERNEL_HOST_CONTEXT_RSP.store(0);
    AGENT_KERNEL_AGENT_INTERRUPT_RSP.store(0, Ordering::Release);
    AGENT_KERNEL_AGENT_INTERRUPT_RIP.store(0, Ordering::Release);
    AGENT_KERNEL_AGENT_IRQ_COUNT.store(0, Ordering::Release);
    AGENT_KERNEL_AGENT_IRQ_SEEN.store(0, Ordering::Release);
    AGENT_KERNEL_AGENT_PREEMPTED.store(0, Ordering::Release);
    AGENT_KERNEL_AGENT_CALL_RSP.store(0, Ordering::Release);
    AGENT_KERNEL_AGENT_CALL_RIP.store(0, Ordering::Release);
    AGENT_KERNEL_AGENT_CALL_COUNT.store(0, Ordering::Release);
    AGENT_KERNEL_AGENT_CALL_SEEN.store(0, Ordering::Release);
    AGENT_KERNEL_AGENT_YIELDED.store(0, Ordering::Release);
}
