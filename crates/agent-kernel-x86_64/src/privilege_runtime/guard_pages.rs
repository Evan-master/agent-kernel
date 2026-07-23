//! Early BSP removal of every per-CPU privileged-stack guard mapping.
//!
//! This architecture-binary child edits the shared kernel root before AP startup.
//! Guard frames remain reserved by the kernel image; only their virtual mappings
//! are removed, and every usable stack boundary is revalidated after mutation.

use core::{
    arch::asm,
    sync::atomic::{AtomicU8, Ordering},
};

use agent_kernel_x86_64::cpu::CpuIndex;
use bootloader_api::BootInfo;
use x86_64::{
    registers::control::Cr3,
    structures::paging::{
        mapper::{Translate, TranslateResult, UnmapError},
        Mapper, OffsetPageTable, Page, PageTable, Size4KiB,
    },
    VirtAddr,
};

use super::PRIVILEGE_SLOTS;
use crate::agent_memory::PHYSICAL_MEMORY_OFFSET;

const UNPREPARED: u8 = 0;
const PREPARING: u8 = 1;
const READY: u8 = 2;
const FAILED: u8 = 3;

static PREPARATION_STATE: AtomicU8 = AtomicU8::new(UNPREPARED);

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum GuardPageError {
    AlreadyPrepared,
    MissingPhysicalMap,
    UnexpectedPhysicalOffset,
    AddressOverflow,
    InvalidStackLayout(CpuIndex),
    GuardPageNotMapped(CpuIndex),
    ParentHugePage(CpuIndex),
    InvalidFrameAddress(CpuIndex),
    StackPageNotMapped(CpuIndex),
    GuardPageStillMapped(CpuIndex),
}

pub(super) fn prepare(boot_info: &BootInfo) -> Result<(), GuardPageError> {
    // SAFETY: guard-page mutation runs on the BSP before interrupt gates or APs
    // become active and keeps IF clear for the remaining early boot sequence.
    unsafe {
        asm!("cli", options(nomem, nostack));
    }
    PREPARATION_STATE
        .compare_exchange(UNPREPARED, PREPARING, Ordering::AcqRel, Ordering::Acquire)
        .map_err(|_| GuardPageError::AlreadyPrepared)?;
    let result = prepare_inner(boot_info);
    PREPARATION_STATE.store(
        if result.is_ok() { READY } else { FAILED },
        Ordering::Release,
    );
    result
}

pub(super) fn ready() -> bool {
    PREPARATION_STATE.load(Ordering::Acquire) == READY
}

fn prepare_inner(boot_info: &BootInfo) -> Result<(), GuardPageError> {
    let physical_offset = boot_info
        .physical_memory_offset
        .into_option()
        .ok_or(GuardPageError::MissingPhysicalMap)?;
    if physical_offset != PHYSICAL_MEMORY_OFFSET {
        return Err(GuardPageError::UnexpectedPhysicalOffset);
    }
    // SAFETY: kernel entry owns the active root, APs have not started, IF is
    // cleared, and the bootloader direct map reaches every page-table frame.
    let mut mapper = unsafe { active_mapper(physical_offset)? };

    for (raw, slot) in PRIVILEGE_SLOTS.iter().enumerate() {
        let cpu = CpuIndex::new(raw as u16).ok_or(GuardPageError::AddressOverflow)?;
        let layout = slot
            .stack
            .layout()
            .ok_or(GuardPageError::InvalidStackLayout(cpu))?;
        require_mapped(&mapper, layout.guard_start(), cpu, true)?;
        require_mapped(&mapper, layout.stack_start(), cpu, false)?;
        require_mapped(&mapper, layout.stack_end() - 1, cpu, false)?;

        let address = VirtAddr::try_new(layout.guard_start() as u64)
            .map_err(|_| GuardPageError::AddressOverflow)?;
        let page = Page::<Size4KiB>::from_start_address(address)
            .map_err(|_| GuardPageError::InvalidStackLayout(cpu))?;
        let (_, flush) = mapper.unmap(page).map_err(|error| match error {
            UnmapError::PageNotMapped => GuardPageError::GuardPageNotMapped(cpu),
            UnmapError::ParentEntryHugePage => GuardPageError::ParentHugePage(cpu),
            UnmapError::InvalidFrameAddress(_) => GuardPageError::InvalidFrameAddress(cpu),
        })?;
        flush.flush();

        if !matches!(mapper.translate(address), TranslateResult::NotMapped) {
            return Err(GuardPageError::GuardPageStillMapped(cpu));
        }
        require_mapped(&mapper, layout.stack_start(), cpu, false)?;
        require_mapped(&mapper, layout.stack_end() - 1, cpu, false)?;
    }
    Ok(())
}

unsafe fn active_mapper(physical_offset: u64) -> Result<OffsetPageTable<'static>, GuardPageError> {
    let (root, _) = Cr3::read();
    let root_virtual = physical_offset
        .checked_add(root.start_address().as_u64())
        .ok_or(GuardPageError::AddressOverflow)?;
    let root_pointer = root_virtual as *mut PageTable;
    // SAFETY: the fixed direct map permanently maps the active root frame.
    Ok(unsafe { OffsetPageTable::new(&mut *root_pointer, VirtAddr::new(physical_offset)) })
}

fn require_mapped(
    mapper: &OffsetPageTable<'_>,
    address: usize,
    cpu: CpuIndex,
    guard: bool,
) -> Result<(), GuardPageError> {
    let address = VirtAddr::try_new(address as u64).map_err(|_| GuardPageError::AddressOverflow)?;
    match mapper.translate(address) {
        TranslateResult::Mapped { .. } => Ok(()),
        TranslateResult::NotMapped if guard => Err(GuardPageError::GuardPageNotMapped(cpu)),
        TranslateResult::NotMapped => Err(GuardPageError::StackPageNotMapped(cpu)),
        TranslateResult::InvalidFrameAddress(_) => Err(GuardPageError::InvalidFrameAddress(cpu)),
    }
}
