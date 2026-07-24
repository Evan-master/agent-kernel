//! Fixed-layout encoders for the V19 TPM command set.
//!
//! The x86 wire layer emits ReadPublic and digest-signing commands with an
//! empty password session; object authorization must therefore remain empty.

use super::{DigestSignCommand, TpmPersistentHandle, TPM_ST_NO_SESSIONS, TPM_ST_SESSIONS};

pub const READ_PUBLIC_COMMAND_BYTES: usize = 14;
pub const SIGN_COMMAND_BYTES: usize = 71;

const TPM_CC_READ_PUBLIC: u32 = 0x0000_0173;
const TPM_CC_SIGN: u32 = 0x0000_015d;
const TPM_CC_SIGN_DIGEST: u32 = 0x0000_01a6;
const TPM_RS_PW: u32 = 0x4000_0009;
const TPM_ALG_NULL: u16 = 0x0010;
const TPM_ST_HASHCHECK: u16 = 0x8024;
const TPM_RH_NULL: u32 = 0x4000_0007;
const PASSWORD_AUTH_BYTES: u32 = 9;

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
    let mut command = [0; SIGN_COMMAND_BYTES];
    command[0..2].copy_from_slice(&TPM_ST_SESSIONS.to_be_bytes());
    command[2..6].copy_from_slice(&(SIGN_COMMAND_BYTES as u32).to_be_bytes());
    let command_code = match mode {
        DigestSignCommand::SignDigestV185 => TPM_CC_SIGN_DIGEST,
        DigestSignCommand::SignV184 => TPM_CC_SIGN,
    };
    command[6..10].copy_from_slice(&command_code.to_be_bytes());
    command[10..14].copy_from_slice(&handle.get().to_be_bytes());
    command[14..18].copy_from_slice(&PASSWORD_AUTH_BYTES.to_be_bytes());
    command[18..22].copy_from_slice(&TPM_RS_PW.to_be_bytes());

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

fn write_digest(command: &mut [u8; SIGN_COMMAND_BYTES], offset: usize, digest: [u8; 32]) {
    command[offset..offset + 2].copy_from_slice(&(digest.len() as u16).to_be_bytes());
    command[offset + 2..offset + 34].copy_from_slice(&digest);
}
