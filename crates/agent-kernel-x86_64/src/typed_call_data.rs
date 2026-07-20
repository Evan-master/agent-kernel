//! Canonical typed messages carried by an Agent's fixed call-data page.
//!
//! This architecture-library module decodes one bounded little-endian record.
//! It owns envelope canonicalization and message-specific field validation,
//! while performing no memory access, authorization, allocation, or mutation.

use agent_kernel_core::{
    CapabilityId, NamespaceKey, NamespaceObject, NamespacePathSegment, ResourceId,
    NAMESPACE_PATH_MAX_DEPTH,
};

use crate::namespace_object_wire::decode_namespace_object;

pub const TYPED_CALL_DATA_MAGIC: [u8; 8] = *b"AGNTMSG1";
pub const TYPED_CALL_DATA_VERSION: u64 = 1;
pub const TYPED_CALL_DATA_BYTES: usize = 160;
pub const TYPED_CALL_DATA_PAYLOAD_BYTES: usize = 112;

const SEGMENTS_OFFSET: usize = 48;
const SEGMENT_BYTES: usize = 16;
const ROOT_OFFSET: usize = 112;
const DEPTH_OFFSET: usize = 120;
const REVISION_OFFSET: usize = 128;
const REPLACEMENT_OFFSET: usize = 136;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u64)]
pub enum CallDataMessageKind {
    CompareAndRebindNamespacePath = 1,
}

impl CallDataMessageKind {
    pub const fn raw(self) -> u64 {
        self as u64
    }

