//! Bootloader memory contract for the x86_64 kernel image.
//!
//! This architecture-binary module fixes the guarded kernel-stack size and
//! supervisor physical-memory window required by the Agent page mapper.

use agent_kernel_core::{
    AgentId, CapabilityId, DurableStatePublicKey, DurableStateSignerRecord,
    DurableStateSignerStatus, ResourceId,
};
use agent_kernel_x86_64::{
    ata::{AtaDrive, AtaPioConfig, NativeAtaDurableConfig},
    native_durable_boot::NativeDurableStorageProfile,
    native_tpm_boot::NativeTpmSignerProfile,
    tpm2::{DigestSignCommand, ProvisionedTpmSignerConfig, Sha256PcrPolicy, TpmPersistentHandle},
};
use bootloader_api::{config::Mapping, BootloaderConfig};

use crate::agent_memory::PHYSICAL_MEMORY_OFFSET;

include!(concat!(env!("OUT_DIR"), "/qemu_durable_profile.rs"));

pub(crate) const KERNEL_STACK_SIZE: u64 = 8 * 1024 * 1024;
const ATA_POLL_BUDGET: u32 = 10_000_000;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum DurableProofRole {
    Disabled,
    Writer,
    Recovery,
}

#[derive(Copy, Clone)]
pub(crate) struct StateSignerBootProfile {
    package: &'static [u8],
    public_key: [u8; 32],
    agent: AgentId,
    archive_authority: CapabilityId,
    storage_authority: CapabilityId,
    nonce: u64,
    through_sequence: u64,
    call_data_generation: u64,
    return_offsets: [u32; 6],
}

pub(crate) static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.kernel_stack_size = KERNEL_STACK_SIZE;
    config.mappings.physical_memory = Some(Mapping::FixedAddress(PHYSICAL_MEMORY_OFFSET));
    config
};

pub(crate) fn durable_proof_role() -> Option<DurableProofRole> {
    match QEMU_DURABLE_ROLE {
        0 => Some(DurableProofRole::Disabled),
        1 => Some(DurableProofRole::Writer),
        2 => Some(DurableProofRole::Recovery),
        _ => None,
    }
}

pub(crate) fn durable_storage_profile(
    role: DurableProofRole,
) -> Option<NativeDurableStorageProfile> {
    if role == DurableProofRole::Disabled {
        return Some(NativeDurableStorageProfile::Disabled);
    }
    let root = ResourceId::new(QEMU_DURABLE_ROOT_RESOURCE);
    let storage = ResourceId::new(QEMU_DURABLE_STORAGE_RESOURCE);
    let public_key = DurableStatePublicKey::ecdsa_p256(QEMU_DURABLE_STATE_PUBLIC_KEY)?;
    let signer = DurableStateSignerRecord::new_with_key(
        root,
        public_key,
        DurableStateSignerStatus::Active,
        QEMU_DURABLE_POLICY_GENERATION,
    )?;
    let pio = AtaPioConfig::new(0x1f0, 0x3f6, AtaDrive::Slave, ATA_POLL_BUDGET).ok()?;
    let config = NativeAtaDurableConfig::new(
        pio,
        root,
        storage,
        QEMU_DURABLE_BASE_LBA,
        signer,
        QEMU_DURABLE_POLICY_GENERATION,
    )
    .ok()?;
    Some(NativeDurableStorageProfile::Ata(config))
}

pub(crate) fn tpm_signer_profile(role: DurableProofRole) -> Option<NativeTpmSignerProfile> {
    if role != DurableProofRole::Writer {
        return Some(NativeTpmSignerProfile::Disabled);
    }
    let handle = TpmPersistentHandle::new(QEMU_DURABLE_TPM_HANDLE)?;
    let policy = Sha256PcrPolicy::new(QEMU_DURABLE_PCR_SELECTION, QEMU_DURABLE_PCR_DIGEST).ok()?;
    let config = ProvisionedTpmSignerConfig::new_pcr_policy(
        handle,
        DigestSignCommand::SignV184,
        QEMU_DURABLE_POLICY_GENERATION,
        QEMU_DURABLE_TPM_NAME,
        QEMU_DURABLE_STATE_PUBLIC_KEY,
        policy,
    )
    .ok()?;
    Some(NativeTpmSignerProfile::Crb(config))
}

pub(crate) fn state_signer_profile(role: DurableProofRole) -> Option<StateSignerBootProfile> {
    (role == DurableProofRole::Writer).then_some(StateSignerBootProfile {
        package: QEMU_STATE_SIGNER_PACKAGE,
        public_key: QEMU_STATE_SIGNER_PUBLIC_KEY,
        agent: AgentId::new(QEMU_STATE_SIGNER_AGENT),
        archive_authority: CapabilityId::new(QEMU_ARCHIVE_AUTHORITY),
        storage_authority: CapabilityId::new(QEMU_STORAGE_AUTHORITY),
        nonce: QEMU_STATE_SIGNER_NONCE,
        through_sequence: QEMU_DURABLE_THROUGH_SEQUENCE,
        call_data_generation: QEMU_DURABLE_CALL_DATA_GENERATION,
        return_offsets: QEMU_STATE_SIGNER_RETURN_OFFSETS,
    })
}

impl DurableProofRole {
    pub(crate) const fn is_enabled(self) -> bool {
        !matches!(self, Self::Disabled)
    }

    pub(crate) const fn is_writer(self) -> bool {
        matches!(self, Self::Writer)
    }
}

impl StateSignerBootProfile {
    pub(crate) const fn package(self) -> &'static [u8] {
        self.package
    }

    pub(crate) const fn public_key(self) -> [u8; 32] {
        self.public_key
    }

    pub(crate) const fn agent(self) -> AgentId {
        self.agent
    }

    pub(crate) const fn archive_authority(self) -> CapabilityId {
        self.archive_authority
    }

    pub(crate) const fn storage_authority(self) -> CapabilityId {
        self.storage_authority
    }

    pub(crate) const fn nonce(self) -> u64 {
        self.nonce
    }

    pub(crate) const fn through_sequence(self) -> u64 {
        self.through_sequence
    }

    pub(crate) const fn call_data_generation(self) -> u64 {
        self.call_data_generation
    }

    pub(crate) const fn return_offsets(self) -> [u32; 6] {
        self.return_offsets
    }
}
