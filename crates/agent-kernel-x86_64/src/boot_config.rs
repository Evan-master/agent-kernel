//! Bootloader memory contract for the x86_64 kernel image.
//!
//! This architecture-binary module fixes the guarded kernel-stack size and
//! supervisor physical-memory window required by the Agent page mapper.

use bootloader_api::{config::Mapping, BootloaderConfig};

use crate::agent_memory::PHYSICAL_MEMORY_OFFSET;

const KERNEL_STACK_SIZE: u64 = 1024 * 1024;

pub(crate) static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.kernel_stack_size = KERNEL_STACK_SIZE;
    config.mappings.physical_memory = Some(Mapping::FixedAddress(PHYSICAL_MEMORY_OFFSET));
    config
};
