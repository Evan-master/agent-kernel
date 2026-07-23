//! Low-memory AP startup page and mode-transition assembly.

use core::{arch::global_asm, ptr};

use agent_kernel_x86_64::{
    apic::StartupVector,
    cpu::{
        ApStartupHandoff, AP_HANDOFF_APIC_ID_OFFSET, AP_HANDOFF_CPU_INDEX_OFFSET,
        AP_HANDOFF_CR3_OFFSET, AP_HANDOFF_ENTRY_OFFSET, AP_HANDOFF_GENERATION_OFFSET,
        AP_HANDOFF_PHYSICAL_OFFSET_OFFSET, AP_HANDOFF_STACK_TOP_OFFSET, AP_HANDOFF_STATUS_OFFSET,
        AP_STARTUP_STATUS_PREPARED,
    },
};
use bootloader_api::{info::MemoryRegionKind, BootInfo};

use crate::agent_memory::PHYSICAL_MEMORY_OFFSET;

use super::memory::{self, ApicMappingError};

const PAGE_BYTES: u64 = 4096;
const LOW_MEMORY_LIMIT: u64 = 0x10_0000;
const PROTECTED_MODE_OFFSET: usize = 0x100;
const LONG_MODE_OFFSET: usize = 0x200;
const GDT_OFFSET: usize = 0x600;
const GDTR_OFFSET: usize = 0x620;
const LONG_JUMP_OFFSET: usize = 0x628;
const HANDOFF_OFFSET: usize = 0x700;
const KERNEL_CODE32_SELECTOR: u16 = 0x08;
const KERNEL_DATA_SELECTOR: u16 = 0x10;
const KERNEL_CODE64_SELECTOR: u16 = 0x18;
const GDT_BYTES: usize = 4 * core::mem::size_of::<u64>();

global_asm!(
    r#"
    .section .text.agent_kernel_ap_trampoline,"ax",@progbits
    .balign 16

    .global agent_kernel_ap_trampoline_start
agent_kernel_ap_trampoline_start:
    .code16
    cli
    cld
    mov ax, cs
    mov ds, ax
    movzx ebx, ax
    shl ebx, 4
1:
    cmp dword ptr cs:[{handoff_status}], {status_prepared}
    je 2f
    pause
    jmp 1b
2:
    lgdt cs:[{gdtr_offset}]
    mov eax, cr0
    or eax, 1
    mov cr0, eax
    .byte 0x66, 0xea
    .long {protected_mode_offset}
    .word {code32_selector}

    .fill {protected_mode_offset} - (. - agent_kernel_ap_trampoline_start), 1, 0x90
agent_kernel_ap_trampoline_protected:
    .code32
    mov ax, {data_selector}
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov esp, ebx
    add esp, {page_bytes}

    mov eax, cr4
    or eax, (1 << 5) | (1 << 7) | (1 << 9) | (1 << 10)
    mov cr4, eax
    mov eax, dword ptr [ebx + {handoff_cr3}]
    mov cr3, eax
    mov ecx, 0xc0000080
    rdmsr
    or eax, (1 << 8) | (1 << 11)
    wrmsr
    mov eax, cr0
    and eax, ~(1 << 2)
    or eax, (1 << 1) | (1 << 5) | (1 << 16) | (1 << 31)
    mov cr0, eax
    .byte 0xff, 0xab
    .long {long_jump_offset}

    .fill {long_mode_offset} - (. - agent_kernel_ap_trampoline_start), 1, 0x90
agent_kernel_ap_trampoline_long:
    .code64
    mov ax, {data_selector}
    mov ds, ax
    mov es, ax
    mov ss, ax
    xor eax, eax
    mov fs, ax
    mov gs, ax

    mov rsp, qword ptr [rbx + {handoff_stack_top}]
    mov edi, dword ptr [rbx + {handoff_cpu_index}]
    mov rsi, qword ptr [rbx + {handoff_generation}]
    mov edx, dword ptr [rbx + {handoff_apic_id}]
    mov rcx, qword ptr [rbx + {handoff_physical_offset}]
    add rcx, rbx
    add rcx, {handoff_offset}
    mov rax, qword ptr [rbx + {handoff_entry}]
    sub rsp, 8
    mov qword ptr [rsp], 0
    jmp rax

    .global agent_kernel_ap_trampoline_end
agent_kernel_ap_trampoline_end:
    .code64
"#,
    protected_mode_offset = const PROTECTED_MODE_OFFSET,
    long_mode_offset = const LONG_MODE_OFFSET,
    gdtr_offset = const GDTR_OFFSET,
    long_jump_offset = const LONG_JUMP_OFFSET,
    handoff_offset = const HANDOFF_OFFSET,
    page_bytes = const PAGE_BYTES,
    code32_selector = const KERNEL_CODE32_SELECTOR,
    data_selector = const KERNEL_DATA_SELECTOR,
    handoff_status = const HANDOFF_OFFSET + AP_HANDOFF_STATUS_OFFSET,
    handoff_cpu_index = const HANDOFF_OFFSET + AP_HANDOFF_CPU_INDEX_OFFSET,
    handoff_apic_id = const HANDOFF_OFFSET + AP_HANDOFF_APIC_ID_OFFSET,
    handoff_generation = const HANDOFF_OFFSET + AP_HANDOFF_GENERATION_OFFSET,
    handoff_cr3 = const HANDOFF_OFFSET + AP_HANDOFF_CR3_OFFSET,
    handoff_stack_top = const HANDOFF_OFFSET + AP_HANDOFF_STACK_TOP_OFFSET,
    handoff_entry = const HANDOFF_OFFSET + AP_HANDOFF_ENTRY_OFFSET,
    handoff_physical_offset = const HANDOFF_OFFSET + AP_HANDOFF_PHYSICAL_OFFSET_OFFSET,
    status_prepared = const AP_STARTUP_STATUS_PREPARED,
);

