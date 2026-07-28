//! QEMU EDU PCI DMA engine adapter.
//!
//! This architecture module owns the EDU MMIO register contract used by the
//! native VT-d proof. PCI discovery, BAR mapping, and semantic DMA authority
//! remain with their respective owners.

use core::ptr;

pub const EDU_VENDOR_ID: u16 = 0x1234;
pub const EDU_DEVICE_ID: u16 = 0x11e8;
pub const EDU_IDENTITY_REG: u16 = 0x00;
pub const EDU_INTERRUPT_STATUS_REG: u16 = 0x24;
pub const EDU_INTERRUPT_ACK_REG: u16 = 0x64;
pub const EDU_SOURCE_REG: u16 = 0x80;
pub const EDU_DESTINATION_REG: u16 = 0x88;
pub const EDU_COUNT_REG: u16 = 0x90;
pub const EDU_COMMAND_REG: u16 = 0x98;
pub const EDU_MMIO_BYTES: u64 = 1 << 20;
pub const EDU_DEVICE_BUFFER: u64 = 0x40000;
pub const EDU_DEVICE_BUFFER_BYTES: u64 = 4096;
pub const EDU_DMA_INTERRUPT: u32 = 0x100;

const EDU_IDENTITY: u32 = 0x0100_00ed;
const DMA_RUN: u64 = 1;
const DMA_TO_DEVICE: u64 = 0;
const DMA_FROM_DEVICE: u64 = 1 << 1;
const DMA_INTERRUPT_REQUEST: u64 = 1 << 2;

pub trait EduRegisterIo {
    fn read_u32(&mut self, offset: u16) -> u32;
    fn read_u64(&mut self, offset: u16) -> u64;
    fn write_u32(&mut self, offset: u16, value: u32);
    fn write_u64(&mut self, offset: u16, value: u64);
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum EduDmaError {
    InvalidPollBudget,
    IdentityMismatch(u32),
    TransferBusy,
    TransferLengthInvalid,
    DeviceBufferOutOfRange,
    AddressOverflow,
    CompletionTimeout,
    DmaInterruptMissing(u32),
    InterruptAcknowledgeFailed(u32),
}

pub struct EduDma<I> {
    io: I,
    poll_budget: u32,
}

impl<I: EduRegisterIo> EduDma<I> {
    pub fn bind(mut io: I, poll_budget: u32) -> Result<Self, EduDmaError> {
        if poll_budget == 0 {
            return Err(EduDmaError::InvalidPollBudget);
        }
        let identity = io.read_u32(EDU_IDENTITY_REG);
        if identity != EDU_IDENTITY {
            return Err(EduDmaError::IdentityMismatch(identity));
        }
        Ok(Self { io, poll_budget })
    }

    pub fn copy_memory_to_device(
        &mut self,
        memory_iova: u64,
        device_offset: u64,
        length: u64,
    ) -> Result<(), EduDmaError> {
        validate_transfer(memory_iova, device_offset, length)?;
        self.start(memory_iova, device_offset, length, DMA_TO_DEVICE)
    }

    pub fn copy_device_to_memory(
        &mut self,
        device_offset: u64,
        memory_iova: u64,
        length: u64,
    ) -> Result<(), EduDmaError> {
        validate_transfer(memory_iova, device_offset, length)?;
        self.start(device_offset, memory_iova, length, DMA_FROM_DEVICE)
    }

    pub fn copy_memory_to_device_interrupting(
        &mut self,
        memory_iova: u64,
        device_offset: u64,
        length: u64,
    ) -> Result<(), EduDmaError> {
        validate_transfer(memory_iova, device_offset, length)?;
        self.start(
            memory_iova,
            device_offset,
            length,
            DMA_TO_DEVICE | DMA_INTERRUPT_REQUEST,
        )
    }

