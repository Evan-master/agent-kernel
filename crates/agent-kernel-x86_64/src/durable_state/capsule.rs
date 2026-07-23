//! Fixed-width durable archive slot capsule values.

mod encoding;
mod parse;

pub use encoding::{
    encode_durable_archive_body, encode_durable_archive_commit_footer,
    encode_durable_archive_prepared_header, DurableArchiveCapsuleEncodeError,
};
pub use parse::parse_durable_archive_slot;

use agent_kernel_core::{
    DurableArchiveManifest, DurableArchiveSignature, DurableSlot, DurableStateDigest, ResourceId,
};

use super::DurableArchiveManifestDecodeError;

pub(super) const HEADER_MAGIC: &[u8; 8] = b"AKDHDR13";
pub(super) const FOOTER_MAGIC: &[u8; 8] = b"AKDCMT13";
pub(super) const CAPSULE_FORMAT_VERSION: u16 = 1;
pub(super) const PREPARED_STATE: u16 = 1;
pub(super) const COMMITTED_STATE: u16 = 2;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DurableArchiveCapsuleError {
    SlotLengthMismatch {
        length: usize,
        required: usize,
    },
    HeaderMagicMismatch,
    HeaderVersionUnsupported {
        version: u16,
    },
    HeaderStateInvalid {
        state: u16,
    },
    HeaderFlagsUnsupported {
        flags: u16,
    },
    HeaderSlotInvalid {
        tag: u8,
    },
    HeaderReservedNotZero,
    ZeroGeneration,
    GenerationSlotMismatch {
        expected: DurableSlot,
        encoded: DurableSlot,
    },
    PhysicalSlotMismatch {
        encoded: DurableSlot,
        physical: DurableSlot,
    },
    BodyLengthMismatch {
        encoded: usize,
        expected: usize,
    },
    PayloadLengthOutOfRange {
        length: usize,
        limit: usize,
    },
    ManifestLengthMismatch {
        length: usize,
        required: usize,
    },
    SignatureLengthMismatch {
        length: usize,
        required: usize,
    },
    Manifest(DurableArchiveManifestDecodeError),
    ManifestGenerationMismatch,
    ManifestStorageMismatch {
        expected: ResourceId,
        encoded: ResourceId,
    },
    ManifestPayloadLengthMismatch,
    PayloadDigestMismatch,
    BodyPaddingNotZero,
    FooterMagicMismatch,
    FooterVersionUnsupported {
        version: u16,
    },
    FooterStateInvalid {
        state: u16,
    },
    FooterSlotInvalid {
        tag: u8,
    },
    FooterSlotMismatch,
    FooterGenerationMismatch,
    FooterManifestDigestMismatch,
    FooterReservedNotZero,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DecodedDurableArchiveSlot<'a> {
    Empty,
    Prepared(DurableArchiveCapsule<'a>),
    Committed(DurableArchiveCapsule<'a>),
}

impl<'a> DecodedDurableArchiveSlot<'a> {
    pub const fn capsule(self) -> Option<DurableArchiveCapsule<'a>> {
        match self {
            Self::Empty => None,
            Self::Prepared(capsule) | Self::Committed(capsule) => Some(capsule),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct DurableArchiveCapsule<'a> {
    pub(super) slot: DurableSlot,
    pub(super) payload: &'a [u8],
    pub(super) manifest: DurableArchiveManifest,
    pub(super) signature: DurableArchiveSignature,
    pub(super) manifest_digest: DurableStateDigest,
}

impl<'a> DurableArchiveCapsule<'a> {
    pub const fn slot(self) -> DurableSlot {
        self.slot
    }

    pub const fn payload(self) -> &'a [u8] {
        self.payload
    }

    pub const fn manifest(self) -> DurableArchiveManifest {
        self.manifest
    }

    pub const fn signature(self) -> DurableArchiveSignature {
        self.signature
    }

    pub const fn manifest_digest(self) -> DurableStateDigest {
        self.manifest_digest
    }
}
