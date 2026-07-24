use agent_kernel_x86_64::tpm2::{
    encode_read_public, encode_sign_p256_digest, parse_p256_signature_response,
    parse_read_public_response, DigestSignCommand, TpmPersistentHandle, TpmWireError,
};

const HANDLE: TpmPersistentHandle =
    TpmPersistentHandle::new(0x8101_0001).expect("persistent handle");
const DIGEST: [u8; 32] = [0x5a; 32];

#[test]
fn read_public_command_is_byte_exact() {
    assert_eq!(
        encode_read_public(HANDLE),
        [0x80, 0x01, 0, 0, 0, 0x0e, 0, 0, 0x01, 0x73, 0x81, 0x01, 0, 0x01,]
    );
}

#[test]
fn sign_digest_v185_command_is_byte_exact() {
    let command = encode_sign_p256_digest(HANDLE, DIGEST, DigestSignCommand::SignDigestV185);
    let mut expected = common_sign_prefix(0x01a6);
    expected[27..29].copy_from_slice(&[0, 0]);
    expected[29..63].copy_from_slice(&digest_field());
    expected[63..].copy_from_slice(&null_hashcheck_ticket());

    assert_eq!(command, expected);
}

#[test]
fn sign_v184_command_is_byte_exact() {
    let command = encode_sign_p256_digest(HANDLE, DIGEST, DigestSignCommand::SignV184);
    let mut expected = common_sign_prefix(0x015d);
    expected[27..61].copy_from_slice(&digest_field());
    expected[61..63].copy_from_slice(&[0, 0x10]);
    expected[63..].copy_from_slice(&null_hashcheck_ticket());

    assert_eq!(command, expected);
}

#[test]
fn read_public_response_retains_exact_public_and_names() {
    let public = p256_public_area();
    let name = [0x44; 34];
    let response = read_public_response(&public, &name);

    let decoded = parse_read_public_response(&response).unwrap();

    assert_eq!(decoded.public_area(), public);
    assert_eq!(decoded.name(), name);
    assert!(decoded.qualified_name().is_empty());
}

#[test]
fn signature_response_converts_to_p1363_and_normalizes_high_s() {
    let mut r = [0; 32];
    r[31] = 1;
    let high_s = [
        0xff, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xbc, 0xe6, 0xfa, 0xad, 0xa7, 0x17, 0x9e, 0x84, 0xf3, 0xb9, 0xca, 0xc2, 0xfc, 0x63,
        0x25, 0x50,
    ];
    let response = signature_response(r, high_s);

    let signature = parse_p256_signature_response(&response).unwrap();

    assert_eq!(&signature[..32], &r);
    assert_eq!(&signature[32..63], &[0; 31]);
    assert_eq!(signature[63], 1);
}

#[test]
fn response_parser_rejects_tpm_errors_bad_sessions_and_invalid_scalars() {
    let error_response = [0x80, 0x01, 0, 0, 0, 10, 0, 0, 1, 0x01];
    assert_eq!(
        parse_p256_signature_response(&error_response),
        Err(TpmWireError::TpmResponseCode(0x101))
    );

    let mut bad_session = signature_response([1; 32], [2; 32]);
    let last = bad_session.len() - 1;
    bad_session[last] = 1;
    assert_eq!(
        parse_p256_signature_response(&bad_session),
        Err(TpmWireError::InvalidAuthorizationResponse)
    );

    let invalid_scalar = signature_response([0; 32], [2; 32]);
    assert_eq!(
        parse_p256_signature_response(&invalid_scalar),
        Err(TpmWireError::InvalidSignatureScalar)
    );
}

fn common_sign_prefix(command_code: u32) -> [u8; 71] {
    let mut command = [0; 71];
    command[0..2].copy_from_slice(&0x8002_u16.to_be_bytes());
    command[2..6].copy_from_slice(&71_u32.to_be_bytes());
    command[6..10].copy_from_slice(&command_code.to_be_bytes());
    command[10..14].copy_from_slice(&HANDLE.get().to_be_bytes());
    command[14..18].copy_from_slice(&9_u32.to_be_bytes());
    command[18..22].copy_from_slice(&0x4000_0009_u32.to_be_bytes());
    command
}

fn digest_field() -> [u8; 34] {
    let mut field = [0; 34];
    field[..2].copy_from_slice(&32_u16.to_be_bytes());
    field[2..].copy_from_slice(&DIGEST);
    field
}

fn null_hashcheck_ticket() -> [u8; 8] {
    [0x80, 0x24, 0x40, 0, 0, 0x07, 0, 0]
}

fn p256_public_area() -> [u8; 88] {
    let mut public = [0; 88];
    public[0..2].copy_from_slice(&0x0023_u16.to_be_bytes());
    public[2..4].copy_from_slice(&0x000b_u16.to_be_bytes());
    public[4..8].copy_from_slice(&0x0004_0072_u32.to_be_bytes());
    public[10..12].copy_from_slice(&0x0010_u16.to_be_bytes());
    public[12..14].copy_from_slice(&0x0018_u16.to_be_bytes());
    public[14..16].copy_from_slice(&0x000b_u16.to_be_bytes());
    public[16..18].copy_from_slice(&0x0003_u16.to_be_bytes());
    public[18..20].copy_from_slice(&0x0010_u16.to_be_bytes());
    public[20..22].copy_from_slice(&32_u16.to_be_bytes());
    public[22..54].fill(0x21);
    public[54..56].copy_from_slice(&32_u16.to_be_bytes());
    public[56..88].fill(0x42);
    public
}

fn read_public_response(public: &[u8], name: &[u8]) -> Vec<u8> {
    let length = 10 + 2 + public.len() + 2 + name.len() + 2;
    let mut response = Vec::with_capacity(length);
    response.extend_from_slice(&0x8001_u16.to_be_bytes());
    response.extend_from_slice(&(length as u32).to_be_bytes());
    response.extend_from_slice(&0_u32.to_be_bytes());
    response.extend_from_slice(&(public.len() as u16).to_be_bytes());
    response.extend_from_slice(public);
    response.extend_from_slice(&(name.len() as u16).to_be_bytes());
    response.extend_from_slice(name);
    response.extend_from_slice(&0_u16.to_be_bytes());
    response
}

fn signature_response(r: [u8; 32], s: [u8; 32]) -> Vec<u8> {
    let mut parameters = Vec::with_capacity(72);
    parameters.extend_from_slice(&0x0018_u16.to_be_bytes());
    parameters.extend_from_slice(&0x000b_u16.to_be_bytes());
    parameters.extend_from_slice(&32_u16.to_be_bytes());
    parameters.extend_from_slice(&r);
    parameters.extend_from_slice(&32_u16.to_be_bytes());
    parameters.extend_from_slice(&s);

    let length = 10 + 4 + parameters.len() + 5;
    let mut response = Vec::with_capacity(length);
    response.extend_from_slice(&0x8002_u16.to_be_bytes());
    response.extend_from_slice(&(length as u32).to_be_bytes());
    response.extend_from_slice(&0_u32.to_be_bytes());
    response.extend_from_slice(&(parameters.len() as u32).to_be_bytes());
    response.extend_from_slice(&parameters);
    response.extend_from_slice(&[0, 0, 0, 0, 0]);
    response
}
