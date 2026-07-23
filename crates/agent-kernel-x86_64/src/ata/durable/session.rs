//! Boot-owned ATA durable archive session and recovered-head binding.
//!
//! The session identifies one device, maps its reserved slot range, scans both
//! slots under a fixed State Signer policy, and retains caller-owned buffers for
//! later commits. It performs no private-key operation or Core mutation.

mod commit;
mod config;
mod error;

pub use commit::NativeAtaDurableCommitError;
pub use config::{NativeAtaDurableConfig, NativeAtaDurableConfigError};
pub use error::NativeAtaDurableInitError;

use agent_kernel_core::{
    DurableRecoveredHead, DurableRecoveryError, MAX_DURABLE_ARCHIVE_PAYLOAD_BYTES,
};
use agent_kernel_hal::DURABLE_SLOT_BYTES;

use super::{AtaDurableBinding, AtaDurableHead, AtaDurableStateBackend};
use crate::{
    ata::{AtaBlockDevice, AtaPioDevice, AtaRegisterIo},
    durable_state::{
        recover_durable_archive, DurableArchiveRecoveryError, DurableStateTrustPolicy,
        VerifiedDurableArchiveRecovery,
    },
};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum NativeAtaDurableBootState {
    Genesis,
    Recovered(u64),
}

pub struct NativeAtaDurableSession<'a, D> {
    config: NativeAtaDurableConfig,
    backend: AtaDurableStateBackend<'a, D>,
    scratch: &'a mut [u8; DURABLE_SLOT_BYTES],
    payload: &'a mut [u8; MAX_DURABLE_ARCHIVE_PAYLOAD_BYTES],
    recovery: Option<VerifiedDurableArchiveRecovery>,
}

impl NativeAtaDurableConfig {
    pub fn initialize<'a, I: AtaRegisterIo>(
        self,
        io: I,
        staging: &'a mut [u8; DURABLE_SLOT_BYTES],
        scratch: &'a mut [u8; DURABLE_SLOT_BYTES],
        payload: &'a mut [u8; MAX_DURABLE_ARCHIVE_PAYLOAD_BYTES],
    ) -> Result<NativeAtaDurableSession<'a, AtaPioDevice<I>>, NativeAtaDurableInitError> {
        let mut device = AtaPioDevice::new(self.pio(), io);
        device
            .identify()
            .map_err(NativeAtaDurableInitError::Identify)?;
        self.initialize_device(device, staging, scratch, payload)
    }

    pub fn initialize_device<'a, D: AtaBlockDevice>(
        self,
        device: D,
        staging: &'a mut [u8; DURABLE_SLOT_BYTES],
        scratch: &'a mut [u8; DURABLE_SLOT_BYTES],
        payload: &'a mut [u8; MAX_DURABLE_ARCHIVE_PAYLOAD_BYTES],
    ) -> Result<NativeAtaDurableSession<'a, D>, NativeAtaDurableInitError> {
        NativeAtaDurableSession::initialize(self, device, staging, scratch, payload)
    }
}

impl<'a, D: AtaBlockDevice> NativeAtaDurableSession<'a, D> {
    fn initialize(
        config: NativeAtaDurableConfig,
        device: D,
        staging: &'a mut [u8; DURABLE_SLOT_BYTES],
        scratch: &'a mut [u8; DURABLE_SLOT_BYTES],
        payload: &'a mut [u8; MAX_DURABLE_ARCHIVE_PAYLOAD_BYTES],
    ) -> Result<Self, NativeAtaDurableInitError> {
        let identity = device.identity().ok_or(NativeAtaDurableInitError::Backend(
            super::AtaDurableBackendInitError::DeviceNotIdentified,
        ))?;
        let binding = AtaDurableBinding::new(config.storage(), config.base_lba(), identity)
            .map_err(NativeAtaDurableInitError::Binding)?;
        let mut backend = AtaDurableStateBackend::new(device, binding, staging)
            .map_err(NativeAtaDurableInitError::Backend)?;
        let signer = config.signer();
        let policy = DurableStateTrustPolicy::new(
            core::slice::from_ref(&signer),
            config.policy_generation(),
        );
        let recovery =
            match recover_durable_archive(&mut backend, policy, config.storage(), scratch) {
                Ok(head) => {
                    backend
                        .bind_head(AtaDurableHead::Recovered(head.generation()))
                        .map_err(NativeAtaDurableInitError::Head)?;
                    Some(VerifiedDurableArchiveRecovery::new(head))
                }
                Err(DurableArchiveRecoveryError::Selection(
                    DurableRecoveryError::NoCommittedSlot,
                )) => {
                    backend
                        .bind_head(AtaDurableHead::Genesis)
                        .map_err(NativeAtaDurableInitError::Head)?;
                    None
                }
                Err(error) => return Err(NativeAtaDurableInitError::Recovery(error)),
            };
        Ok(Self {
            config,
            backend,
            scratch,
            payload,
            recovery,
        })
    }

    pub const fn config(&self) -> NativeAtaDurableConfig {
        self.config
    }

    pub const fn backend(&self) -> &AtaDurableStateBackend<'a, D> {
        &self.backend
    }

    pub fn backend_mut(&mut self) -> &mut AtaDurableStateBackend<'a, D> {
        &mut self.backend
    }

    pub fn boot_state(&self) -> NativeAtaDurableBootState {
        self.recovered_head()
            .map(|head| NativeAtaDurableBootState::Recovered(head.generation()))
            .unwrap_or(NativeAtaDurableBootState::Genesis)
    }

    pub fn recovered_head(&self) -> Option<DurableRecoveredHead> {
        self.recovery
            .as_ref()
            .map(VerifiedDurableArchiveRecovery::head)
    }

    pub const fn recovery_verifier(&self) -> Option<&VerifiedDurableArchiveRecovery> {
        self.recovery.as_ref()
    }

    pub fn recovery_verifier_mut(&mut self) -> Option<&mut VerifiedDurableArchiveRecovery> {
        self.recovery.as_mut()
    }
}
