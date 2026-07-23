//! Initialization failures for the native ATA durable boot session.

use crate::{
    ata::{
        AtaDurableBackendInitError, AtaDurableBindingError, AtaDurableHeadBindError, AtaPioError,
    },
    durable_state::DurableArchiveRecoveryError,
};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum NativeAtaDurableInitError {
    Identify(AtaPioError),
    Binding(AtaDurableBindingError),
    Backend(AtaDurableBackendInitError),
    Recovery(DurableArchiveRecoveryError),
    Head(AtaDurableHeadBindError),
}
