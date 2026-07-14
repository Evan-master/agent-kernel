//! Single-core storage for the fixed x86_64 Agent CPU runtime.
//!
//! The runtime and interrupt assembly share this module's stack, context
//! pointers, and atomic evidence mailbox. Access is valid only on the boot CPU
//! while the runtime type-state owns the current transition.

use core::{
    arch::asm,
    cell::UnsafeCell,
    sync::atomic::{AtomicU64, AtomicU8, Ordering},
};

use agent_kernel_x86_64::context::{bootstrap_stack_pointer, CalleeSavedFrame};

const AGENT_STACK_BYTES: usize = 32 * 1024;
const STACK_CANARY: u64 = 0x4147_454e_5453_544b;
pub(super) const RFLAGS_INTERRUPT_ENABLE: u64 = 1 << 9;

#[repr(C, align(16))]
struct AgentStack {
    bytes: UnsafeCell<[u8; AGENT_STACK_BYTES]>,
}

impl AgentStack {
    const fn new() -> Self {
        Self {
            bytes: UnsafeCell::new([0; AGENT_STACK_BYTES]),
        }
    }
}

// SAFETY: storage is accessed only by the single-core boot CPU. Runtime
// transitions keep IF clear except while that CPU executes the Agent.
unsafe impl Sync for AgentStack {}

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
        // SAFETY: reads occur with IF clear after the assembly writer returned.
        unsafe { self.rsp.get().read_volatile() }
    }

    fn store(&self, rsp: u64) {
        // SAFETY: initialization occurs with IF clear before Agent entry.
        unsafe {
            self.rsp.get().write_volatile(rsp);
        }
    }
}

// SAFETY: the slot follows the same single-core/IF ownership rule as the stack.
unsafe impl Sync for ContextSlot {}

#[no_mangle]
#[used]
static AGENT_KERNEL_AGENT_STACK: AgentStack = AgentStack::new();

#[no_mangle]
#[used]
pub(super) static AGENT_KERNEL_HOST_CONTEXT_RSP: ContextSlot = ContextSlot::new();

#[no_mangle]
#[used]
pub(super) static AGENT_KERNEL_AGENT_CONTEXT_RSP: ContextSlot = ContextSlot::new();

#[no_mangle]
#[used]
pub(super) static AGENT_KERNEL_AGENT_INTERRUPT_RSP: AtomicU64 = AtomicU64::new(0);

#[no_mangle]
#[used]
pub(super) static AGENT_KERNEL_AGENT_INTERRUPT_RIP: AtomicU64 = AtomicU64::new(0);

#[no_mangle]
#[used]
pub(super) static AGENT_KERNEL_AGENT_ENTRY_RSP: AtomicU64 = AtomicU64::new(0);

#[no_mangle]
#[used]
pub(super) static AGENT_KERNEL_AGENT_ENTRY_COUNT: AtomicU8 = AtomicU8::new(0);

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
pub(super) static AGENT_KERNEL_AGENT_RESUMED: AtomicU8 = AtomicU8::new(0);

#[no_mangle]
#[used]
pub(super) static AGENT_KERNEL_AGENT_YIELDED: AtomicU8 = AtomicU8::new(0);

pub(super) static AGENT_KERNEL_AGENT_ARM_FAILED: AtomicU8 = AtomicU8::new(0);
static AGENT_KERNEL_AGENT_RUNTIME_READY: AtomicU8 = AtomicU8::new(0);

#[derive(Copy, Clone)]
pub(super) struct StackBounds {
    pub(super) start: usize,
    pub(super) end: usize,
}

pub(super) fn initialize(entry_rip: u64) -> Option<StackBounds> {
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

    let start = AGENT_KERNEL_AGENT_STACK.bytes.get().cast::<u8>() as usize;
    let end = start.checked_add(AGENT_STACK_BYTES)?;
    let initial_rsp = bootstrap_stack_pointer(start, AGENT_STACK_BYTES)?;
    let frame = CalleeSavedFrame {
        r15: 0,
        r14: 0,
        r13: 0,
        r12: 0,
        rbx: 0,
        rbp: 0,
        return_rip: entry_rip,
    };

    // SAFETY: the computed pointer is aligned, lies within the exclusive
    // static stack, and reserves the ABI padding above the frame.
    unsafe {
        (start as *mut u64).write_volatile(STACK_CANARY);
        (initial_rsp as *mut CalleeSavedFrame).write_volatile(frame);
    }
    AGENT_KERNEL_AGENT_CONTEXT_RSP.store(initial_rsp as u64);

    Some(StackBounds { start, end })
}

pub(super) fn stack_canary_valid(stack: StackBounds) -> bool {
    // SAFETY: the bounds originate from the live static stack.
    unsafe { (stack.start as *const u64).read_volatile() == STACK_CANARY }
}

pub(super) fn interrupts_are_clear() -> bool {
    let rflags: u64;
    // SAFETY: pushfq/pop only inspect the current flags on the owned stack.
    unsafe {
        asm!("pushfq", "pop {}", out(reg) rflags, options(nomem, preserves_flags));
    }
    rflags & RFLAGS_INTERRUPT_ENABLE == 0
}

fn reset_mailbox() {
    AGENT_KERNEL_HOST_CONTEXT_RSP.store(0);
    AGENT_KERNEL_AGENT_CONTEXT_RSP.store(0);
    AGENT_KERNEL_AGENT_INTERRUPT_RSP.store(0, Ordering::Release);
    AGENT_KERNEL_AGENT_INTERRUPT_RIP.store(0, Ordering::Release);
    AGENT_KERNEL_AGENT_ENTRY_RSP.store(0, Ordering::Release);
    AGENT_KERNEL_AGENT_ENTRY_COUNT.store(0, Ordering::Release);
    AGENT_KERNEL_AGENT_IRQ_COUNT.store(0, Ordering::Release);
    AGENT_KERNEL_AGENT_IRQ_SEEN.store(0, Ordering::Release);
    AGENT_KERNEL_AGENT_PREEMPTED.store(0, Ordering::Release);
    AGENT_KERNEL_AGENT_RESUMED.store(0, Ordering::Release);
    AGENT_KERNEL_AGENT_YIELDED.store(0, Ordering::Release);
    AGENT_KERNEL_AGENT_ARM_FAILED.store(0, Ordering::Release);
}
