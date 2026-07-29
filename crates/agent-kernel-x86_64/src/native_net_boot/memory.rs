//! Exclusive VT-d and Virtio network DMA pages for the V29 proof.
//!
//! Five frames hold translation tables. Four disjoint frames hold Rx/Tx
//! metadata and packet buffers, all removed from the boot allocator first.

use agent_kernel_x86_64::iommu::{VtdLegacyTableAddresses, VtdLegacyTablePages, VtdTableError};
use bootloader_api::BootInfo;
use x86_64::{structures::paging::PhysFrame, PhysAddr};

use crate::agent_memory::{BootFrameAllocator, PHYSICAL_MEMORY_OFFSET};

const PAGE_BYTES: usize = 4096;
const FRAME_COUNT: usize = 9;
const RX_METADATA_FRAME: usize = 5;
const RX_PACKET_FRAME: usize = 6;
const TX_METADATA_FRAME: usize = 7;
const TX_PACKET_FRAME: usize = 8;

pub(super) struct NativeNetPages {
    frames: [u64; FRAME_COUNT],
}

impl NativeNetPages {
    pub(super) const fn rx_metadata_physical(&self) -> u64 {
        self.frames[RX_METADATA_FRAME]
    }

    pub(super) const fn rx_packet_physical(&self) -> u64 {
        self.frames[RX_PACKET_FRAME]
    }

    pub(super) const fn tx_metadata_physical(&self) -> u64 {
        self.frames[TX_METADATA_FRAME]
    }

    pub(super) const fn tx_packet_physical(&self) -> u64 {
        self.frames[TX_PACKET_FRAME]
    }

    pub(super) fn rx_metadata_pointer(&self) -> *mut [u8; PAGE_BYTES] {
        physical_pointer(self.rx_metadata_physical()).cast()
    }

    pub(super) fn rx_packet_pointer(&self) -> *mut [u8; PAGE_BYTES] {
        physical_pointer(self.rx_packet_physical()).cast()
    }

    pub(super) fn tx_metadata_pointer(&self) -> *mut [u8; PAGE_BYTES] {
        physical_pointer(self.tx_metadata_physical()).cast()
    }

    pub(super) fn tx_packet_pointer(&self) -> *mut [u8; PAGE_BYTES] {
        physical_pointer(self.tx_packet_physical()).cast()
    }

    pub(super) fn table_pages(&mut self) -> Result<VtdLegacyTablePages<'_>, VtdTableError> {
        let addresses = VtdLegacyTableAddresses::new(
            self.frames[0],
            self.frames[1],
            self.frames[2],
            self.frames[3],
            self.frames[4],
        )?;
        // SAFETY: allocation removed nine distinct frames. The first five
        // remain exclusively owned by this table set.
        unsafe {
            Ok(VtdLegacyTablePages::new(
                table_pointer(self.frames[0]),
                table_pointer(self.frames[1]),
                table_pointer(self.frames[2]),
                table_pointer(self.frames[3]),
                table_pointer(self.frames[4]),
                addresses,
            ))
        }
    }
}

pub(super) fn allocate(boot_info: &mut BootInfo) -> Option<NativeNetPages> {
    if boot_info.physical_memory_offset.into_option()? != PHYSICAL_MEMORY_OFFSET {
        return None;
    }
    let zero = PhysFrame::from_start_address(PhysAddr::new(0)).ok()?;
    let mut allocated = [zero; FRAME_COUNT];
    let mut allocator = BootFrameAllocator::new(&mut boot_info.memory_regions);
    for frame in &mut allocated {
        *frame = allocator.allocate()?;
    }
    let mut frames = [0; FRAME_COUNT];
    for (index, frame) in allocated.iter().enumerate() {
        frames[index] = frame.start_address().as_u64();
        // SAFETY: each frame was removed from a Usable region and is now
        // exclusively owned by this boot profile.
        unsafe {
            core::ptr::write_bytes(physical_pointer(frames[index]), 0, PAGE_BYTES);
        }
    }
    Some(NativeNetPages { frames })
}

fn physical_pointer(physical: u64) -> *mut u8 {
    (PHYSICAL_MEMORY_OFFSET + physical) as *mut u8
}

unsafe fn table_pointer(physical: u64) -> &'static mut [u64; 512] {
    // SAFETY: caller guarantees one exclusive aligned table frame.
    unsafe { &mut *physical_pointer(physical).cast::<[u64; 512]>() }
}
