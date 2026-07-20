use agent_kernel_core::{CapabilityId, NamespaceKey, NamespacePathSegment, ResourceId};
use agent_kernel_x86_64::namespace_path_buffer::{
    NamespacePathBuffer, NamespacePathBufferDecodeError, NAMESPACE_PATH_BUFFER_BYTES,
    NAMESPACE_PATH_BUFFER_MAGIC, NAMESPACE_PATH_BUFFER_VERSION,
};

const ROOT: ResourceId = ResourceId::new(1);
const GENERATION: u64 = 7;

#[test]
fn canonical_three_and_four_hop_records_decode_to_fixed_segments() {
    for depth in [3, 4] {
        let bytes = canonical_record(depth);
        let decoded = NamespacePathBuffer::decode(&bytes, ROOT, GENERATION).unwrap();

        assert_eq!(decoded.root(), ROOT);
        assert_eq!(decoded.generation(), GENERATION);
        assert_eq!(usize::from(decoded.depth()), depth);
        assert_eq!(decoded.segments(), &segments()[..depth]);
    }
}

#[test]
fn decoder_rejects_noncanonical_headers_and_register_mismatches() {
    let cases = [
        (0, 0, NamespacePathBufferDecodeError::InvalidMagic),
        (8, 2, NamespacePathBufferDecodeError::UnsupportedVersion),
        (16, 0, NamespacePathBufferDecodeError::GenerationMismatch),
        (24, 2, NamespacePathBufferDecodeError::RootMismatch),
        (32, 2, NamespacePathBufferDecodeError::InvalidDepth),
        (40, 111, NamespacePathBufferDecodeError::InvalidLength),
    ];

    for (offset, value, expected) in cases {
        let mut bytes = canonical_record(4);
        write_word(&mut bytes, offset, value);
        assert_eq!(
            NamespacePathBuffer::decode(&bytes, ROOT, GENERATION),
            Err(expected)
        );
    }

    let bytes = canonical_record(4);
    assert_eq!(
        NamespacePathBuffer::decode(&bytes, ROOT, GENERATION + 1),
        Err(NamespacePathBufferDecodeError::GenerationMismatch)
    );
    assert_eq!(
        NamespacePathBuffer::decode(&bytes, ResourceId::new(2), GENERATION),
        Err(NamespacePathBufferDecodeError::RootMismatch)
    );
}

#[test]
fn decoder_rejects_zero_authority_and_nonzero_unused_segment() {
    let mut zero_authority = canonical_record(4);
    write_word(&mut zero_authority, 48 + 2 * 16, 0);
    assert_eq!(
        NamespacePathBuffer::decode(&zero_authority, ROOT, GENERATION),
        Err(NamespacePathBufferDecodeError::InvalidAuthority)
    );

    let mut unused_segment = canonical_record(3);
    write_word(&mut unused_segment, 96, 99);
    assert_eq!(
        NamespacePathBuffer::decode(&unused_segment, ROOT, GENERATION),
        Err(NamespacePathBufferDecodeError::NonCanonicalUnusedSegment)
    );
}

fn canonical_record(depth: usize) -> [u8; NAMESPACE_PATH_BUFFER_BYTES] {
    let mut bytes = [0; NAMESPACE_PATH_BUFFER_BYTES];
    bytes[..8].copy_from_slice(&NAMESPACE_PATH_BUFFER_MAGIC);
    write_word(&mut bytes, 8, NAMESPACE_PATH_BUFFER_VERSION);
    write_word(&mut bytes, 16, GENERATION);
    write_word(&mut bytes, 24, ROOT.raw());
    write_word(&mut bytes, 32, depth as u64);
    write_word(&mut bytes, 40, NAMESPACE_PATH_BUFFER_BYTES as u64);
    for (index, segment) in segments().iter().copied().take(depth).enumerate() {
        write_word(&mut bytes, 48 + index * 16, segment.authority().raw());
        write_word(&mut bytes, 56 + index * 16, segment.key().raw());
    }
    bytes
}

fn segments() -> [NamespacePathSegment; 4] {
    [
        NamespacePathSegment::new(CapabilityId::new(12), NamespaceKey::new(0x1001)),
        NamespacePathSegment::new(CapabilityId::new(13), NamespaceKey::new(0x1002)),
        NamespacePathSegment::new(CapabilityId::new(14), NamespaceKey::new(0x1003)),
        NamespacePathSegment::new(CapabilityId::new(15), NamespaceKey::new(0x1004)),
    ]
}

fn write_word(bytes: &mut [u8; NAMESPACE_PATH_BUFFER_BYTES], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}
