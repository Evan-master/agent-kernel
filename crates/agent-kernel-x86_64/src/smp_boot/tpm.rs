//! BSP-owned TPM CRB device mapping.
//!
//! This boot adapter consumes strict ACPI discovery, installs one uncached
//! supervisor mapping, and transfers one-shot MMIO ownership to the signer.

use agent_kernel_x86_64::tpm2::{Tpm2AcpiTable, VolatileCrbIo};
use bootloader_api::BootInfo;

use crate::agent_memory::PHYSICAL_MEMORY_OFFSET;

use super::{memory, SmpBootError, SmpBootstrap};

const CRB_LOCALITY_BYTES: usize = 4096;

impl SmpBootstrap {
    pub(crate) fn prepare_tpm_crb_io(
        &mut self,
        boot_info: &mut BootInfo,
    ) -> Result<(Tpm2AcpiTable, VolatileCrbIo), SmpBootError> {
        if self.tpm_crb_prepared {
            return Err(SmpBootError::TpmCrbAlreadyPrepared);
        }
        let table = self
            .tpm2_table
            .map_err(SmpBootError::Tpm2Acpi)?
            .ok_or(SmpBootError::MissingTpm2Table)?;
        let locality_base = table.locality_base();
        memory::map_tpm_crb_page(boot_info, locality_base).map_err(SmpBootError::ApicMapping)?;
        let virtual_base = PHYSICAL_MEMORY_OFFSET
            .checked_add(locality_base)
            .ok_or(SmpBootError::InvalidTpmCrbMapping)?;
        // SAFETY: map_tpm_crb_page installed this exact uncached,
        // supervisor-only physical-window alias and SmpBootstrap grants one
        // transaction owner for the remainder of boot.
        let io = unsafe { VolatileCrbIo::new(locality_base, virtual_base, CRB_LOCALITY_BYTES) }
            .map_err(|_| SmpBootError::InvalidTpmCrbMapping)?;
        self.tpm_crb_prepared = true;
        Ok((table, io))
    }
}