    pub fn acknowledge_dma_interrupt(&mut self) -> Result<u32, EduDmaError> {
        let status = self.io.read_u32(EDU_INTERRUPT_STATUS_REG);
        if status & EDU_DMA_INTERRUPT == 0 {
            return Err(EduDmaError::DmaInterruptMissing(status));
        }
        self.io.write_u32(EDU_INTERRUPT_ACK_REG, EDU_DMA_INTERRUPT);
        let remaining = self.io.read_u32(EDU_INTERRUPT_STATUS_REG);
        if remaining & EDU_DMA_INTERRUPT != 0 {
            return Err(EduDmaError::InterruptAcknowledgeFailed(remaining));
        }
        Ok(status)
    }

    pub fn into_io(self) -> I {
        self.io
    }

    fn start(
        &mut self,
        source: u64,
        destination: u64,
        length: u64,
        direction: u64,
    ) -> Result<(), EduDmaError> {
        if self.io.read_u64(EDU_COMMAND_REG) & DMA_RUN != 0 {
            return Err(EduDmaError::TransferBusy);
        }
        self.io.write_u64(EDU_SOURCE_REG, source);
        self.io.write_u64(EDU_DESTINATION_REG, destination);
        self.io.write_u64(EDU_COUNT_REG, length);
        self.io.write_u64(EDU_COMMAND_REG, DMA_RUN | direction);
        for _ in 0..self.poll_budget {
            if self.io.read_u64(EDU_COMMAND_REG) & DMA_RUN == 0 {
                return Ok(());
            }
            core::hint::spin_loop();
        }
        Err(EduDmaError::CompletionTimeout)
    }
}

fn validate_transfer(memory_iova: u64, device_offset: u64, length: u64) -> Result<(), EduDmaError> {
    if length == 0 || length > EDU_DEVICE_BUFFER_BYTES {
        return Err(EduDmaError::TransferLengthInvalid);
    }
    let device_end = device_offset
        .checked_add(length)
        .ok_or(EduDmaError::AddressOverflow)?;
    let buffer_end = EDU_DEVICE_BUFFER + EDU_DEVICE_BUFFER_BYTES;
    if device_offset < EDU_DEVICE_BUFFER || device_end > buffer_end {
        return Err(EduDmaError::DeviceBufferOutOfRange);
    }
    memory_iova
        .checked_add(length)
        .ok_or(EduDmaError::AddressOverflow)?;
    Ok(())
}

pub struct VolatileEduMmio {
    base: *mut u8,
}

impl VolatileEduMmio {
    /// Bind one mapped EDU BAR0 window.
    ///
    /// # Safety
    ///
    /// `base` must remain a writable, uncached mapping of EDU's first 4 KiB
    /// register page while this value exists.
    pub unsafe fn new(base: *mut u8) -> Option<Self> {
        if base.is_null() || !(base as usize).is_multiple_of(4096) {
            None
        } else {
            Some(Self { base })
        }
    }
}

impl EduRegisterIo for VolatileEduMmio {
    fn read_u32(&mut self, offset: u16) -> u32 {
        // SAFETY: construction binds the complete naturally aligned BAR.
        unsafe { ptr::read_volatile(self.base.add(usize::from(offset)).cast::<u32>()) }
    }

    fn read_u64(&mut self, offset: u16) -> u64 {
        // SAFETY: construction binds the complete naturally aligned BAR.
        unsafe { ptr::read_volatile(self.base.add(usize::from(offset)).cast::<u64>()) }
    }

    fn write_u32(&mut self, offset: u16, value: u32) {
        // SAFETY: construction binds the complete naturally aligned BAR.
        unsafe {
            ptr::write_volatile(self.base.add(usize::from(offset)).cast::<u32>(), value);
        }
    }

    fn write_u64(&mut self, offset: u16, value: u64) {
        // SAFETY: construction binds the complete naturally aligned BAR.
        unsafe {
            ptr::write_volatile(self.base.add(usize::from(offset)).cast::<u64>(), value);
        }
    }
}
