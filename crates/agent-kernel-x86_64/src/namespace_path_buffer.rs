//! Canonical fixed-page transport for native Namespace paths.
//!
//! This architecture-library module decodes one bounded, little-endian record
//! copied from an Agent's fixed call-data page. It performs no memory access,
//! allocation, authorization, traversal, or mutation.

use agent_kernel_core::{
    CapabilityId, NamespaceKey, NamespacePathSegment, ResourceId, NAMESPACE_PATH_MAX_DEPTH,
};

pub const NAMESPACE_PATH_BUFFER_MAGIC: [u8; 8] = *b"NSPATH51";
pub const NAMESPACE_PATH_BUFFER_VERSION: u64 = 1;
pub const NAMESPACE_PATH_BUFFER_BYTES: usize = 112;

const GENERATION_OFFSET: usize = 16;
const ROOT_OFFSET: usize = 24;
const DEPTH_OFFSET: usize = 32;
const LENGTH_OFFSET: usize = 40;
const SEGMENTS_OFFSET: usize = 48;
const SEGMENT_BYTES: usize = 16;
const MIN_MEMORY_PATH_DEPTH: usize = 3;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct NamespacePathBuffer {
    root: ResourceId,
    generation: u64,
    depth: u8,
    segments: [NamespacePathSegment; NAMESPACE_PATH_MAX_DEPTH],
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum NamespacePathBufferDecodeError {
    InvalidMagic,
    UnsupportedVersion,
    GenerationMismatch,
    RootMismatch,
    InvalidDepth,
    InvalidLength,
    InvalidAuthority,
    NonCanonicalUnusedSegment,
}

impl NamespacePathBuffer {
    pub fn decode(
        bytes: &[u8; NAMESPACE_PATH_BUFFER_BYTES],
        expected_root: ResourceId,
        expected_generation: u64,
    ) -> Result<Self, NamespacePathBufferDecodeError> {
        if bytes[..8] != NAMESPACE_PATH_BUFFER_MAGIC {
            return Err(NamespacePathBufferDecodeError::InvalidMagic);
        }
        if read_word(bytes, 8) != NAMESPACE_PATH_BUFFER_VERSION {
            return Err(NamespacePathBufferDecodeError::UnsupportedVersion);
        }

        let generation = read_word(bytes, GENERATION_OFFSET);
        if generation == 0 || generation != expected_generation {
            return Err(NamespacePathBufferDecodeError::GenerationMismatch);
        }
        let root = ResourceId::new(read_word(bytes, ROOT_OFFSET));
        if root.raw() == 0 || root != expected_root {
            return Err(NamespacePathBufferDecodeError::RootMismatch);
        }

        let depth = usize::try_from(read_word(bytes, DEPTH_OFFSET))
            .map_err(|_| NamespacePathBufferDecodeError::InvalidDepth)?;
        if !(MIN_MEMORY_PATH_DEPTH..=NAMESPACE_PATH_MAX_DEPTH).contains(&depth) {
            return Err(NamespacePathBufferDecodeError::InvalidDepth);
        }
        if read_word(bytes, LENGTH_OFFSET) != NAMESPACE_PATH_BUFFER_BYTES as u64 {
            return Err(NamespacePathBufferDecodeError::InvalidLength);
        }

        let empty = NamespacePathSegment::new(CapabilityId::new(0), NamespaceKey::new(0));
        let mut segments = [empty; NAMESPACE_PATH_MAX_DEPTH];
        for (index, segment) in segments.iter_mut().enumerate().take(depth) {
            let offset = SEGMENTS_OFFSET + index * SEGMENT_BYTES;
            let authority = CapabilityId::new(read_word(bytes, offset));
            if authority.raw() == 0 {
                return Err(NamespacePathBufferDecodeError::InvalidAuthority);
            }
            *segment = NamespacePathSegment::new(
                authority,
                NamespaceKey::new(read_word(bytes, offset + 8)),
            );
        }
        for index in depth..NAMESPACE_PATH_MAX_DEPTH {
            let offset = SEGMENTS_OFFSET + index * SEGMENT_BYTES;
            if read_word(bytes, offset) != 0 || read_word(bytes, offset + 8) != 0 {
                return Err(NamespacePathBufferDecodeError::NonCanonicalUnusedSegment);
            }
        }

        Ok(Self {
            root,
            generation,
            depth: depth as u8,
            segments,
        })
    }

    pub const fn root(self) -> ResourceId {
        self.root
    }

    pub const fn generation(self) -> u64 {
        self.generation
    }

    pub const fn depth(self) -> u8 {
        self.depth
    }

    pub fn segments(&self) -> &[NamespacePathSegment] {
        &self.segments[..usize::from(self.depth)]
    }
}

fn read_word(bytes: &[u8; NAMESPACE_PATH_BUFFER_BYTES], offset: usize) -> u64 {
    let mut word = [0; 8];
    word.copy_from_slice(&bytes[offset..offset + 8]);
    u64::from_le_bytes(word)
}
