//! Persistent x86_64 exception gates and boot-time breakpoint proof.
//!
//! This architecture-binary module owns the static IDT, privileged `lidt`,
//! exception assembly entries, and a fixed breakpoint mailbox. Other hardware
//! adapters may add gates only while IF is clear. Agent Kernel semantic state
//! never crosses this architecture-local boundary.

use core::{
    arch::{asm, global_asm},
    cell::UnsafeCell,
    sync::atomic::{AtomicU64, AtomicU8, Ordering},
};

use agent_kernel_x86_64::{
    interrupt::{IdtEntry, IdtPointer, PIC_MASTER_OFFSET},
    native_runtime::{GENERAL_PROTECTION_VECTOR, INVALID_OPCODE_VECTOR, PAGE_FAULT_VECTOR},
};

const IDT_ENTRY_COUNT: usize = 256;
const EXCEPTION_VECTOR_COUNT: usize = 32;
const BREAKPOINT_VECTOR: usize = 3;
const QEMU_EXIT_PORT: u16 = 0xf4;
const QEMU_FAILURE: u8 = 0x11;

#[no_mangle]
#[used]
static AGENT_KERNEL_EXCEPTION_SEEN: AtomicU8 = AtomicU8::new(0);

#[no_mangle]
#[used]
static AGENT_KERNEL_EXCEPTION_COUNT: AtomicU8 = AtomicU8::new(0);

#[no_mangle]
#[used]
static AGENT_KERNEL_EXCEPTION_VECTOR: AtomicU8 = AtomicU8::new(0);

#[no_mangle]
#[used]
static AGENT_KERNEL_EXCEPTION_RIP: AtomicU64 = AtomicU64::new(0);

static IDT_READY: AtomicU8 = AtomicU8::new(0);

global_asm!(
    r#"
    .section .text.agent_kernel_exceptions,"ax",@progbits

    .macro fatal_exception name, number
    .global \name
    .type \name,@function
\name:
    mov byte ptr [rip + {exception_vector}], \number
    inc byte ptr [rip + {exception_count}]
    mov dx, {qemu_exit_port}
    mov al, {qemu_failure}
    out dx, al
    cli
1:
    hlt
    jmp 1b
    .size \name, . - \name
    .endm

    fatal_exception agent_kernel_exception_0, 0
    fatal_exception agent_kernel_exception_1, 1
    fatal_exception agent_kernel_exception_2, 2
    fatal_exception agent_kernel_exception_4, 4
    fatal_exception agent_kernel_exception_5, 5
    fatal_exception agent_kernel_exception_6, 6
    fatal_exception agent_kernel_exception_7, 7
    fatal_exception agent_kernel_exception_8, 8
    fatal_exception agent_kernel_exception_9, 9
    fatal_exception agent_kernel_exception_10, 10
    fatal_exception agent_kernel_exception_11, 11
    fatal_exception agent_kernel_exception_12, 12
    fatal_exception agent_kernel_exception_13, 13
    fatal_exception agent_kernel_exception_14, 14
    fatal_exception agent_kernel_exception_15, 15
    fatal_exception agent_kernel_exception_16, 16
    fatal_exception agent_kernel_exception_17, 17
    fatal_exception agent_kernel_exception_18, 18
    fatal_exception agent_kernel_exception_19, 19
    fatal_exception agent_kernel_exception_20, 20
    fatal_exception agent_kernel_exception_21, 21
    fatal_exception agent_kernel_exception_22, 22
    fatal_exception agent_kernel_exception_23, 23
    fatal_exception agent_kernel_exception_24, 24
    fatal_exception agent_kernel_exception_25, 25
    fatal_exception agent_kernel_exception_26, 26
    fatal_exception agent_kernel_exception_27, 27
    fatal_exception agent_kernel_exception_28, 28
    fatal_exception agent_kernel_exception_29, 29
    fatal_exception agent_kernel_exception_30, 30
    fatal_exception agent_kernel_exception_31, 31

    .global agent_kernel_exception_3
    .type agent_kernel_exception_3,@function
agent_kernel_exception_3:
    push rax
    mov rax, qword ptr [rsp + 8]
    mov qword ptr [rip + {exception_rip}], rax
    mov byte ptr [rip + {exception_vector}], 3
    inc byte ptr [rip + {exception_count}]
    mov byte ptr [rip + {exception_seen}], 1
    pop rax
    iretq
    .size agent_kernel_exception_3, . - agent_kernel_exception_3
"#,
    exception_seen = sym AGENT_KERNEL_EXCEPTION_SEEN,
    exception_count = sym AGENT_KERNEL_EXCEPTION_COUNT,
    exception_vector = sym AGENT_KERNEL_EXCEPTION_VECTOR,
    exception_rip = sym AGENT_KERNEL_EXCEPTION_RIP,
    qemu_exit_port = const QEMU_EXIT_PORT,
    qemu_failure = const QEMU_FAILURE,
);

