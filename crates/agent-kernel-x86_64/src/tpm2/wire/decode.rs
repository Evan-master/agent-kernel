//! Strict decoders for ReadPublic and P-256 signature responses.
//!
//! The x86 wire layer retains bounded public data and converts only canonical
//! successful TPM responses into kernel-facing values.

use p256::ecdsa::Signature;

use super::cursor::Cursor;
use super::{
    TpmPolicySessionHandle, TpmWireError, TPM_ALG_ECDSA, TPM_ALG_SHA256, TPM_ST_NO_SESSIONS,
    TPM_ST_SESSIONS,
};

pub const MAX_TPM_PUBLIC_BYTES: usize = 512;
pub const MAX_TPM_NAME_BYTES: usize = 68;
const POLICY_NONCE_BYTES: usize = 16;
const CONTINUE_SESSION: u8 = 1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadPublicResponse {
    public_area: [u8; MAX_TPM_PUBLIC_BYTES],
    public_area_length: usize,
    name: [u8; MAX_TPM_NAME_BYTES],
    name_length: usize,
    qualified_name: [u8; MAX_TPM_NAME_BYTES],
    qualified_name_length: usize,
}

impl ReadPublicResponse {
    pub fn public_area(&self) -> &[u8] {
        &self.public_area[..self.public_area_length]
    }

    pub fn name(&self) -> &[u8] {
        &self.name[..self.name_length]
    }

    pub fn qualified_name(&self) -> &[u8] {
        &self.qualified_name[..self.qualified_name_length]
    }
}

pub fn parse_read_public_response(response: &[u8]) -> Result<ReadPublicResponse, TpmWireError> {
    let parameters = success_parameters(response, TPM_ST_NO_SESSIONS)?;
    let mut cursor = Cursor::new(parameters);
    let public = cursor.tpm2b()?;
    if public.len() > MAX_TPM_PUBLIC_BYTES {
        return Err(TpmWireError::PublicAreaTooLarge {
            declared: public.len(),
        });
    }
    let name = cursor.tpm2b()?;
    if name.len() > MAX_TPM_NAME_BYTES {
        return Err(TpmWireError::NameTooLarge {
            declared: name.len(),
        });
    }
    let qualified_name = cursor.tpm2b()?;
    if qualified_name.len() > MAX_TPM_NAME_BYTES {
        return Err(TpmWireError::QualifiedNameTooLarge {
            declared: qualified_name.len(),
        });
    }
    if !cursor.is_finished() {
        return Err(TpmWireError::TrailingBytes);
    }

    let mut decoded = ReadPublicResponse {
        public_area: [0; MAX_TPM_PUBLIC_BYTES],
        public_area_length: public.len(),
        name: [0; MAX_TPM_NAME_BYTES],
        name_length: name.len(),
        qualified_name: [0; MAX_TPM_NAME_BYTES],
        qualified_name_length: qualified_name.len(),
    };
    decoded.public_area[..public.len()].copy_from_slice(public);
    decoded.name[..name.len()].copy_from_slice(name);
    decoded.qualified_name[..qualified_name.len()].copy_from_slice(qualified_name);
    Ok(decoded)
}

pub fn parse_p256_signature_response(response: &[u8]) -> Result<[u8; 64], TpmWireError> {
    parse_authorized_signature(response, AuthorizationResponse::Password)
}

pub fn parse_p256_policy_signature_response(response: &[u8]) -> Result<[u8; 64], TpmWireError> {
    parse_authorized_signature(response, AuthorizationResponse::Policy)
}

pub fn parse_start_policy_session_response(
    response: &[u8],
) -> Result<TpmPolicySessionHandle, TpmWireError> {
    let payload = success_parameters(response, TPM_ST_NO_SESSIONS)?;
    let mut cursor = Cursor::new(payload);
    let handle = parse_policy_session_handle(&mut cursor)?;
    let nonce = cursor.tpm2b()?;
    if nonce.len() != POLICY_NONCE_BYTES {
        return Err(TpmWireError::InvalidPolicySessionNonce {
            declared: nonce.len(),
        });
    }
    if !cursor.is_finished() {
        return Err(TpmWireError::TrailingBytes);
    }
    Ok(handle)
}

pub(crate) fn recover_start_policy_session_handle(
    response: &[u8],
) -> Option<TpmPolicySessionHandle> {
    let payload = success_parameters(response, TPM_ST_NO_SESSIONS).ok()?;
    parse_policy_session_handle(&mut Cursor::new(payload)).ok()
}

