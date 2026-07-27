//! Immutable SHA-256 PCR policy used by the provisioned TPM signer.
//!
//! The policy digest is computed in kernel memory from the exact TPM-marshalled
//! PCR selection and the configured signing command.

use sha2::{Digest, Sha256};

use super::DigestSignCommand;

const TPM_CC_POLICY_PCR: u32 = 0x0000_017f;
const TPM_CC_POLICY_COMMAND_CODE: u32 = 0x0000_016c;
const TPM_ALG_SHA256: u16 = 0x000b;
const MARSHALLED_SELECTION_BYTES: usize = 10;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TpmPcrPolicyError {
    EmptySelection,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Sha256PcrPolicy {
    selection: [u8; 3],
    expected_digest: [u8; 32],
}

impl Sha256PcrPolicy {
    pub const fn new(
        selection: [u8; 3],
        expected_digest: [u8; 32],
    ) -> Result<Self, TpmPcrPolicyError> {
        if selection[0] == 0 && selection[1] == 0 && selection[2] == 0 {
            return Err(TpmPcrPolicyError::EmptySelection);
        }
        Ok(Self {
            selection,
            expected_digest,
        })
    }

    pub const fn selection(self) -> [u8; 3] {
        self.selection
    }

    pub const fn expected_digest(self) -> [u8; 32] {
        self.expected_digest
    }

    pub fn authorization_digest(self, command: DigestSignCommand) -> [u8; 32] {
        let mut pcr_hasher = Sha256::new();
        pcr_hasher.update([0; 32]);
        pcr_hasher.update(TPM_CC_POLICY_PCR.to_be_bytes());
        pcr_hasher.update(self.marshalled_selection());
        pcr_hasher.update(self.expected_digest);
        let pcr_policy: [u8; 32] = pcr_hasher.finalize().into();

        let mut command_hasher = Sha256::new();
        command_hasher.update(pcr_policy);
        command_hasher.update(TPM_CC_POLICY_COMMAND_CODE.to_be_bytes());
        command_hasher.update(command.command_code().to_be_bytes());
        command_hasher.finalize().into()
    }

    pub(crate) fn marshalled_selection(self) -> [u8; MARSHALLED_SELECTION_BYTES] {
        let mut encoded = [0; MARSHALLED_SELECTION_BYTES];
        encoded[..4].copy_from_slice(&1_u32.to_be_bytes());
        encoded[4..6].copy_from_slice(&TPM_ALG_SHA256.to_be_bytes());
        encoded[6] = self.selection.len() as u8;
        encoded[7..].copy_from_slice(&self.selection);
        encoded
    }
}
