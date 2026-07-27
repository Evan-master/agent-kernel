//! Fixed-layout encoders for the bounded TPM signer command set.
//!
//! V20 adds a fresh PCR policy session while preserving the V19 empty-password
//! signing path.

use crate::tpm2::Sha256PcrPolicy;

use super::{
    DigestSignCommand, TpmPersistentHandle, TpmPolicySessionHandle, TPM_ST_NO_SESSIONS,
    TPM_ST_SESSIONS,
};

pub const READ_PUBLIC_COMMAND_BYTES: usize = 14;
pub const SIGN_COMMAND_BYTES: usize = 71;
pub const START_POLICY_SESSION_COMMAND_BYTES: usize = 43;
pub const POLICY_PCR_COMMAND_BYTES: usize = 58;
pub const POLICY_COMMAND_CODE_BYTES: usize = 18;
pub const FLUSH_CONTEXT_COMMAND_BYTES: usize = 14;

const TPM_CC_READ_PUBLIC: u32 = 0x0000_0173;
const TPM_CC_START_AUTH_SESSION: u32 = 0x0000_0176;
const TPM_CC_POLICY_PCR: u32 = 0x0000_017f;
const TPM_CC_POLICY_COMMAND_CODE: u32 = 0x0000_016c;
const TPM_CC_FLUSH_CONTEXT: u32 = 0x0000_0165;
const TPM_RS_PW: u32 = 0x4000_0009;
const TPM_ALG_NULL: u16 = 0x0010;
const TPM_ALG_SHA256: u16 = 0x000b;
const TPM_ST_HASHCHECK: u16 = 0x8024;
const TPM_RH_NULL: u32 = 0x4000_0007;
const TPM_SE_POLICY: u8 = 1;
const AUTHORIZATION_BYTES: u32 = 9;
const CONTINUE_SESSION: u8 = 1;

pub fn encode_read_public(handle: TpmPersistentHandle) -> [u8; READ_PUBLIC_COMMAND_BYTES] {
    let mut command = [0; READ_PUBLIC_COMMAND_BYTES];
    command[0..2].copy_from_slice(&TPM_ST_NO_SESSIONS.to_be_bytes());
    command[2..6].copy_from_slice(&(READ_PUBLIC_COMMAND_BYTES as u32).to_be_bytes());
    command[6..10].copy_from_slice(&TPM_CC_READ_PUBLIC.to_be_bytes());
    command[10..14].copy_from_slice(&handle.get().to_be_bytes());
    command
}

pub fn encode_sign_p256_digest(
    handle: TpmPersistentHandle,
    digest: [u8; 32],
    mode: DigestSignCommand,
) -> [u8; SIGN_COMMAND_BYTES] {
    encode_sign(handle, digest, mode, TPM_RS_PW, 0)
}

pub fn encode_policy_sign_p256_digest(
    handle: TpmPersistentHandle,
    digest: [u8; 32],
    mode: DigestSignCommand,
    session: TpmPolicySessionHandle,
) -> [u8; SIGN_COMMAND_BYTES] {
    encode_sign(handle, digest, mode, session.get(), CONTINUE_SESSION)
}

pub fn encode_start_policy_session(nonce: [u8; 16]) -> [u8; START_POLICY_SESSION_COMMAND_BYTES] {
    let mut command = [0; START_POLICY_SESSION_COMMAND_BYTES];
    write_header(&mut command, TPM_ST_NO_SESSIONS, TPM_CC_START_AUTH_SESSION);
    command[10..14].copy_from_slice(&TPM_RH_NULL.to_be_bytes());
    command[14..18].copy_from_slice(&TPM_RH_NULL.to_be_bytes());
    command[18..20].copy_from_slice(&(nonce.len() as u16).to_be_bytes());
    command[20..36].copy_from_slice(&nonce);
    command[38] = TPM_SE_POLICY;
    command[39..41].copy_from_slice(&TPM_ALG_NULL.to_be_bytes());
    command[41..43].copy_from_slice(&TPM_ALG_SHA256.to_be_bytes());
    command
}

pub fn encode_policy_pcr(
    session: TpmPolicySessionHandle,
    policy: Sha256PcrPolicy,
) -> [u8; POLICY_PCR_COMMAND_BYTES] {
    let mut command = [0; POLICY_PCR_COMMAND_BYTES];
    write_header(&mut command, TPM_ST_NO_SESSIONS, TPM_CC_POLICY_PCR);
    command[10..14].copy_from_slice(&session.get().to_be_bytes());
    command[14..16].copy_from_slice(&32_u16.to_be_bytes());
    command[16..48].copy_from_slice(&policy.expected_digest());
    command[48..58].copy_from_slice(&policy.marshalled_selection());
    command
}

pub fn encode_policy_command_code(
    session: TpmPolicySessionHandle,
    mode: DigestSignCommand,
) -> [u8; POLICY_COMMAND_CODE_BYTES] {
    let mut command = [0; POLICY_COMMAND_CODE_BYTES];
    write_header(&mut command, TPM_ST_NO_SESSIONS, TPM_CC_POLICY_COMMAND_CODE);
    command[10..14].copy_from_slice(&session.get().to_be_bytes());
    command[14..18].copy_from_slice(&mode.command_code().to_be_bytes());
    command
}

pub fn encode_flush_context(session: TpmPolicySessionHandle) -> [u8; FLUSH_CONTEXT_COMMAND_BYTES] {
    let mut command = [0; FLUSH_CONTEXT_COMMAND_BYTES];
    write_header(&mut command, TPM_ST_NO_SESSIONS, TPM_CC_FLUSH_CONTEXT);
    command[10..14].copy_from_slice(&session.get().to_be_bytes());
    command
}

fn encode_sign(
    handle: TpmPersistentHandle,
    digest: [u8; 32],
    mode: DigestSignCommand,
    authorization_handle: u32,
    session_attributes: u8,
) -> [u8; SIGN_COMMAND_BYTES] {
    let mut command = [0; SIGN_COMMAND_BYTES];
    write_header(&mut command, TPM_ST_SESSIONS, mode.command_code());
    command[10..14].copy_from_slice(&handle.get().to_be_bytes());
    command[14..18].copy_from_slice(&AUTHORIZATION_BYTES.to_be_bytes());
    command[18..22].copy_from_slice(&authorization_handle.to_be_bytes());
    command[24] = session_attributes;

    match mode {
        DigestSignCommand::SignDigestV185 => {
            write_digest(&mut command, 29, digest);
        }
        DigestSignCommand::SignV184 => {
            write_digest(&mut command, 27, digest);
            command[61..63].copy_from_slice(&TPM_ALG_NULL.to_be_bytes());
        }
    }
    command[63..65].copy_from_slice(&TPM_ST_HASHCHECK.to_be_bytes());
    command[65..69].copy_from_slice(&TPM_RH_NULL.to_be_bytes());
    command
}

fn write_header<const N: usize>(command: &mut [u8; N], tag: u16, command_code: u32) {
    command[0..2].copy_from_slice(&tag.to_be_bytes());
    command[2..6].copy_from_slice(&(N as u32).to_be_bytes());
    command[6..10].copy_from_slice(&command_code.to_be_bytes());
}

fn write_digest(command: &mut [u8; SIGN_COMMAND_BYTES], offset: usize, digest: [u8; 32]) {
    command[offset..offset + 2].copy_from_slice(&(digest.len() as u16).to_be_bytes());
    command[offset + 2..offset + 34].copy_from_slice(&digest);
}
