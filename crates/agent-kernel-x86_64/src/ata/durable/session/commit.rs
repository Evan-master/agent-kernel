//! Signed Event archive orchestration over one initialized ATA session.

use agent_kernel_core::DurableArchivePreflight;

use crate::{
    ata::{AtaBlockDevice, NativeAtaDurableSession, NativeDurableArchiveCaller},
    durable_archive_request::{
        DurableArchiveRequest, DURABLE_ARCHIVE_REQUEST_BYTES,
        DURABLE_ARCHIVE_REQUEST_RESERVED_OFFSET, DURABLE_ARCHIVE_REQUEST_SIGNATURE_OFFSET,
    },
    durable_state::{
        commit_durable_archive, DurableArchiveCommitError, DurableStateTrustPolicy,
        DurableStateVerificationError, VerifiedDurableArchiveCommit,
    },
};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum NativeAtaDurableCommitError {
    SessionFaulted,
    NoPreparation,
    CallerMismatch,
    PreflightMismatch,
    RequestMismatch,
    Trust(DurableStateVerificationError),
    Transaction(DurableArchiveCommitError),
}

impl<'a, D: AtaBlockDevice> NativeAtaDurableSession<'a, D> {
    pub fn commit_prepared(
        &mut self,
        caller: NativeDurableArchiveCaller,
        preflight: DurableArchivePreflight,
        request_bytes: &[u8; DURABLE_ARCHIVE_REQUEST_BYTES],
    ) -> Result<VerifiedDurableArchiveCommit, NativeAtaDurableCommitError> {
        if self.faulted {
            return Err(NativeAtaDurableCommitError::SessionFaulted);
        }
        let preparation = self
            .preparation
            .ok_or(NativeAtaDurableCommitError::NoPreparation)?;
        if preparation.caller() != caller {
            return Err(NativeAtaDurableCommitError::CallerMismatch);
        }
        if preparation.preflight() != preflight {
            return Err(NativeAtaDurableCommitError::PreflightMismatch);
        }
        let expected_bytes = preparation.request_bytes();
        if request_bytes[..DURABLE_ARCHIVE_REQUEST_SIGNATURE_OFFSET]
            != expected_bytes[..DURABLE_ARCHIVE_REQUEST_SIGNATURE_OFFSET]
            || request_bytes[DURABLE_ARCHIVE_REQUEST_RESERVED_OFFSET..]
                != expected_bytes[DURABLE_ARCHIVE_REQUEST_RESERVED_OFFSET..]
        {
            return Err(NativeAtaDurableCommitError::RequestMismatch);
        }
        let request =
            DurableArchiveRequest::decode(request_bytes, preparation.call_data_generation())
                .map_err(|_| NativeAtaDurableCommitError::RequestMismatch)?;
        if request.generation() != preparation.call_data_generation()
            || request.storage_authority() != preflight.storage_authority()
            || request.manifest() != preparation.manifest()
        {
            return Err(NativeAtaDurableCommitError::RequestMismatch);
        }

        let signer = self.config.signer();
        let policy = DurableStateTrustPolicy::new(
            core::slice::from_ref(&signer),
            self.config.policy_generation(),
        );
        policy
            .verify(request.manifest(), request.signature())
            .map_err(NativeAtaDurableCommitError::Trust)?;
        let payload_length = preparation.payload_length() as usize;
        let result = commit_durable_archive(
            &mut self.backend,
            policy,
            &self.payload[..payload_length],
            request.manifest(),
            request.signature(),
            self.scratch.as_mut(),
        );
        match result {
            Ok(verified) => {
                self.preparation = None;
                Ok(verified)
            }
            Err(error) => {
                self.preparation = None;
                self.faulted = true;
                Err(NativeAtaDurableCommitError::Transaction(error))
            }
        }
    }
}