    const fn from_raw(raw: u64) -> Option<Self> {
        match raw {
            1 => Some(Self::CompareAndRebindNamespacePath),
            _ => None,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CallDataMessage {
    CompareAndRebindNamespacePath(NamespacePathRebindMessage),
}

impl CallDataMessage {
    pub fn decode(
        bytes: &[u8; TYPED_CALL_DATA_BYTES],
        expected_kind: CallDataMessageKind,
        expected_generation: u64,
    ) -> Result<Self, CallDataMessageDecodeError> {
        if bytes[..8] != TYPED_CALL_DATA_MAGIC {
            return Err(CallDataMessageDecodeError::InvalidMagic);
        }
        if read_word(bytes, 8) != TYPED_CALL_DATA_VERSION {
            return Err(CallDataMessageDecodeError::UnsupportedVersion);
        }
        let generation = read_word(bytes, 16);
        if generation == 0 || generation != expected_generation {
            return Err(CallDataMessageDecodeError::GenerationMismatch);
        }
        let kind = CallDataMessageKind::from_raw(read_word(bytes, 24))
            .ok_or(CallDataMessageDecodeError::UnsupportedKind)?;
        if kind != expected_kind {
            return Err(CallDataMessageDecodeError::KindMismatch);
        }
        if read_word(bytes, 32) != TYPED_CALL_DATA_BYTES as u64 {
            return Err(CallDataMessageDecodeError::InvalidTotalLength);
        }
        if read_word(bytes, 40) != TYPED_CALL_DATA_PAYLOAD_BYTES as u64 {
            return Err(CallDataMessageDecodeError::InvalidPayloadLength);
        }
        if read_word(bytes, 144) != 0 {
            return Err(CallDataMessageDecodeError::NonCanonicalFlags);
        }
        if read_word(bytes, 152) != 0 {
            return Err(CallDataMessageDecodeError::NonCanonicalReserved);
        }

        match kind {
            CallDataMessageKind::CompareAndRebindNamespacePath => {
                decode_namespace_path_rebind(bytes, generation)
                    .map(Self::CompareAndRebindNamespacePath)
            }
        }
    }

    pub const fn kind(self) -> CallDataMessageKind {
        match self {
            Self::CompareAndRebindNamespacePath(_) => {
                CallDataMessageKind::CompareAndRebindNamespacePath
            }
        }
    }

    pub const fn generation(self) -> u64 {
        match self {
            Self::CompareAndRebindNamespacePath(message) => message.generation,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct NamespacePathRebindMessage {
    generation: u64,
    root: ResourceId,
    depth: u8,
    expected_revision: u64,
    replacement: NamespaceObject,
    segments: [NamespacePathSegment; NAMESPACE_PATH_MAX_DEPTH],
}

impl NamespacePathRebindMessage {
    pub const fn generation(self) -> u64 {
        self.generation
    }

    pub const fn root(self) -> ResourceId {
        self.root
    }

    pub const fn depth(self) -> u8 {
        self.depth
    }

    pub const fn expected_revision(self) -> u64 {
        self.expected_revision
    }

    pub const fn replacement(self) -> NamespaceObject {
        self.replacement
    }

    pub fn segments(&self) -> &[NamespacePathSegment] {
        &self.segments[..usize::from(self.depth)]
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CallDataMessageDecodeError {
    InvalidMagic,
    UnsupportedVersion,
    UnsupportedKind,
    KindMismatch,
    GenerationMismatch,
    InvalidTotalLength,
    InvalidPayloadLength,
    NonCanonicalFlags,
    NonCanonicalReserved,
    InvalidRoot,
    InvalidDepth,
    InvalidRevision,
    InvalidReplacement,
    InvalidAuthority,
    NonCanonicalUnusedSegment,
}

fn decode_namespace_path_rebind(
    bytes: &[u8; TYPED_CALL_DATA_BYTES],
    generation: u64,
) -> Result<NamespacePathRebindMessage, CallDataMessageDecodeError> {
    let root = ResourceId::new(read_word(bytes, ROOT_OFFSET));
    if root.raw() == 0 {
        return Err(CallDataMessageDecodeError::InvalidRoot);
    }
    let depth = usize::try_from(read_word(bytes, DEPTH_OFFSET))
        .map_err(|_| CallDataMessageDecodeError::InvalidDepth)?;
    if !(1..=NAMESPACE_PATH_MAX_DEPTH).contains(&depth) {
        return Err(CallDataMessageDecodeError::InvalidDepth);
    }
    let expected_revision = read_word(bytes, REVISION_OFFSET);
    if expected_revision == 0 {
        return Err(CallDataMessageDecodeError::InvalidRevision);
    }
    let replacement = decode_namespace_object(read_word(bytes, REPLACEMENT_OFFSET))
        .ok_or(CallDataMessageDecodeError::InvalidReplacement)?;
    let empty = NamespacePathSegment::new(CapabilityId::new(0), NamespaceKey::new(0));
    let mut segments = [empty; NAMESPACE_PATH_MAX_DEPTH];
    for (index, segment) in segments.iter_mut().enumerate().take(depth) {
        let offset = SEGMENTS_OFFSET + index * SEGMENT_BYTES;
        let authority = CapabilityId::new(read_word(bytes, offset));
        if authority.raw() == 0 {
            return Err(CallDataMessageDecodeError::InvalidAuthority);
        }
        *segment =
            NamespacePathSegment::new(authority, NamespaceKey::new(read_word(bytes, offset + 8)));
    }
    for index in depth..NAMESPACE_PATH_MAX_DEPTH {
        let offset = SEGMENTS_OFFSET + index * SEGMENT_BYTES;
        if read_word(bytes, offset) != 0 || read_word(bytes, offset + 8) != 0 {
            return Err(CallDataMessageDecodeError::NonCanonicalUnusedSegment);
        }
    }

    Ok(NamespacePathRebindMessage {
        generation,
        root,
        depth: depth as u8,
        expected_revision,
        replacement,
        segments,
    })
}

fn read_word(bytes: &[u8; TYPED_CALL_DATA_BYTES], offset: usize) -> u64 {
    let mut word = [0; 8];
    word.copy_from_slice(&bytes[offset..offset + 8]);
    u64::from_le_bytes(word)
}
