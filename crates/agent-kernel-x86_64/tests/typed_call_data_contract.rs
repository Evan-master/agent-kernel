use agent_kernel_core::{
    CapabilityId, NamespaceKey, NamespaceObject, NamespacePathSegment, ResourceId,
};
use agent_kernel_x86_64::{
    agent_call::encode_namespace_object,
    typed_call_data::{
        CallDataMessage, CallDataMessageDecodeError, CallDataMessageKind,
        NamespacePathRebindMessage, TYPED_CALL_DATA_BYTES, TYPED_CALL_DATA_MAGIC,
        TYPED_CALL_DATA_PAYLOAD_BYTES, TYPED_CALL_DATA_VERSION,
    },
};

const GENERATION: u64 = 1;
const ROOT: ResourceId = ResourceId::new(1);

#[test]
fn canonical_one_and_four_hop_messages_decode_to_typed_requests() {
    for depth in [1, 4] {
        let bytes = canonical_record(depth);
        let decoded = CallDataMessage::decode(
            &bytes,
            CallDataMessageKind::CompareAndRebindNamespacePath,
            GENERATION,
        )
        .unwrap();

        assert_eq!(
            decoded.kind(),
            CallDataMessageKind::CompareAndRebindNamespacePath
        );
        assert_eq!(decoded.generation(), GENERATION);
        let CallDataMessage::CompareAndRebindNamespacePath(request) = decoded else {
            panic!("expected Namespace path rebinding message");
        };
        assert_request(request, depth);
    }
}

#[test]
fn decoder_rejects_noncanonical_envelope_words() {
    let cases = [
        (0, 0, CallDataMessageDecodeError::InvalidMagic),
        (8, 2, CallDataMessageDecodeError::UnsupportedVersion),
        (16, 0, CallDataMessageDecodeError::GenerationMismatch),
        (24, 3, CallDataMessageDecodeError::UnsupportedKind),
        (32, 159, CallDataMessageDecodeError::InvalidTotalLength),
        (40, 95, CallDataMessageDecodeError::InvalidPayloadLength),
        (144, 1, CallDataMessageDecodeError::NonCanonicalFlags),
        (152, 1, CallDataMessageDecodeError::NonCanonicalReserved),
    ];

    for (offset, value, expected) in cases {
        let mut bytes = canonical_record(4);
        write_word(&mut bytes, offset, value);
        assert_eq!(
            CallDataMessage::decode(
                &bytes,
                CallDataMessageKind::CompareAndRebindNamespacePath,
                GENERATION,
            ),
            Err(expected)
        );
    }

    let bytes = canonical_record(4);
    assert_eq!(
        CallDataMessage::decode(
            &bytes,
            CallDataMessageKind::CompareAndRebindNamespacePath,
            GENERATION + 1,
        ),
        Err(CallDataMessageDecodeError::GenerationMismatch)
    );
}

#[test]
fn decoder_rejects_invalid_path_mutation_payloads() {
    let cases = [
        (112, 0, CallDataMessageDecodeError::InvalidRoot),
        (120, 0, CallDataMessageDecodeError::InvalidDepth),
        (120, 5, CallDataMessageDecodeError::InvalidDepth),
        (128, 0, CallDataMessageDecodeError::InvalidRevision),
        (136, 0, CallDataMessageDecodeError::InvalidReplacement),
        (48, 0, CallDataMessageDecodeError::InvalidAuthority),
    ];

    for (offset, value, expected) in cases {
        let mut bytes = canonical_record(4);
        write_word(&mut bytes, offset, value);
        assert_eq!(
            CallDataMessage::decode(
                &bytes,
                CallDataMessageKind::CompareAndRebindNamespacePath,
                GENERATION,
            ),
            Err(expected)
        );
    }

    let mut unused = canonical_record(1);
    write_word(&mut unused, 64, 99);
    assert_eq!(
        CallDataMessage::decode(
            &unused,
            CallDataMessageKind::CompareAndRebindNamespacePath,
            GENERATION,
        ),
        Err(CallDataMessageDecodeError::NonCanonicalUnusedSegment)
    );
}

fn assert_request(request: NamespacePathRebindMessage, depth: usize) {
    assert_eq!(request.root(), ROOT);
    assert_eq!(usize::from(request.depth()), depth);
    assert_eq!(request.expected_revision(), 1);
    assert_eq!(
        request.replacement(),
        NamespaceObject::Resource(ResourceId::new(3))
    );
    assert_eq!(request.segments(), &segments()[..depth]);
}

fn canonical_record(depth: usize) -> [u8; TYPED_CALL_DATA_BYTES] {
    let mut bytes = [0; TYPED_CALL_DATA_BYTES];
    bytes[..8].copy_from_slice(&TYPED_CALL_DATA_MAGIC);
    write_word(&mut bytes, 8, TYPED_CALL_DATA_VERSION);
    write_word(&mut bytes, 16, GENERATION);
    write_word(
        &mut bytes,
        24,
        CallDataMessageKind::CompareAndRebindNamespacePath.raw(),
    );
    write_word(&mut bytes, 32, TYPED_CALL_DATA_BYTES as u64);
    write_word(&mut bytes, 40, TYPED_CALL_DATA_PAYLOAD_BYTES as u64);
    write_word(&mut bytes, 112, ROOT.raw());
    write_word(&mut bytes, 120, depth as u64);
    write_word(&mut bytes, 128, 1);
    write_word(
        &mut bytes,
        136,
        encode_namespace_object(NamespaceObject::Resource(ResourceId::new(3))).unwrap(),
    );
    for (index, segment) in segments().iter().copied().take(depth).enumerate() {
        write_word(&mut bytes, 48 + index * 16, segment.authority().raw());
        write_word(&mut bytes, 56 + index * 16, segment.key().raw());
    }
    bytes
}

fn segments() -> [NamespacePathSegment; 4] {
    [
        NamespacePathSegment::new(CapabilityId::new(12), NamespaceKey::new(0x4e53_0001)),
        NamespacePathSegment::new(CapabilityId::new(13), NamespaceKey::new(0x4e53_0002)),
        NamespacePathSegment::new(CapabilityId::new(20), NamespaceKey::new(0x4e53_0003)),
        NamespacePathSegment::new(CapabilityId::new(21), NamespaceKey::new(0x4e53_0004)),
    ]
}

fn write_word(bytes: &mut [u8; TYPED_CALL_DATA_BYTES], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}