pub fn parse_command_success(response: &[u8]) -> Result<(), TpmWireError> {
    if !success_parameters(response, TPM_ST_NO_SESSIONS)?.is_empty() {
        return Err(TpmWireError::TrailingBytes);
    }
    Ok(())
}

fn parse_authorized_signature(
    response: &[u8],
    authorization_kind: AuthorizationResponse,
) -> Result<[u8; 64], TpmWireError> {
    let payload = success_parameters(response, TPM_ST_SESSIONS)?;
    let mut payload_cursor = Cursor::new(payload);
    let parameter_size =
        usize::try_from(payload_cursor.u32()?).map_err(|_| TpmWireError::ParameterSizeMismatch)?;
    if parameter_size > payload_cursor.remaining().len() {
        return Err(TpmWireError::ParameterSizeMismatch);
    }
    let parameters = payload_cursor.take(parameter_size)?;
    let authorization = payload_cursor.remaining();

    let signature = parse_p256_signature(parameters)?;
    if !authorization_kind.is_valid(authorization) {
        return Err(TpmWireError::InvalidAuthorizationResponse);
    }
    Ok(signature)
}

fn parse_policy_session_handle(
    cursor: &mut Cursor<'_>,
) -> Result<TpmPolicySessionHandle, TpmWireError> {
    let raw_handle = cursor.u32()?;
    TpmPolicySessionHandle::new(raw_handle)
        .ok_or(TpmWireError::InvalidPolicySessionHandle(raw_handle))
}

#[derive(Copy, Clone)]
enum AuthorizationResponse {
    Password,
    Policy,
}

impl AuthorizationResponse {
    fn is_valid(self, encoded: &[u8]) -> bool {
        let mut cursor = Cursor::new(encoded);
        let Ok(nonce) = cursor.tpm2b() else {
            return false;
        };
        let nonce_valid = match self {
            Self::Password => nonce.is_empty(),
            Self::Policy => nonce.len() == POLICY_NONCE_BYTES,
        };
        let Ok(attributes) = cursor.u8() else {
            return false;
        };
        let Ok(hmac) = cursor.tpm2b() else {
            return false;
        };
        nonce_valid && attributes == CONTINUE_SESSION && hmac.is_empty() && cursor.is_finished()
    }
}

fn success_parameters(response: &[u8], expected_tag: u16) -> Result<&[u8], TpmWireError> {
    let mut cursor = Cursor::new(response);
    let tag = cursor.u16()?;
    let declared =
        usize::try_from(cursor.u32()?).map_err(|_| TpmWireError::ResponseLengthMismatch {
            declared: usize::MAX,
            actual: response.len(),
        })?;
    if declared != response.len() {
        return Err(TpmWireError::ResponseLengthMismatch {
            declared,
            actual: response.len(),
        });
    }
    let response_code = cursor.u32()?;
    if response_code != 0 {
        return Err(TpmWireError::TpmResponseCode(response_code));
    }
    if tag != expected_tag {
        return Err(TpmWireError::UnexpectedTag {
            expected: expected_tag,
            actual: tag,
        });
    }
    Ok(cursor.remaining())
}

fn parse_p256_signature(parameters: &[u8]) -> Result<[u8; 64], TpmWireError> {
    let mut cursor = Cursor::new(parameters);
    let algorithm = cursor.u16()?;
    if algorithm != TPM_ALG_ECDSA {
        return Err(TpmWireError::InvalidSignatureAlgorithm { algorithm });
    }
    let hash = cursor.u16()?;
    if hash != TPM_ALG_SHA256 {
        return Err(TpmWireError::InvalidSignatureHash { algorithm: hash });
    }
    let r = p256_scalar(&mut cursor)?;
    let s = p256_scalar(&mut cursor)?;
    if !cursor.is_finished() {
        return Err(TpmWireError::TrailingBytes);
    }
    let signature =
        Signature::from_scalars(r, s).map_err(|_| TpmWireError::InvalidSignatureScalar)?;
    let normalized = signature.normalize_s().unwrap_or(signature);
    Ok(normalized.to_bytes().into())
}

fn p256_scalar(cursor: &mut Cursor<'_>) -> Result<[u8; 32], TpmWireError> {
    let encoded = cursor.tpm2b()?;
    if encoded.is_empty() || encoded.len() > 32 {
        return Err(TpmWireError::InvalidSignatureScalar);
    }
    let mut scalar = [0; 32];
    scalar[32 - encoded.len()..].copy_from_slice(encoded);
    Ok(scalar)
}
