//! Permanent x86_64 GDT, TSS, and privileged-entry stack.
//!
//! This architecture-binary module installs the segment authority model and
//! owns one indexed RSP0 stack per logical CPU. Descriptor bytes come from the
//! host-tested architecture library.

mod assembly;

use core::{
    arch::asm,
    cell::UnsafeCell,
    sync::atomic::{AtomicU8, Ordering},
};

use agent_kernel_x86_64::cpu::{CpuIndex, MAX_CPU_COUNT};
use agent_kernel_x86_64::privilege::{
    gdt_entries, GdtPointer, TaskStateSegment64, GDT_ENTRY_COUNT, KERNEL_CODE_SELECTOR,
    KERNEL_DATA_SELECTOR, TSS_SELECTOR,
};

const PRIVILEGED_STACK_BYTES: usize = 32 * 1024;
const PRIVILEGED_STACK_CANARY: u64 = 0x5253_5030_5354_414b;

#[repr(C, align(4096))]
struct PrivilegedStack {
    bytes: UnsafeCell<[u8; PRIVILEGED_STACK_BYTES]>,
}

impl PrivilegedStack {
    const fn new() -> Self {
        Self {
            bytes: UnsafeCell::new([0; PRIVILEGED_STACK_BYTES]),
        }
    }
}

struct TssStorage {
    value: UnsafeCell<TaskStateSegment64>,
}

struct GdtStorage {
    entries: UnsafeCell<[u64; GDT_ENTRY_COUNT]>,
}

struct CpuPrivilegeSlot {
    stack: PrivilegedStack,
    tss: TssStorage,
    gdt: GdtStorage,
    install_state: AtomicU8,
}

impl CpuPrivilegeSlot {
    const fn new() -> Self {
        Self {
            stack: PrivilegedStack::new(),
            tss: TssStorage {
                value: UnsafeCell::new(TaskStateSegment64::new(0)),
            },
            gdt: GdtStorage {
                entries: UnsafeCell::new([0; GDT_ENTRY_COUNT]),
            },
            install_state: AtomicU8::new(0),
        }
    }
}

// SAFETY: each logical CPU exclusively installs and uses its indexed slot;
// slots remain live and disjoint for the kernel image lifetime.
unsafe impl Sync for CpuPrivilegeSlot {}

static PRIVILEGE_SLOTS: [CpuPrivilegeSlot; MAX_CPU_COUNT] =
    [const { CpuPrivilegeSlot::new() }; MAX_CPU_COUNT];

#[derive(Copy, Clone)]
pub(crate) struct PrivilegedStackBounds {
    pub(crate) start: usize,
    pub(crate) end: usize,
}

pub(crate) struct PrivilegeBoundary {
    cpu: CpuIndex,
    stack: PrivilegedStackBounds,
}

impl PrivilegeBoundary {
    pub(crate) fn install(cpu: CpuIndex) -> Option<Self> {
        // SAFETY: descriptor replacement is owned by the indexed CPU.
        unsafe {
            asm!("cli", options(nomem, nostack));
        }
        let slot = PRIVILEGE_SLOTS.get(cpu.as_usize())?;
        if slot
            .install_state
            .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return None;
        }

        let start = slot.stack.bytes.get().cast::<u8>() as usize;
        let end = start.checked_add(PRIVILEGED_STACK_BYTES)?;
        if !start.is_multiple_of(4096) || !end.is_multiple_of(16) {
            return None;
        }
        // SAFETY: installation exclusively initializes the static stack.
        unsafe {
            (start as *mut u64).write_volatile(PRIVILEGED_STACK_CANARY);
        }

        let tss_ptr = slot.tss.value.get();
        // SAFETY: IF is clear and the TSS has not been loaded before this write.
        unsafe {
            tss_ptr.write_volatile(TaskStateSegment64::new(end as u64));
        }
        let entries = gdt_entries(tss_ptr as usize as u64);
        let gdt_ptr = slot.gdt.entries.get();
        // SAFETY: the permanent table is exclusively initialized before lgdt.
        unsafe {
            gdt_ptr.write_volatile(entries);
        }
        let pointer = GdtPointer::for_table(gdt_ptr as usize as u64, GDT_ENTRY_COUNT)?;

        // SAFETY: the pointer covers live static descriptors and all selectors
        // name entries in that table.
        unsafe {
            assembly::load_tables(
                &pointer,
                KERNEL_CODE_SELECTOR,
                KERNEL_DATA_SELECTOR,
                TSS_SELECTOR,
            );
        }
        if current_code_selector() != KERNEL_CODE_SELECTOR
            || current_task_selector() != TSS_SELECTOR
            || !stack_canary_valid(PrivilegedStackBounds { start, end })
        {
            return None;
        }

        slot.install_state.store(2, Ordering::Release);
        Some(Self {
            cpu,
            stack: PrivilegedStackBounds { start, end },
        })
    }

    pub(crate) const fn stack_bounds(&self) -> PrivilegedStackBounds {
        self.stack
    }

    pub(crate) const fn cpu(&self) -> CpuIndex {
        self.cpu
    }
}

pub(crate) fn startup_stack_top(cpu: CpuIndex) -> Option<u64> {
    let slot = PRIVILEGE_SLOTS.get(cpu.as_usize())?;
    let start = slot.stack.bytes.get().cast::<u8>() as usize;
    let end = start.checked_add(PRIVILEGED_STACK_BYTES)?;
    (start.is_multiple_of(4096) && end.is_multiple_of(16)).then_some(end as u64)
}

pub(crate) fn stack_canary_valid(stack: PrivilegedStackBounds) -> bool {
    // SAFETY: bounds are created only from the live static RSP0 stack.
    unsafe { (stack.start as *const u64).read_volatile() == PRIVILEGED_STACK_CANARY }
}

pub(crate) fn current_privilege_level() -> u16 {
    current_code_selector() & 0x3
}

fn current_code_selector() -> u16 {
    let selector: u16;
    // SAFETY: reading CS does not mutate machine state.
    unsafe {
        asm!(
            "mov {selector:x}, cs",
            selector = out(reg) selector,
            options(nomem, nostack, preserves_flags)
        );
    }
    selector
}

fn current_task_selector() -> u16 {
    let selector: u16;
    // SAFETY: str only copies the currently loaded task selector.
    unsafe {
        asm!(
            "str {selector:x}",
            selector = out(reg) selector,
            options(nomem, nostack, preserves_flags)
        );
    }
    selector
}
