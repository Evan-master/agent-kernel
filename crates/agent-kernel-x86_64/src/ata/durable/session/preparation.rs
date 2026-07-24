//! One-shot canonical preparation for a native signed archive transaction.
//!
//! The ATA session binds one Core-produced preflight to the scheduled
//! Agent/Task/Image identity, retains the exact payload, and stages an unsigned
//! request. No signature verification or device write occurs in this phase.

use agent_kernel_core::{
    encode_event_archive_payload, AgentId, AgentImageId, DurableArchiveAnchor,
    DurableArchiveManifest, DurableArchiveManifestError, DurableArchivePreflight,
    DurableStateDigest, Event, EventArchiveEncodingError, TaskId,
};

use crate::{
    ata::{AtaBlockDevice, AtaDurableHead, NativeAtaDurableSession},
    durable_archive_request::{
        encode_unsigned_durable_archive_request, DurableArchiveRequestEncodeError,
        DURABLE_ARCHIVE_REQUEST_BYTES,
    },
};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct NativeDurableArchiveCaller {
    agent: AgentId,
    task: TaskId,
    image: AgentImageId,
}

impl NativeDurableArchiveCaller {
    pub const fn new(agent: AgentId, task: TaskId, image: AgentImageId) -> Option<Self> {
        if agent.raw() == 0 || task.raw() == 0 || image.raw() == 0 {
            None
        } else {
            Some(Self { agent, task, image })
        }
    }

    pub const fn agent(self) -> AgentId {
        self.agent
    }

    pub const fn task(self) -> TaskId {
        self.task
    }

    pub const fn image(self) -> AgentImageId {
        self.image
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct NativeDurableArchivePreparation {
    caller: NativeDurableArchiveCaller,
    preflight: DurableArchivePreflight,
    call_data_generation: u64,
    manifest: DurableArchiveManifest,
    payload_length: u32,
    request_bytes: [u8; DURABLE_ARCHIVE_REQUEST_BYTES],
}

impl NativeDurableArchivePreparation {
    pub const fn caller(self) -> NativeDurableArchiveCaller {
        self.caller
    }

    pub const fn preflight(self) -> DurableArchivePreflight {
        self.preflight
    }

    pub const fn call_data_generation(self) -> u64 {
        self.call_data_generation
    }

    pub const fn manifest(self) -> DurableArchiveManifest {
        self.manifest
    }

    pub const fn payload_length(self) -> u32 {
        self.payload_length
    }

    pub const fn request_bytes(self) -> [u8; DURABLE_ARCHIVE_REQUEST_BYTES] {
        self.request_bytes
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum NativeAtaDurablePrepareError {
    SessionFaulted,
    AlreadyPrepared,
    CallerMismatch,
    ConfigurationMismatch,
    GenerationMismatch { expected: u64, actual: u64 },
    Archive(EventArchiveEncodingError),
    Manifest(DurableArchiveManifestError),
    Request(DurableArchiveRequestEncodeError),
}

impl<'a, D: AtaBlockDevice> NativeAtaDurableSession<'a, D> {
    pub fn prepare(
        &mut self,
        caller: NativeDurableArchiveCaller,
        preflight: DurableArchivePreflight,
        events: &[Event],
        call_data_generation: u64,
    ) -> Result<NativeDurableArchivePreparation, NativeAtaDurablePrepareError> {
        if self.faulted {
            return Err(NativeAtaDurablePrepareError::SessionFaulted);
        }
        if self.preparation.is_some() {
            return Err(NativeAtaDurablePrepareError::AlreadyPrepared);
        }
        if caller.agent() != preflight.actor() {
            return Err(NativeAtaDurablePrepareError::CallerMismatch);
        }
        if preflight.root() != self.config.root() || preflight.storage() != self.config.storage() {
            return Err(NativeAtaDurablePrepareError::ConfigurationMismatch);
        }
        let proposal = preflight.proposal();
        let expected_generation = self
            .backend
            .head()
            .and_then(AtaDurableHead::next_generation)
            .ok_or(NativeAtaDurablePrepareError::SessionFaulted)?;
        if proposal.generation() != expected_generation {
            return Err(NativeAtaDurablePrepareError::GenerationMismatch {
                expected: expected_generation,
                actual: proposal.generation(),
            });
        }

        let payload_length = encode_event_archive_payload(proposal, events, self.payload.as_mut())
            .map_err(NativeAtaDurablePrepareError::Archive)?;
        let signer = self.config.signer();
        let manifest = DurableArchiveManifest::new_for_signature_algorithm(
            proposal,
            caller.agent(),
            preflight.archive_authority(),
            preflight.root(),
            preflight.storage(),
            payload_length as u32,
            DurableStateDigest::from_archive(proposal.digest()),
            signer.signer_id,
            signer.signature_algorithm(),
            self.config.policy_generation(),
            DurableArchiveAnchor::unanchored(),
        )
        .map_err(NativeAtaDurablePrepareError::Manifest)?;
        let request_bytes = encode_unsigned_durable_archive_request(
            call_data_generation,
            preflight.storage_authority(),
            manifest,
        )
        .map_err(NativeAtaDurablePrepareError::Request)?;
        let preparation = NativeDurableArchivePreparation {
            caller,
            preflight,
            call_data_generation,
            manifest,
            payload_length: payload_length as u32,
            request_bytes,
        };
        self.preparation = Some(preparation);
        Ok(preparation)
    }
}
