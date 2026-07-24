//! Trusted cancellation for one live durable archive preparation.
//!
//! The boot-owned ATA session releases staged payload state only for the exact
//! Agent, Task, Image, and call-data generation that created it. Cancellation
//! performs no storage I/O and leaves faulted sessions closed.

use crate::ata::{
    AtaBlockDevice, NativeAtaDurableSession, NativeDurableArchiveCaller,
    NativeDurableArchivePreparation,
};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum NativeAtaDurableCancelError {
    SessionFaulted,
    NoPreparation,
    CallerMismatch,
    GenerationMismatch { expected: u64, actual: u64 },
}

impl<'a, D: AtaBlockDevice> NativeAtaDurableSession<'a, D> {
    pub fn cancel_preparation(
        &mut self,
        caller: NativeDurableArchiveCaller,
        call_data_generation: u64,
    ) -> Result<NativeDurableArchivePreparation, NativeAtaDurableCancelError> {
        if self.faulted {
            return Err(NativeAtaDurableCancelError::SessionFaulted);
        }
        let preparation = self
            .preparation
            .ok_or(NativeAtaDurableCancelError::NoPreparation)?;
        if preparation.caller() != caller {
            return Err(NativeAtaDurableCancelError::CallerMismatch);
        }
        if preparation.call_data_generation() != call_data_generation {
            return Err(NativeAtaDurableCancelError::GenerationMismatch {
                expected: preparation.call_data_generation(),
                actual: call_data_generation,
            });
        }
        self.preparation = None;
        Ok(preparation)
    }
}