macro_rules! declare_exception_handlers {
    ($($handler:ident),+ $(,)?) => {
        unsafe extern "C" {
            $(fn $handler();)+
        }

        fn exception_handlers() -> [unsafe extern "C" fn(); EXCEPTION_VECTOR_COUNT] {
            [$($handler),+]
        }
    };
}

declare_exception_handlers!(
    agent_kernel_exception_0,
    agent_kernel_exception_1,
    agent_kernel_exception_2,
    agent_kernel_exception_3,
    agent_kernel_exception_4,
    agent_kernel_exception_5,
    agent_kernel_exception_6,
    agent_kernel_exception_7,
    agent_kernel_exception_8,
    agent_kernel_exception_9,
    agent_kernel_exception_10,
    agent_kernel_exception_11,
    agent_kernel_exception_12,
    agent_kernel_exception_13,
    agent_kernel_exception_14,
    agent_kernel_exception_15,
    agent_kernel_exception_16,
    agent_kernel_exception_17,
    agent_kernel_exception_18,
    agent_kernel_exception_19,
    agent_kernel_exception_20,
    agent_kernel_exception_21,
    agent_kernel_exception_22,
    agent_kernel_exception_23,
    agent_kernel_exception_24,
    agent_kernel_exception_25,
    agent_kernel_exception_26,
    agent_kernel_exception_27,
    agent_kernel_exception_28,
    agent_kernel_exception_29,
    agent_kernel_exception_30,
    agent_kernel_exception_31,
);

#[repr(C, align(16))]
struct IdtStorage {
    entries: UnsafeCell<[IdtEntry; IDT_ENTRY_COUNT]>,
}

impl IdtStorage {
    const fn new() -> Self {
        Self {
            entries: UnsafeCell::new([IdtEntry::missing(); IDT_ENTRY_COUNT]),
        }
    }
}

// SAFETY: IDT installation and gate updates occur with IF clear during
// single-core boot, and the table remains live for the image lifetime.
unsafe impl Sync for IdtStorage {}

static IDT: IdtStorage = IdtStorage::new();

pub fn install_and_probe() -> Option<()> {
    // SAFETY: the ring-0 single-core boot path takes ownership of IF before
    // touching the shared descriptor table.
    unsafe {
        asm!("cli", options(nomem, nostack));
    }
    if IDT_READY.load(Ordering::Acquire) != 0 {
        return None;
    }
    reset_probe_mailbox();

    // SAFETY: IF is clear and no other core can observe the table mutation.
    unsafe {
        install_exception_idt()?;
    }

    let expected_rip: u64;
    // SAFETY: vector 3 is installed as a returning trap gate whose assembly
    // preserves the only register it uses and restores the CPU frame via iretq.
    unsafe {
        asm!(
            "lea {expected}, [rip + 2f]",
            "int3",
            "2:",
            expected = lateout(reg) expected_rip,
        );
    }

    let seen = AGENT_KERNEL_EXCEPTION_SEEN.load(Ordering::Acquire);
    let count = AGENT_KERNEL_EXCEPTION_COUNT.load(Ordering::Acquire);
    let vector = AGENT_KERNEL_EXCEPTION_VECTOR.load(Ordering::Acquire);
    let captured_rip = AGENT_KERNEL_EXCEPTION_RIP.load(Ordering::Acquire);
    if seen != 1
        || count != 1
        || usize::from(vector) != BREAKPOINT_VECTOR
        || captured_rip != expected_rip
    {
        return None;
    }

    Some(())
}

