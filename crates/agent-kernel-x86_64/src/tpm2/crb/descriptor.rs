//! Validation for TPM-provided CRB command and response descriptors.
//!
//! V19 supports the PC Client locality-zero data window only. Descriptor
//! addresses may share that window, but may never overlap control registers
//! or escape the single boot-owned MMIO page.

const DATA_BUFFER_OFFSET: u64 = 0x80;
pub(super) const LOCALITY_BYTES: u64 = 4096;
const TPM_HEADER_SIZE: usize = 10;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(super) struct BufferDescriptor {
    pub(super) command_size: usize,
    pub(super) command_address: u64,
    pub(super) response_size: usize,
    pub(super) response_address: u64,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(super) enum BufferDescriptorError {
    InvalidCommand,
    InvalidResponse,
}

impl BufferDescriptor {
    pub(super) fn new(
        locality_base: u64,
        command_size: u32,
        command_address: u64,
        response_size: u32,
        response_address: u64,
    ) -> Result<Self, BufferDescriptorError> {
        let descriptor = Self {
            command_size: command_size as usize,
            command_address,
            response_size: response_size as usize,
            response_address,
        };
        if descriptor.command_size == 0
            || !range_is_in_data_window(
                locality_base,
                descriptor.command_address,
                descriptor.command_size,
            )
        {
            return Err(BufferDescriptorError::InvalidCommand);
        }
        if descriptor.response_size < TPM_HEADER_SIZE
            || !range_is_in_data_window(
                locality_base,
                descriptor.response_address,
                descriptor.response_size,
            )
        {
            return Err(BufferDescriptorError::InvalidResponse);
        }
        Ok(descriptor)
    }
}

fn range_is_in_data_window(locality_base: u64, address: u64, length: usize) -> bool {
    let Some(start) = locality_base.checked_add(DATA_BUFFER_OFFSET) else {
        return false;
    };
    let Some(limit) = locality_base.checked_add(LOCALITY_BYTES) else {
        return false;
    };
    let Ok(length) = u64::try_from(length) else {
        return false;
    };
    address >= start && address.checked_add(length).is_some_and(|end| end <= limit)
}
