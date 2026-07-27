use agent_kernel_x86_64::tpm2::{
    encode_flush_context, encode_policy_command_code, encode_policy_pcr,
    encode_policy_sign_p256_digest, encode_start_policy_session, parse_command_success,
    parse_p256_policy_signature_response, parse_start_policy_session_response, DigestSignCommand,
    Sha256PcrPolicy, TpmPcrPolicyError, TpmPersistentHandle, TpmPolicySessionHandle, TpmWireError,
};

const KEY_HANDLE: TpmPersistentHandle =
    TpmPersistentHandle::new(0x8101_0001).expect("persistent handle");
const SESSION_HANDLE: TpmPolicySessionHandle =
    TpmPolicySessionHandle::new(0x0300_0042).expect("policy session handle");
const PCR_SELECTION: [u8; 3] = [0x81, 0x08, 0x00];
const PCR_DIGEST: [u8; 32] = [0xa5; 32];
const MANIFEST_DIGEST: [u8; 32] = [0x5a; 32];
const NONCE: [u8; 16] = [0xc3; 16];

#[test]
fn policy_configuration_rejects_an_empty_pcr_selection() {
    assert_eq!(
        Sha256PcrPolicy::new([0; 3], PCR_DIGEST),
        Err(TpmPcrPolicyError::EmptySelection)
    );
}

#[test]
fn policy_digest_vectors_bind_pcrs_and_the_signing_command() {
    let policy = policy();
    assert_eq!(
        policy.authorization_digest(DigestSignCommand::SignDigestV185),
        hex32("acd40828fc7ac8d7fce55172c045a7dffa296af388510854c7063b5c36f90792")
    );
    assert_eq!(
        policy.authorization_digest(DigestSignCommand::SignV184),
        hex32("5aa06717fbd83def1fb44ae3f7731bc6b5e2dfd97ca3ab23755086618786c382")
    );
}

#[test]
fn policy_session_commands_are_byte_exact() {
    assert_eq!(
        encode_start_policy_session(NONCE),
        start_policy_session_command()
    );
    assert_eq!(
        encode_policy_pcr(SESSION_HANDLE, policy()),
        policy_pcr_command()
    );
    assert_eq!(
        encode_policy_command_code(SESSION_HANDLE, DigestSignCommand::SignDigestV185),
        [0x80, 0x01, 0, 0, 0, 0x12, 0, 0, 0x01, 0x6c, 0x03, 0, 0, 0x42, 0, 0, 0x01, 0xa6,]
    );
    assert_eq!(
        encode_flush_context(SESSION_HANDLE),
        [0x80, 0x01, 0, 0, 0, 0x0e, 0, 0, 0x01, 0x65, 0x03, 0, 0, 0x42,]
    );
}

#[test]
fn policy_authorized_signing_uses_the_session_and_keeps_it_for_cleanup() {
    let command = encode_policy_sign_p256_digest(
        KEY_HANDLE,
        MANIFEST_DIGEST,
        DigestSignCommand::SignDigestV185,
        SESSION_HANDLE,
    );

    assert_eq!(&command[18..22], &SESSION_HANDLE.get().to_be_bytes());
    assert_eq!(&command[22..27], &[0, 0, 1, 0, 0]);
    assert_eq!(&command[31..63], &MANIFEST_DIGEST);
}

#[test]
fn policy_session_responses_are_strictly_bounded() {
    let response = start_policy_session_response(SESSION_HANDLE, NONCE);
    assert_eq!(
        parse_start_policy_session_response(&response).unwrap(),
        SESSION_HANDLE
    );
    assert_eq!(
        parse_command_success(&[0x80, 0x01, 0, 0, 0, 10, 0, 0, 0, 0]),
        Ok(())
    );

    let mut bad_handle = response.clone();
    bad_handle[10] = 0x02;
    assert_eq!(
        parse_start_policy_session_response(&bad_handle),
        Err(TpmWireError::InvalidPolicySessionHandle(0x0200_0042))
    );

    let mut trailing = vec![0x80, 0x01, 0, 0, 0, 11, 0, 0, 0, 0];
    trailing.push(0);
    assert_eq!(
        parse_command_success(&trailing),
        Err(TpmWireError::TrailingBytes)
    );
}

