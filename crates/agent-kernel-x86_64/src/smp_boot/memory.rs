//! Supervisor-only device mappings in the active kernel address space.

use agent_kernel_x86_64::acpi_topology::IoApicDescriptor;
use bootloader_api::BootInfo;
use x86_64::{
    registers::control::Cr3,
    structures::paging::{
        mapper::TranslateError, Mapper, OffsetPageTable, Page, PageTable, PageTableFlags,
        PhysFrame, Size4KiB,
    },
    PhysAddr, VirtAddr,
};

use crate::agent_memory::{BootFrameAllocator, PHYSICAL_MEMORY_OFFSET};

const DEVICE_FLAGS: PageTableFlags = PageTableFlags::PRESENT
    .union(PageTableFlags::WRITABLE)
    .union(PageTableFlags::NO_EXECUTE)
    .union(PageTableFlags::NO_CACHE)
    .union(PageTableFlags::WRITE_THROUGH);
const TRAMPOLINE_FLAGS: PageTableFlags = PageTableFlags::PRESENT.union(PageTableFlags::WRITABLE);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ApicMappingError {
    MissingPhysicalMap,
    UnexpectedPhysicalOffset,
    AddressOverflow,
    InvalidPage,
    MappingConflict,
    ParentHugePage,
    FrameAllocationFailed,
    FlagUpdateFailed,
}

pub(super) fn map_apic_pages(
    boot_info: &mut BootInfo,
    local_apic: u64,
    io_apics: &[IoApicDescriptor],
) -> Result<(), ApicMappingError> {
    let physical_offset = boot_info
        .physical_memory_offset
        .into_option()
        .ok_or(ApicMappingError::MissingPhysicalMap)?;
    if physical_offset != PHYSICAL_MEMORY_OFFSET {
        return Err(ApicMappingError::UnexpectedPhysicalOffset);
    }

    // SAFETY: the BSP owns the active kernel page tables with IF clear and no
    // AP has been started. The bootloader direct map covers every page-table
    // frame returned through CR3 and BootFrameAllocator.
    let mut mapper = unsafe { active_mapper(physical_offset)? };
    let mut allocator = BootFrameAllocator::new(&mut boot_info.memory_regions);
    map_device_page(&mut mapper, &mut allocator, physical_offset, local_apic)?;
    for descriptor in io_apics {
        map_device_page(
            &mut mapper,
            &mut allocator,
            physical_offset,
            descriptor.address(),
        )?;
    }
    Ok(())
}

pub(super) fn map_tpm_crb_page(
    boot_info: &mut BootInfo,
    locality_base: u64,
) -> Result<(), ApicMappingError> {
    let physical_offset = boot_info
        .physical_memory_offset
        .into_option()
        .ok_or(ApicMappingError::MissingPhysicalMap)?;
    if physical_offset != PHYSICAL_MEMORY_OFFSET {
        return Err(ApicMappingError::UnexpectedPhysicalOffset);
    }
    // SAFETY: the BSP owns the active root before any application processor
    // starts and the direct map covers this firmware-described device page.
    let mut mapper = unsafe { active_mapper(physical_offset)? };
    let mut allocator = BootFrameAllocator::new(&mut boot_info.memory_regions);
    map_device_page(&mut mapper, &mut allocator, physical_offset, locality_base)
}

pub(super) fn map_trampoline_page(
    boot_info: &mut BootInfo,
    physical_address: u64,
) -> Result<(), ApicMappingError> {
    let physical_offset = boot_info
        .physical_memory_offset
        .into_option()
        .ok_or(ApicMappingError::MissingPhysicalMap)?;
    if physical_offset != PHYSICAL_MEMORY_OFFSET {
        return Err(ApicMappingError::UnexpectedPhysicalOffset);
    }
    // SAFETY: the BSP remains the sole active processor and owns this root.
    let mut mapper = unsafe { active_mapper(physical_offset)? };
    let mut allocator = BootFrameAllocator::new(&mut boot_info.memory_regions);
    map_page(
        &mut mapper,
        &mut allocator,
        physical_address,
        physical_address,
        TRAMPOLINE_FLAGS,
    )
}

unsafe fn active_mapper(
    physical_offset: u64,
) -> Result<OffsetPageTable<'static>, ApicMappingError> {
    let (root_frame, _) = Cr3::read();
    let root_virtual = physical_offset
        .checked_add(root_frame.start_address().as_u64())
        .ok_or(ApicMappingError::AddressOverflow)?;
    let root_pointer = root_virtual as *mut PageTable;
    // SAFETY: caller owns the active root and the fixed direct map translates
    // its physical frame to this permanent supervisor virtual address.
    Ok(unsafe { OffsetPageTable::new(&mut *root_pointer, VirtAddr::new(physical_offset)) })
}

fn map_device_page(
    mapper: &mut OffsetPageTable<'_>,
    allocator: &mut BootFrameAllocator<'_>,
    physical_offset: u64,
    physical_address: u64,
) -> Result<(), ApicMappingError> {
    let virtual_address = physical_offset
        .checked_add(physical_address)
        .ok_or(ApicMappingError::AddressOverflow)?;
    map_page(
        mapper,
        allocator,
        virtual_address,
        physical_address,
        DEVICE_FLAGS,
    )
}

fn map_page(
    mapper: &mut OffsetPageTable<'_>,
    allocator: &mut BootFrameAllocator<'_>,
    virtual_address: u64,
    physical_address: u64,
    flags: PageTableFlags,
) -> Result<(), ApicMappingError> {
    let frame = PhysFrame::<Size4KiB>::from_start_address(PhysAddr::new(physical_address))
        .map_err(|_| ApicMappingError::InvalidPage)?;
    let page = Page::<Size4KiB>::from_start_address(
        VirtAddr::try_new(virtual_address).map_err(|_| ApicMappingError::AddressOverflow)?,
    )
    .map_err(|_| ApicMappingError::InvalidPage)?;

    match mapper.translate_page(page) {
        Ok(existing) if existing == frame => {
            // SAFETY: this canonical physical-window alias names a device page;
            // no user mapping or executable code may depend on weaker flags.
            unsafe { mapper.update_flags(page, flags) }
                .map_err(|_| ApicMappingError::FlagUpdateFailed)?
                .flush();
        }
        Ok(_) => return Err(ApicMappingError::MappingConflict),
        Err(TranslateError::ParentEntryHugePage) => {
            return Err(ApicMappingError::ParentHugePage);
        }
        Err(TranslateError::InvalidFrameAddress(_)) => {
            return Err(ApicMappingError::MappingConflict);
        }
        Err(TranslateError::PageNotMapped) => {
            // SAFETY: the physical page is exclusive device MMIO, this
            // virtual page belongs to the physical window, and allocated table
            // frames are removed permanently from BootInfo Usable regions.
            unsafe { mapper.map_to(page, frame, flags, allocator) }
                .map_err(|_| ApicMappingError::FrameAllocationFailed)?
                .flush();
        }
    }
    Ok(())
}
