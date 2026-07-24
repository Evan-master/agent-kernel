//! Verification of the immutable TPM ECC signing-key template.
//!
//! This x86 policy boundary parses no dynamic allocations and binds the TPM
//! Name, attributes, scheme, curve, and full point to immutable boot policy.

use p256::PublicKey;
use sha2::{Digest, Sha256};

use super::ReadPublicResponse;

const TPM_ALG_SHA256: u16 = 0x000b;
const TPM_ALG_NULL: u16 = 0x0010;
const TPM_ALG_ECDSA: u16 = 0x0018;
const TPM_ALG_ECC: u16 = 0x0023;
const TPM_ECC_NIST_P256: u16 = 0x0003;

const ATTR_FIXED_TPM: u32 = 1 << 1;
const ATTR_FIXED_PARENT: u32 = 1 << 4;
const ATTR_SENSITIVE_DATA_ORIGIN: u32 = 1 << 5;
const ATTR_USER_WITH_AUTH: u32 = 1 << 6;
const ATTR_RESTRICTED: u32 = 1 << 16;
const ATTR_DECRYPT: u32 = 1 << 17;
const ATTR_SIGN_ENCRYPT: u32 = 1 << 18;
const ATTR_X509_SIGN: u32 = 1 << 19;
const REQUIRED_ATTRIBUTES: u32 = ATTR_FIXED_TPM
    | ATTR_FIXED_PARENT
    | ATTR_SENSITIVE_DATA_ORIGIN
    | ATTR_USER_WITH_AUTH
    | ATTR_SIGN_ENCRYPT;
const FORBIDDEN_ATTRIBUTES: u32 = ATTR_RESTRICTED | ATTR_DECRYPT | ATTR_X509_SIGN;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TpmPublicError {
    NameMismatch,
    NameDigestMismatch,
    Truncated,
    UnexpectedType { object_type: u16 },
    UnexpectedNameAlgorithm { algorithm: u16 },
    MissingAttributes { missing: u32 },
    ForbiddenAttributes { present: u32 },
    NonemptyAuthorizationPolicy,
    UnsupportedSymmetric { algorithm: u16 },
    UnsupportedScheme { algorithm: u16 },
    UnsupportedSchemeHash { algorithm: u16 },
    UnsupportedCurve { curve: u16 },
    UnsupportedKdf { algorithm: u16 },
    InvalidCoordinateSize,
    InvalidPoint,
    PublicKeyMismatch,
    TrailingBytes,
}

pub(super) fn verify_signing_public(
    response: &ReadPublicResponse,
    expected_name: [u8; 34],
    expected_public_key: [u8; 33],
) -> Result<(), TpmPublicError> {
    if response.name() != expected_name {
        return Err(TpmPublicError::NameMismatch);
    }
    let digest: [u8; 32] = Sha256::digest(response.public_area()).into();
    let mut computed_name = [0; 34];
    computed_name[..2].copy_from_slice(&TPM_ALG_SHA256.to_be_bytes());
    computed_name[2..].copy_from_slice(&digest);
    if response.name() != computed_name {
        return Err(TpmPublicError::NameDigestMismatch);
    }

    let mut cursor = PublicCursor::new(response.public_area());
    require(cursor.u16()?, TPM_ALG_ECC, |object_type| {
        TpmPublicError::UnexpectedType { object_type }
    })?;
    require(cursor.u16()?, TPM_ALG_SHA256, |algorithm| {
        TpmPublicError::UnexpectedNameAlgorithm { algorithm }
    })?;
    let attributes = cursor.u32()?;
    let missing = REQUIRED_ATTRIBUTES & !attributes;
    if missing != 0 {
        return Err(TpmPublicError::MissingAttributes { missing });
    }
    let present = FORBIDDEN_ATTRIBUTES & attributes;
    if present != 0 {
        return Err(TpmPublicError::ForbiddenAttributes { present });
    }
    if !cursor.tpm2b()?.is_empty() {
        return Err(TpmPublicError::NonemptyAuthorizationPolicy);
    }
    require(cursor.u16()?, TPM_ALG_NULL, |algorithm| {
        TpmPublicError::UnsupportedSymmetric { algorithm }
    })?;
    require(cursor.u16()?, TPM_ALG_ECDSA, |algorithm| {
        TpmPublicError::UnsupportedScheme { algorithm }
    })?;
    require(cursor.u16()?, TPM_ALG_SHA256, |algorithm| {
        TpmPublicError::UnsupportedSchemeHash { algorithm }
    })?;
    require(cursor.u16()?, TPM_ECC_NIST_P256, |curve| {
        TpmPublicError::UnsupportedCurve { curve }
    })?;
    require(cursor.u16()?, TPM_ALG_NULL, |algorithm| {
        TpmPublicError::UnsupportedKdf { algorithm }
    })?;
    let x = cursor.tpm2b()?;
    let y = cursor.tpm2b()?;
    if x.len() != 32 || y.len() != 32 {
        return Err(TpmPublicError::InvalidCoordinateSize);
    }
    if !cursor.finished() {
        return Err(TpmPublicError::TrailingBytes);
    }
    let mut uncompressed = [0; 65];
    uncompressed[0] = 0x04;
    uncompressed[1..33].copy_from_slice(x);
    uncompressed[33..].copy_from_slice(y);
    PublicKey::from_sec1_bytes(&uncompressed).map_err(|_| TpmPublicError::InvalidPoint)?;
    let mut compressed = [0; 33];
    compressed[0] = 0x02 | (y[31] & 1);
    compressed[1..].copy_from_slice(x);
    if compressed != expected_public_key {
        return Err(TpmPublicError::PublicKeyMismatch);
    }
    Ok(())
}

fn require(
    actual: u16,
    expected: u16,
    error: fn(u16) -> TpmPublicError,
) -> Result<(), TpmPublicError> {
    if actual != expected {
        return Err(error(actual));
    }
    Ok(())
}

struct PublicCursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> PublicCursor<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], TpmPublicError> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or(TpmPublicError::Truncated)?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or(TpmPublicError::Truncated)?;
        self.offset = end;
        Ok(value)
    }

    fn u16(&mut self) -> Result<u16, TpmPublicError> {
        let value = self.take(2)?;
        Ok(u16::from_be_bytes([value[0], value[1]]))
    }

    fn u32(&mut self) -> Result<u32, TpmPublicError> {
        let value = self.take(4)?;
        Ok(u32::from_be_bytes([value[0], value[1], value[2], value[3]]))
    }

    fn tpm2b(&mut self) -> Result<&'a [u8], TpmPublicError> {
        let length = usize::from(self.u16()?);
        self.take(length)
    }

    const fn finished(&self) -> bool {
        self.offset == self.bytes.len()
    }
}