#[test]
fn policy_signature_response_accepts_only_its_session_acknowledgment() {
    let response = signature_response([1; 32], [2; 32], &NONCE, 1);
    assert!(parse_p256_policy_signature_response(&response).is_ok());

    let bad_attributes = signature_response([1; 32], [2; 32], &NONCE, 0);
    assert_eq!(
        parse_p256_policy_signature_response(&bad_attributes),
        Err(TpmWireError::InvalidAuthorizationResponse)
    );

    let oversized_nonce = signature_response([1; 32], [2; 32], &[3; 17], 1);
    assert_eq!(
        parse_p256_policy_signature_response(&oversized_nonce),
        Err(TpmWireError::InvalidAuthorizationResponse)
    );
}

fn policy() -> Sha256PcrPolicy {
    Sha256PcrPolicy::new(PCR_SELECTION, PCR_DIGEST).unwrap()
}

fn start_policy_session_command() -> [u8; 43] {
    let mut command = [0; 43];
    command[0..2].copy_from_slice(&0x8001_u16.to_be_bytes());
    command[2..6].copy_from_slice(&43_u32.to_be_bytes());
    command[6..10].copy_from_slice(&0x0000_0176_u32.to_be_bytes());
    command[10..14].copy_from_slice(&0x4000_0007_u32.to_be_bytes());
    command[14..18].copy_from_slice(&0x4000_0007_u32.to_be_bytes());
    command[18..20].copy_from_slice(&16_u16.to_be_bytes());
    command[20..36].copy_from_slice(&NONCE);
    command[38] = 1;
    command[39..41].copy_from_slice(&0x0010_u16.to_be_bytes());
    command[41..43].copy_from_slice(&0x000b_u16.to_be_bytes());
    command
}

fn policy_pcr_command() -> [u8; 58] {
    let mut command = [0; 58];
    command[0..2].copy_from_slice(&0x8001_u16.to_be_bytes());
    command[2..6].copy_from_slice(&58_u32.to_be_bytes());
    command[6..10].copy_from_slice(&0x0000_017f_u32.to_be_bytes());
    command[10..14].copy_from_slice(&SESSION_HANDLE.get().to_be_bytes());
    command[14..16].copy_from_slice(&32_u16.to_be_bytes());
    command[16..48].copy_from_slice(&PCR_DIGEST);
    command[48..52].copy_from_slice(&1_u32.to_be_bytes());
    command[52..54].copy_from_slice(&0x000b_u16.to_be_bytes());
    command[54] = 3;
    command[55..58].copy_from_slice(&PCR_SELECTION);
    command
}

fn start_policy_session_response(handle: TpmPolicySessionHandle, nonce: [u8; 16]) -> Vec<u8> {
    let mut response = Vec::with_capacity(32);
    response.extend_from_slice(&0x8001_u16.to_be_bytes());
    response.extend_from_slice(&32_u32.to_be_bytes());
    response.extend_from_slice(&0_u32.to_be_bytes());
    response.extend_from_slice(&handle.get().to_be_bytes());
    response.extend_from_slice(&16_u16.to_be_bytes());
    response.extend_from_slice(&nonce);
    response
}

fn signature_response(r: [u8; 32], s: [u8; 32], nonce: &[u8], attributes: u8) -> Vec<u8> {
    let mut parameters = Vec::with_capacity(72);
    parameters.extend_from_slice(&0x0018_u16.to_be_bytes());
    parameters.extend_from_slice(&0x000b_u16.to_be_bytes());
    parameters.extend_from_slice(&32_u16.to_be_bytes());
    parameters.extend_from_slice(&r);
    parameters.extend_from_slice(&32_u16.to_be_bytes());
    parameters.extend_from_slice(&s);

    let length = 10 + 4 + parameters.len() + 2 + nonce.len() + 1 + 2;
    let mut response = Vec::with_capacity(length);
    response.extend_from_slice(&0x8002_u16.to_be_bytes());
    response.extend_from_slice(&(length as u32).to_be_bytes());
    response.extend_from_slice(&0_u32.to_be_bytes());
    response.extend_from_slice(&(parameters.len() as u32).to_be_bytes());
    response.extend_from_slice(&parameters);
    response.extend_from_slice(&(nonce.len() as u16).to_be_bytes());
    response.extend_from_slice(nonce);
    response.push(attributes);
    response.extend_from_slice(&0_u16.to_be_bytes());
    response
}

fn hex32(encoded: &str) -> [u8; 32] {
    let mut output = [0; 32];
    for (index, byte) in output.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&encoded[index * 2..index * 2 + 2], 16).unwrap();
    }
    output
}
