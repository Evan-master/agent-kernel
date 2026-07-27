//! Allocation-free TPM 2.0 wire encoding for the provisioned signer.
//!
//! The x86 TPM layer owns fixed command layouts and strict response parsing;
//! it accepts only the algorithms and bounded structures used through V20.

mod cursor;
mod decode;
mod encode;

pub(crate) use decode::recover_start_policy_session_handle;
pub use decode::{
    parse_command_success, parse_p256_policy_signature_response, parse_p256_signature_response,
    parse_read_public_response, parse_start_policy_session_response, ReadPublicResponse,
    MAX_TPM_NAME_BYTES, MAX_TPM_PUBLIC_BYTES,
};
pub use encode::{
    encode_flush_context, encode_policy_command_code, encode_policy_pcr,
    encode_policy_sign_p256_digest, encode_read_public, encode_sign_p256_digest,
    encode_start_policy_session, FLUSH_CONTEXT_COMMAND_BYTES, POLICY_COMMAND_CODE_BYTES,
    POLICY_PCR_COMMAND_BYTES, READ_PUBLIC_COMMAND_BYTES, SIGN_COMMAND_BYTES,
    START_POLICY_SESSION_COMMAND_BYTES,
};

const TPM_ST_NO_SESSIONS: u16 = 0x8001;
const TPM_ST_SESSIONS: u16 = 0x8002;
const TPM_ALG_SHA256: u16 = 0x000b;
const TPM_ALG_ECDSA: u16 = 0x0018;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum DigestSignCommand {
    SignDigestV185 = 1,
    SignV184 = 2,
}

impl DigestSignCommand {
    pub const fn command_code(self) -> u32 {
        match self {
            Self::SignDigestV185 => 0x0000_01a6,
            Self::SignV184 => 0x0000_015d,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct TpmPersistentHandle(u32);

impl TpmPersistentHandle {
    pub const MIN: u32 = 0x8100_0000;
    pub const MAX: u32 = 0x81ff_ffff;

    pub const fn new(handle: u32) -> Option<Self> {
        if handle >= Self::MIN && handle <= Self::MAX {
            Some(Self(handle))
        } else {
            None
        }
    }

    pub const fn get(self) -> u32 {
        self.0
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct TpmPolicySessionHandle(u32);

impl TpmPolicySessionHandle {
    pub const MIN: u32 = 0x0300_0000;
    pub const MAX: u32 = 0x03ff_ffff;

    pub const fn new(handle: u32) -> Option<Self> {
        if handle >= Self::MIN && handle <= Self::MAX {
            Some(Self(handle))
        } else {
            None
        }
    }

    pub const fn get(self) -> u32 {
        self.0
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TpmWireError {
    Truncated,
    ResponseLengthMismatch { declared: usize, actual: usize },
    UnexpectedTag { expected: u16, actual: u16 },
    TpmResponseCode(u32),
    PublicAreaTooLarge { declared: usize },
    NameTooLarge { declared: usize },
    QualifiedNameTooLarge { declared: usize },
    ParameterSizeMismatch,
    InvalidSignatureAlgorithm { algorithm: u16 },
    InvalidSignatureHash { algorithm: u16 },
    InvalidSignatureScalar,
    InvalidAuthorizationResponse,
    InvalidPolicySessionHandle(u32),
    InvalidPolicySessionNonce { declared: usize },
    TrailingBytes,
}
