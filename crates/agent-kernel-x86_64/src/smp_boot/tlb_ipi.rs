//! Hardware execution of the generation-bound TLB IPI mailbox.

use core::{
    arch::{asm, global_asm},
    sync::atomic::{AtomicU64, Ordering},
};

use agent_kernel_x86_64::{
    apic::{LocalApicBase, LocalApicMmio, VolatileMmio},
    cpu::{CpuIndex, CpuMask},
    per_cpu::{PER_CPU_CPU_INDEX_OFFSET, PER_CPU_KERNEL_CR3_OFFSET},
    tlb::{TlbFlushKind, TlbIpiMailbox, TlbIpiWork, TlbShootdownRequest},
};

const CR3_CONTROL_MASK: u64 = (1 << 3) | (1 << 4);
const CR4_PAGE_GLOBAL_ENABLE: u64 = 1 << 7;

global_asm!(
    r#"
    .section .text.agent_kernel_tlb_ipi,"ax",@progbits

    .global agent_kernel_tlb_ipi_stub
    .type agent_kernel_tlb_ipi_stub,@function
agent_kernel_tlb_ipi_stub:
    push rax
    push rbx
    push rcx
    push rdx
    push rsi
    push rdi
    push rbp
    push r8
    push r9
    push r10
    push r11
    push r12
    push r13
    push r14
    push r15
    mov r15, cr3
    mov rax, qword ptr gs:[{kernel_cr3_offset}]
    cmp r15, rax
    je 1f
    mov cr3, rax
1:
    movzx edi, word ptr gs:[{cpu_index_offset}]
    mov rsi, r15
    mov r12, rsp
    and rsp, -16
    call agent_kernel_tlb_ipi_handle
    mov rsp, r12
    mov rax, qword ptr gs:[{kernel_cr3_offset}]
    cmp r15, rax
    je 2f
    mov cr3, r15
2:
    pop r15
    pop r14
    pop r13
    pop r12
    pop r11
    pop r10
    pop r9
    pop r8
    pop rbp
    pop rdi
    pop rsi
    pop rdx
    pop rcx
    pop rbx
    pop rax
    iretq
    .size agent_kernel_tlb_ipi_stub, . - agent_kernel_tlb_ipi_stub
"#,
    kernel_cr3_offset = const PER_CPU_KERNEL_CR3_OFFSET,
    cpu_index_offset = const PER_CPU_CPU_INDEX_OFFSET,
);

unsafe extern "C" {
    fn agent_kernel_tlb_ipi_stub();
}

static MAILBOX: TlbIpiMailbox = TlbIpiMailbox::new();
static LOCAL_APIC_BASE: AtomicU64 = AtomicU64::new(0);
static PHYSICAL_OFFSET: AtomicU64 = AtomicU64::new(0);

pub(super) fn configure(local_apic_base: LocalApicBase, physical_offset: u64) -> Option<()> {
    if LOCAL_APIC_BASE
        .compare_exchange(
            0,
            local_apic_base.physical(),
            Ordering::AcqRel,
            Ordering::Acquire,
        )
        .is_err()
        || PHYSICAL_OFFSET
            .compare_exchange(0, physical_offset, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
    {
        return None;
    }
    Some(())
}

pub(super) const fn handler() -> unsafe extern "C" fn() {
    agent_kernel_tlb_ipi_stub
}

pub(super) fn publish(request: TlbShootdownRequest) -> Option<()> {
    MAILBOX.publish(request).ok()
}

pub(super) fn acknowledged() -> CpuMask {
    MAILBOX.acknowledged_mask()
}

pub(super) fn finish(generation: u64) -> Option<TlbIpiWork> {
    MAILBOX.finish(generation).ok()
}

pub(super) fn reset_complete() -> Option<()> {
    MAILBOX.reset_complete().ok()
}

pub(super) fn mark_timed_out(generation: u64) -> Option<CpuMask> {
    MAILBOX.mark_timed_out(generation).ok()
}

#[no_mangle]
extern "C" fn agent_kernel_tlb_ipi_handle(cpu_raw: u16, original_cr3: u64) {
    let Some(cpu) = CpuIndex::new(cpu_raw) else {
        end_of_interrupt();
        return;
    };
    if let Ok(work) = MAILBOX.work_for(cpu) {
        // SAFETY: work contains canonical, bounded values published by the BSP;
        // every target address space retains the shared supervisor mapping.
        if unsafe { flush(work, original_cr3) } {
            let _ = MAILBOX.acknowledge(cpu, work.generation());
        }
    }
    end_of_interrupt();
}

unsafe fn flush(work: TlbIpiWork, original_cr3: u64) -> bool {
    let kernel_cr3 = read_cr3();
    match work.scope().kind() {
        TlbFlushKind::Page | TlbFlushKind::Range | TlbFlushKind::AddressSpace => {
            let target_cr3 = work.address_space().root() | (original_cr3 & CR3_CONTROL_MASK);
            write_cr3(target_cr3);
            match work.scope().kind() {
                TlbFlushKind::Page | TlbFlushKind::Range => {
                    let Some(start) = work.scope().start() else {
                        write_cr3(kernel_cr3);
                        return false;
                    };
                    let Some(pages) = work.scope().page_count() else {
                        write_cr3(kernel_cr3);
                        return false;
                    };
                    for page in 0..u64::from(pages) {
                        let address = start + page * 4096;
                        // SAFETY: scope construction proved canonical aligned
                        // addresses and a bounded non-overflowing range.
                        unsafe {
                            asm!("invlpg [{}]", in(reg) address, options(nostack, preserves_flags));
                        }
                    }
                }
                TlbFlushKind::AddressSpace => write_cr3(target_cr3),
                TlbFlushKind::AllContexts => return false,
            }
            write_cr3(kernel_cr3);
        }
        TlbFlushKind::AllContexts => {
            let cr4 = read_cr4();
            if cr4 & CR4_PAGE_GLOBAL_ENABLE != 0 {
                write_cr4(cr4 & !CR4_PAGE_GLOBAL_ENABLE);
                write_cr4(cr4);
            }
            write_cr3(kernel_cr3);
        }
    }
    true
}

fn end_of_interrupt() {
    let base = LOCAL_APIC_BASE.load(Ordering::Acquire);
    let offset = PHYSICAL_OFFSET.load(Ordering::Acquire);
    let Some(base) = LocalApicBase::new(base) else {
        return;
    };
    if let Some(mut apic) = LocalApicMmio::new(base, offset, VolatileMmio) {
        apic.end_of_interrupt();
    }
}

#[no_mangle]
extern "C" fn agent_kernel_local_apic_eoi() {
    end_of_interrupt();
}

fn read_cr3() -> u64 {
    let value: u64;
    // SAFETY: privileged interrupt context reads the active root only.
    unsafe {
        asm!("mov {}, cr3", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

fn write_cr3(value: u64) {
    // SAFETY: callers supply a validated retained root and architectural CR3
    // control bits while executing through shared supervisor mappings.
    unsafe {
        asm!("mov cr3, {}", in(reg) value, options(nostack, preserves_flags));
    }
}

fn read_cr4() -> u64 {
    let value: u64;
    // SAFETY: privileged interrupt context reads CR4 without mutation.
    unsafe {
        asm!("mov {}, cr4", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

fn write_cr4(value: u64) {
    // SAFETY: the handler changes only PGE and immediately restores CR4.
    unsafe {
        asm!("mov cr4, {}", in(reg) value, options(nomem, nostack, preserves_flags));
    }
}
