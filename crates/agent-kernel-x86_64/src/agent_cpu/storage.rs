//! Per-CPU transition slot installation for ring-3 Agent contexts.
//!
//! The binary owns one fixed slot per logical CPU and binds the active slot to
//! IA32_GS_BASE before that CPU can dispatch an Agent. Assembly uses GS-relative
//! offsets; Rust retains the typed slot reference in `AgentCpuRuntime`.

use core::arch::asm;

use agent_kernel_x86_64::{
    address_space::AddressSpaceRoots,
    cpu::{CpuIndex, MAX_CPU_COUNT},
    per_cpu::CpuTransitionStorage,
};

pub(super) const RFLAGS_INTERRUPT_ENABLE: u64 = 1 << 9;
const IA32_GS_BASE: u32 = 0xc000_0101;

static TRANSITION_SLOTS: [CpuTransitionStorage; MAX_CPU_COUNT] =
    [const { CpuTransitionStorage::new() }; MAX_CPU_COUNT];

pub(super) fn install(
    roots: AddressSpaceRoots,
    cpu: CpuIndex,
) -> Option<&'static CpuTransitionStorage> {
    if current_raw_cr3() != roots.kernel_cr3() {
        return None;
    }
    install_kernel_slot(roots.kernel_cr3(), cpu)
}

pub(crate) fn install_kernel_slot(
    kernel_cr3: u64,
    cpu: CpuIndex,
) -> Option<&'static CpuTransitionStorage> {
    // SAFETY: the CPU takes ownership of IF before binding its private slot.
    unsafe {
        asm!("cli", options(nomem, nostack));
    }
    if current_raw_cr3() != kernel_cr3 {
        return None;
    }
    disable_fsgsbase();
    let slot = TRANSITION_SLOTS.get(cpu.as_usize())?;
    slot.install(kernel_cr3).ok()?;
    write_msr(IA32_GS_BASE, slot.as_ptr() as usize as u64);
    (read_msr(IA32_GS_BASE) == slot.as_ptr() as usize as u64).then_some(slot)
}

pub(super) fn begin_dispatch(slot: &CpuTransitionStorage, roots: AddressSpaceRoots) -> Option<()> {
    // SAFETY: dispatch begins on the slot-owning CPU and keeps IF clear until
    // the privilege frame enables interrupts during iretq.
    unsafe {
        asm!("cli", options(nomem, nostack));
    }
    if current_raw_cr3() != roots.kernel_cr3() {
        return None;
    }
    slot.begin_dispatch(roots.kernel_cr3()).ok()
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

fn read_msr(register: u32) -> u64 {
    let low: u32;
    let high: u32;
    // SAFETY: Agent CPU installation runs at CPL0 and reads the architectural
    // GS base MSR assigned to the current CPU.
    unsafe {
        asm!(
            "rdmsr",
            in("ecx") register,
            out("eax") low,
            out("edx") high,
            options(nomem, nostack, preserves_flags)
        );
    }
    (u64::from(high) << 32) | u64::from(low)
}

fn write_msr(register: u32, value: u64) {
    let low = value as u32;
    let high = (value >> 32) as u32;
    // SAFETY: installation owns the current CPU with IF clear and writes only
    // IA32_GS_BASE to bind its live static transition slot.
    unsafe {
        asm!(
            "wrmsr",
            in("ecx") register,
            in("eax") low,
            in("edx") high,
            options(nomem, nostack, preserves_flags)
        );
    }
}

fn disable_fsgsbase() {
    const CR4_FSGSBASE: u64 = 1 << 16;
    let mut cr4: u64;
    // SAFETY: the CPU owns IF and no Agent is active while its transition slot
    // is installed. Clearing FSGSBASE prevents CPL3 from replacing kernel GS.
    unsafe {
        asm!("mov {}, cr4", out(reg) cr4, options(nomem, nostack, preserves_flags));
        cr4 &= !CR4_FSGSBASE;
        asm!("mov cr4, {}", in(reg) cr4, options(nomem, nostack, preserves_flags));
    }
}
