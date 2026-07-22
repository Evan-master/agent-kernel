use agent_kernel_core::{AgentImageKind, AgentImageKindScope, AgentImageSignerId, CapabilityId};
use agent_kernel_x86_64::typed_call_data::{
    AgentImageSignerRotationMessage, CallDataMessage, CallDataMessageDecodeError,
    CallDataMessageKind, TYPED_CALL_DATA_BYTES, TYPED_CALL_DATA_MAGIC,
    TYPED_CALL_DATA_PAYLOAD_BYTES, TYPED_CALL_DATA_VERSION,
};

const MESSAGE_GENERATION: u64 = 7;
const POLICY_GENERATION: u64 = 1;
const AUTHORITY: u64 = 12;
const PREVIOUS_SIGNER: [u8; 32] = [0x21; 32];
const REPLACEMENT_KEY: [u8; 32] = [0x43; 32];

#[test]
fn rotation_message_decodes_the_canonical_160_byte_record() {
    let bytes = rotation_message();

    let decoded = CallDataMessage::decode(
        &bytes,
        CallDataMessageKind::RotateAgentImageSigner,
        MESSAGE_GENERATION,
    )
    .expect("canonical signer rotation should decode");
    assert_eq!(decoded.kind(), CallDataMessageKind::RotateAgentImageSigner);
    assert_eq!(decoded.generation(), MESSAGE_GENERATION);
    let CallDataMessage::RotateAgentImageSigner(message) = decoded else {
        panic!("expected signer rotation message");
    };
    assert_rotation(message);
}

#[test]
fn rotation_message_rejects_noncanonical_policy_fields() {
    let valid = rotation_message();
    let cases = [
        (
            mutated_word(valid, 48, 0),
            CallDataMessageDecodeError::InvalidAuthority,
        ),
        (
            mutated_word(valid, 56, 0),
            CallDataMessageDecodeError::InvalidPolicyGeneration,
        ),
        (
            mutated_range(valid, 64, &[0; 32]),
            CallDataMessageDecodeError::InvalidSignerId,
        ),
        (
            mutated_word(valid, 128, 0),
            CallDataMessageDecodeError::InvalidImageKindScope,
        ),
        (
            mutated_word(valid, 128, 1 << 12),
            CallDataMessageDecodeError::InvalidImageKindScope,
        ),
        (
            mutated_range(valid, 136, &[0, 0, 1, 0, 0, 0, 0, 0]),
            CallDataMessageDecodeError::InvalidAbiRange,
        ),
        (
            mutated_range(valid, 136, &[2, 0, 1, 0, 0, 0, 0, 0]),
            CallDataMessageDecodeError::InvalidAbiRange,
        ),
        (
            mutated_range(valid, 140, &[1, 0, 0, 0]),
            CallDataMessageDecodeError::NonCanonicalAbiReserved,
        ),
    ];

    for (bytes, expected) in cases {
        assert_eq!(
            CallDataMessage::decode(
                &bytes,
                CallDataMessageKind::RotateAgentImageSigner,
                MESSAGE_GENERATION,
            ),
            Err(expected)
        );
    }
}

#[test]
fn rotation_message_keeps_kind_and_generation_bound_to_the_request() {
    let bytes = rotation_message();
    assert_eq!(
        CallDataMessage::decode(
            &bytes,
            CallDataMessageKind::CompareAndRebindNamespacePath,
            MESSAGE_GENERATION,
        ),
        Err(CallDataMessageDecodeError::KindMismatch)
    );
    assert_eq!(
        CallDataMessage::decode(
            &bytes,
            CallDataMessageKind::RotateAgentImageSigner,
            MESSAGE_GENERATION + 1,
        ),
        Err(CallDataMessageDecodeError::GenerationMismatch)
    );
}

fn assert_rotation(message: AgentImageSignerRotationMessage) {
    assert_eq!(message.generation(), MESSAGE_GENERATION);
    assert_eq!(message.authority(), CapabilityId::new(AUTHORITY));
    assert_eq!(message.expected_policy_generation(), POLICY_GENERATION);
    assert_eq!(
        message.previous_signer_id(),
        AgentImageSignerId::new(PREVIOUS_SIGNER)
    );
    assert_eq!(message.replacement_public_key(), REPLACEMENT_KEY);
    assert_eq!(
        message.replacement_image_kinds(),
        AgentImageKindScope::only(AgentImageKind::Worker)
    );
    assert_eq!(message.replacement_minimum_abi(), 1);
    assert_eq!(message.replacement_maximum_abi(), 1);
}

fn rotation_message() -> [u8; TYPED_CALL_DATA_BYTES] {
    let mut bytes = [0; TYPED_CALL_DATA_BYTES];
    bytes[..8].copy_from_slice(&TYPED_CALL_DATA_MAGIC);
    put_word(&mut bytes, 8, TYPED_CALL_DATA_VERSION);
    put_word(&mut bytes, 16, MESSAGE_GENERATION);
    put_word(
        &mut bytes,
        24,
        CallDataMessageKind::RotateAgentImageSigner.raw(),
    );
    put_word(&mut bytes, 32, TYPED_CALL_DATA_BYTES as u64);
    put_word(&mut bytes, 40, TYPED_CALL_DATA_PAYLOAD_BYTES as u64);
    put_word(&mut bytes, 48, AUTHORITY);
    put_word(&mut bytes, 56, POLICY_GENERATION);
    bytes[64..96].copy_from_slice(&PREVIOUS_SIGNER);
    bytes[96..128].copy_from_slice(&REPLACEMENT_KEY);
    put_word(
        &mut bytes,
        128,
        u64::from(AgentImageKindScope::only(AgentImageKind::Worker).bits()),
    );
    bytes[136..138].copy_from_slice(&1_u16.to_le_bytes());
    bytes[138..140].copy_from_slice(&1_u16.to_le_bytes());
    bytes
}

fn put_word(bytes: &mut [u8; TYPED_CALL_DATA_BYTES], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn mutated_word(
    mut bytes: [u8; TYPED_CALL_DATA_BYTES],
    offset: usize,
    value: u64,
) -> [u8; TYPED_CALL_DATA_BYTES] {
    put_word(&mut bytes, offset, value);
    bytes
}

fn mutated_range(
    mut bytes: [u8; TYPED_CALL_DATA_BYTES],
    offset: usize,
    value: &[u8],
) -> [u8; TYPED_CALL_DATA_BYTES] {
    bytes[offset..offset + value.len()].copy_from_slice(value);
    bytes
}
