#![allow(dead_code)]

use std::collections::VecDeque;

use agent_kernel_hal::TpmCommandTransport;
use agent_kernel_x86_64::tpm2::TpmPolicySessionHandle;
use p256::ecdsa::{Signature, SigningKey};
use sha2::{Digest, Sha256};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TransportError {
    Exhausted,
    OutputTooSmall,
}

pub struct ScriptedTpm {
    responses: VecDeque<Vec<u8>>,
    commands: Vec<Vec<u8>>,
}

impl ScriptedTpm {
    pub fn new(responses: impl IntoIterator<Item = Vec<u8>>) -> Self {
        Self {
            responses: responses.into_iter().collect(),
            commands: Vec::new(),
        }
    }

    pub fn commands(&self) -> &[Vec<u8>] {
        &self.commands
    }
}

impl TpmCommandTransport for ScriptedTpm {
    type Error = TransportError;

    fn execute(&mut self, command: &[u8], response: &mut [u8]) -> Result<usize, Self::Error> {
        self.commands.push(command.to_vec());
        let next = self
            .responses
            .pop_front()
            .ok_or(TransportError::Exhausted)?;
        if response.len() < next.len() {
            return Err(TransportError::OutputTooSmall);
        }
        response[..next.len()].copy_from_slice(&next);
        Ok(next.len())
    }
}

pub struct PublicFixture {
    pub response: Vec<u8>,
    pub name: [u8; 34],
    pub compressed: [u8; 33],
}

pub fn public_fixture(key: &SigningKey, attributes: u32) -> PublicFixture {
    public_fixture_with_auth_policy(key, attributes, &[])
}

pub fn policy_public_fixture(
    key: &SigningKey,
    attributes: u32,
    auth_policy: [u8; 32],
) -> PublicFixture {
    public_fixture_with_auth_policy(key, attributes, &auth_policy)
}

fn public_fixture_with_auth_policy(
    key: &SigningKey,
    attributes: u32,
    auth_policy: &[u8],
) -> PublicFixture {
    let compressed_point = key.verifying_key().to_encoded_point(true);
    let mut compressed = [0; 33];
    compressed.copy_from_slice(compressed_point.as_bytes());
    let point = key.verifying_key().to_encoded_point(false);
    let mut public = Vec::with_capacity(88 + auth_policy.len());
    public.extend_from_slice(&0x0023_u16.to_be_bytes());
    public.extend_from_slice(&0x000b_u16.to_be_bytes());
    public.extend_from_slice(&attributes.to_be_bytes());
    public.extend_from_slice(&(auth_policy.len() as u16).to_be_bytes());
    public.extend_from_slice(auth_policy);
    public.extend_from_slice(&0x0010_u16.to_be_bytes());
    public.extend_from_slice(&0x0018_u16.to_be_bytes());
    public.extend_from_slice(&0x000b_u16.to_be_bytes());
    public.extend_from_slice(&0x0003_u16.to_be_bytes());
    public.extend_from_slice(&0x0010_u16.to_be_bytes());
    public.extend_from_slice(&32_u16.to_be_bytes());
    public.extend_from_slice(point.x().expect("uncompressed x"));
    public.extend_from_slice(&32_u16.to_be_bytes());
    public.extend_from_slice(point.y().expect("uncompressed y"));

    let mut name = [0; 34];
    name[..2].copy_from_slice(&0x000b_u16.to_be_bytes());
    name[2..].copy_from_slice(&Sha256::digest(&public));

    let length = 10 + 2 + public.len() + 2 + name.len() + 2;
    let mut response = Vec::with_capacity(length);
    response.extend_from_slice(&0x8001_u16.to_be_bytes());
    response.extend_from_slice(&(length as u32).to_be_bytes());
    response.extend_from_slice(&0_u32.to_be_bytes());
    response.extend_from_slice(&(public.len() as u16).to_be_bytes());
    response.extend_from_slice(&public);
    response.extend_from_slice(&(name.len() as u16).to_be_bytes());
    response.extend_from_slice(&name);
    response.extend_from_slice(&0_u16.to_be_bytes());
    PublicFixture {
        response,
        name,
        compressed,
    }
}

pub fn signature_response(signature: Signature) -> Vec<u8> {
    signature_response_with_authorization(signature, &[], 1)
}

pub fn policy_signature_response(signature: Signature, nonce: [u8; 16]) -> Vec<u8> {
    signature_response_with_authorization(signature, &nonce, 1)
}

fn signature_response_with_authorization(
    signature: Signature,
    nonce: &[u8],
    attributes: u8,
) -> Vec<u8> {
    let encoded = signature.to_bytes();
    let mut parameters = Vec::with_capacity(72);
    parameters.extend_from_slice(&0x0018_u16.to_be_bytes());
    parameters.extend_from_slice(&0x000b_u16.to_be_bytes());
    parameters.extend_from_slice(&32_u16.to_be_bytes());
    parameters.extend_from_slice(&encoded[..32]);
    parameters.extend_from_slice(&32_u16.to_be_bytes());
    parameters.extend_from_slice(&encoded[32..]);

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

pub fn policy_session_response(handle: TpmPolicySessionHandle, nonce: [u8; 16]) -> Vec<u8> {
    let mut response = Vec::with_capacity(32);
    response.extend_from_slice(&0x8001_u16.to_be_bytes());
    response.extend_from_slice(&32_u32.to_be_bytes());
    response.extend_from_slice(&0_u32.to_be_bytes());
    response.extend_from_slice(&handle.get().to_be_bytes());
    response.extend_from_slice(&16_u16.to_be_bytes());
    response.extend_from_slice(&nonce);
    response
}

pub fn command_success() -> Vec<u8> {
    vec![0x80, 0x01, 0, 0, 0, 10, 0, 0, 0, 0]
}

pub fn command_error(code: u32) -> Vec<u8> {
    let mut response = Vec::with_capacity(10);
    response.extend_from_slice(&0x8001_u16.to_be_bytes());
    response.extend_from_slice(&10_u32.to_be_bytes());
    response.extend_from_slice(&code.to_be_bytes());
    response
}