pub unsafe fn install_irq_gate(vector: u8, handler: unsafe extern "C" fn()) -> Option<()> {
    if IDT_READY.load(Ordering::Acquire) != 1 || vector < PIC_MASTER_OFFSET {
        return None;
    }
    let selector = unsafe { current_code_selector() };
    let entries = IDT.entries.get().cast::<IdtEntry>();
    // SAFETY: the caller guarantees IF is clear during this single-core update;
    // volatile storage makes the descriptor write visible to the CPU-owned IDT.
    unsafe {
        entries
            .add(usize::from(vector))
            .write_volatile(IdtEntry::interrupt_gate(handler_address(handler), selector));
    }
    Some(())
}

pub unsafe fn install_user_interrupt_gate(
    vector: u8,
    handler: unsafe extern "C" fn(),
) -> Option<()> {
    if IDT_READY.load(Ordering::Acquire) != 1 || vector < PIC_MASTER_OFFSET {
        return None;
    }
    let selector = unsafe { current_code_selector() };
    let entries = IDT.entries.get().cast::<IdtEntry>();
    // SAFETY: the caller holds IF clear during this single-core gate update.
    unsafe {
        entries
            .add(usize::from(vector))
            .write_volatile(IdtEntry::user_interrupt_gate(
                handler_address(handler),
                selector,
            ));
    }
    Some(())
}

pub unsafe fn install_agent_exception_gate(
    vector: u8,
    handler: unsafe extern "C" fn(),
) -> Option<()> {
    if IDT_READY.load(Ordering::Acquire) != 1
        || (vector != INVALID_OPCODE_VECTOR
            && vector != GENERAL_PROTECTION_VECTOR
            && vector != PAGE_FAULT_VECTOR)
    {
        return None;
    }
    let selector = unsafe { current_code_selector() };
    let entries = IDT.entries.get().cast::<IdtEntry>();
    // SAFETY: the caller holds IF clear during this single-core gate update.
    unsafe {
        entries
            .add(usize::from(vector))
            .write_volatile(IdtEntry::interrupt_gate(handler_address(handler), selector));
    }
    Some(())
}

fn reset_probe_mailbox() {
    AGENT_KERNEL_EXCEPTION_SEEN.store(0, Ordering::Release);
    AGENT_KERNEL_EXCEPTION_COUNT.store(0, Ordering::Release);
    AGENT_KERNEL_EXCEPTION_VECTOR.store(0, Ordering::Release);
    AGENT_KERNEL_EXCEPTION_RIP.store(0, Ordering::Release);
}

unsafe fn install_exception_idt() -> Option<()> {
    let selector = unsafe { current_code_selector() };
    let handlers = exception_handlers();
    let entries = IDT.entries.get().cast::<IdtEntry>();
    for (vector, handler) in handlers.into_iter().enumerate() {
        let address = handler_address(handler);
        let entry = if vector == BREAKPOINT_VECTOR {
            IdtEntry::trap_gate(address, selector)
        } else {
            IdtEntry::interrupt_gate(address, selector)
        };
        // SAFETY: the caller holds IF clear, boot remains single-core, and the
        // volatile store publishes each descriptor to the CPU-owned table.
        unsafe {
            entries.add(vector).write_volatile(entry);
        }
    }
    let pointer = IdtPointer::for_table(entries as u64, IDT_ENTRY_COUNT)?;

    // SAFETY: `pointer` covers the complete static table, which remains live.
    unsafe {
        asm!(
            "lidt [{pointer}]",
            pointer = in(reg) &pointer,
            options(readonly, nostack, preserves_flags)
        );
    }
    IDT_READY.store(1, Ordering::Release);
    Some(())
}

fn handler_address(handler: unsafe extern "C" fn()) -> u64 {
    handler as *const () as usize as u64
}

unsafe fn current_code_selector() -> u16 {
    let selector: u16;
    unsafe {
        asm!(
            "mov {selector:x}, cs",
            selector = out(reg) selector,
            options(nomem, nostack, preserves_flags)
        );
    }
    selector
}