unsafe extern "C" {
    static agent_kernel_ap_trampoline_start: u8;
    static agent_kernel_ap_trampoline_end: u8;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TrampolineError {
    MissingLowMemory,
    Mapping(ApicMappingError),
    AddressOverflow,
    ImageTooLarge,
    InvalidStartupVector,
    InvalidGdt,
}

pub(super) struct TrampolinePage {
    physical_address: u64,
    vector: StartupVector,
    handoff: &'static ApStartupHandoff,
}

impl TrampolinePage {
    pub(super) fn prepare(boot_info: &mut BootInfo) -> Result<Self, TrampolineError> {
        let physical_address =
            reserve_low_page(boot_info).ok_or(TrampolineError::MissingLowMemory)?;
        let vector = StartupVector::from_trampoline_address(physical_address)
            .ok_or(TrampolineError::InvalidStartupVector)?;
        memory::map_trampoline_page(boot_info, physical_address)
            .map_err(TrampolineError::Mapping)?;
        let virtual_address = PHYSICAL_MEMORY_OFFSET
            .checked_add(physical_address)
            .ok_or(TrampolineError::AddressOverflow)?;
        let page = virtual_address as *mut u8;

        let source_start = core::ptr::addr_of!(agent_kernel_ap_trampoline_start);
        let source_end = core::ptr::addr_of!(agent_kernel_ap_trampoline_end);
        let image_bytes = (source_end as usize)
            .checked_sub(source_start as usize)
            .ok_or(TrampolineError::ImageTooLarge)?;
        if image_bytes > LONG_MODE_OFFSET + 0x100 || HANDOFF_OFFSET + 128 > PAGE_BYTES as usize {
            return Err(TrampolineError::ImageTooLarge);
        }

        // SAFETY: the reserved frame is exclusive, writable through the fixed
        // physical alias, and does not overlap the linked source image.
        unsafe {
            ptr::write_bytes(page, 0, PAGE_BYTES as usize);
            ptr::copy_nonoverlapping(source_start, page, image_bytes);
            install_gdt(page, physical_address)?;
        }
        let handoff_pointer = unsafe { page.add(HANDOFF_OFFSET).cast::<ApStartupHandoff>() };
        // SAFETY: offset and page alignment satisfy the 64-byte handoff ABI and
        // the complete value fits inside this exclusive permanent frame.
        unsafe {
            handoff_pointer.write(ApStartupHandoff::new());
        }
        let handoff = unsafe { &*handoff_pointer };
        Ok(Self {
            physical_address,
            vector,
            handoff,
        })
    }

    pub(super) const fn physical_address(&self) -> u64 {
        self.physical_address
    }

    pub(super) const fn vector(&self) -> StartupVector {
        self.vector
    }

    pub(super) const fn handoff(&self) -> &'static ApStartupHandoff {
        self.handoff
    }
}

fn reserve_low_page(boot_info: &mut BootInfo) -> Option<u64> {
    for region in boot_info.memory_regions.iter_mut() {
        if region.kind != MemoryRegionKind::Usable {
            continue;
        }
        let candidate = align_up(region.start.max(PAGE_BYTES), PAGE_BYTES)?;
        let end = candidate.checked_add(PAGE_BYTES)?;
        if end <= region.end && end <= LOW_MEMORY_LIMIT {
            region.start = end;
            return Some(candidate);
        }
    }
    None
}

unsafe fn install_gdt(page: *mut u8, physical_address: u64) -> Result<(), TrampolineError> {
    let code32_base = u32::try_from(physical_address).map_err(|_| TrampolineError::InvalidGdt)?;
    let gdt_physical = physical_address
        .checked_add(GDT_OFFSET as u64)
        .and_then(|address| u32::try_from(address).ok())
        .ok_or(TrampolineError::InvalidGdt)?;
    let long_entry = physical_address
        .checked_add(LONG_MODE_OFFSET as u64)
        .and_then(|address| u32::try_from(address).ok())
        .ok_or(TrampolineError::InvalidGdt)?;
    let entries = [
        0,
        segment_descriptor(code32_base, 0x9a, 0xc),
        segment_descriptor(0, 0x92, 0xc),
        segment_descriptor(0, 0x9a, 0xa),
    ];

    // SAFETY: all fixed offsets are checked to fit the exclusive page.
    unsafe {
        ptr::copy_nonoverlapping(
            entries.as_ptr().cast::<u8>(),
            page.add(GDT_OFFSET),
            GDT_BYTES,
        );
        page.add(GDTR_OFFSET)
            .cast::<u16>()
            .write_unaligned((GDT_BYTES - 1) as u16);
        page.add(GDTR_OFFSET + 2)
            .cast::<u32>()
            .write_unaligned(gdt_physical);
        page.add(LONG_JUMP_OFFSET)
            .cast::<u32>()
            .write_unaligned(long_entry);
        page.add(LONG_JUMP_OFFSET + 4)
            .cast::<u16>()
            .write_unaligned(KERNEL_CODE64_SELECTOR);
    }
    Ok(())
}

fn segment_descriptor(base: u32, access: u8, flags: u8) -> u64 {
    let base = u64::from(base);
    0xffff
        | ((base & 0xffff) << 16)
        | (((base >> 16) & 0xff) << 32)
        | (u64::from(access) << 40)
        | (0xf << 48)
        | (u64::from(flags & 0xf) << 52)
        | (((base >> 24) & 0xff) << 56)
}

fn align_up(value: u64, alignment: u64) -> Option<u64> {
    value
        .checked_add(alignment.checked_sub(1)?)?
        .checked_div(alignment)?
        .checked_mul(alignment)
}
