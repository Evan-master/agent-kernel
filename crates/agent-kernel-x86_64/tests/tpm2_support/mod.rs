use std::collections::VecDeque;

use agent_kernel_hal::TpmCommandTransport;
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
    let compressed_point = key.verifying_key().to_encoded_point(true);
    let mut compressed = [0; 33];
    compressed.copy_from_slice(compressed_point.as_bytes());
    let point = key.verifying_key().to_encoded_point(false);
    let mut public = [0; 88];
    public[0..2].copy_from_slice(&0x0023_u16.to_be_bytes());
    public[2..4].copy_from_slice(&0x000b_u16.to_be_bytes());
    public[4..8].copy_from_slice(&attributes.to_be_bytes());
    public[10..12].copy_from_slice(&0x0010_u16.to_be_bytes());
    public[12..14].copy_from_slice(&0x0018_u16.to_be_bytes());
    public[14..16].copy_from_slice(&0x000b_u16.to_be_bytes());
    public[16..18].copy_from_slice(&0x0003_u16.to_be_bytes());
    public[18..20].copy_from_slice(&0x0010_u16.to_be_bytes());
    public[20..22].copy_from_slice(&32_u16.to_be_bytes());
    public[22..54].copy_from_slice(point.x().expect("uncompressed x"));
    public[54..56].copy_from_slice(&32_u16.to_be_bytes());
    public[56..88].copy_from_slice(point.y().expect("uncompressed y"));

    let mut name = [0; 34];
    name[..2].copy_from_slice(&0x000b_u16.to_be_bytes());
    name[2..].copy_from_slice(&Sha256::digest(public));

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
    let encoded = signature.to_bytes();
    let mut parameters = Vec::with_capacity(72);
    parameters.extend_from_slice(&0x0018_u16.to_be_bytes());
    parameters.extend_from_slice(&0x000b_u16.to_be_bytes());
    parameters.extend_from_slice(&32_u16.to_be_bytes());
    parameters.extend_from_slice(&encoded[..32]);
    parameters.extend_from_slice(&32_u16.to_be_bytes());
    parameters.extend_from_slice(&encoded[32..]);

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
