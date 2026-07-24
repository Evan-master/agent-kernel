//! TPM 2.0 discovery, transport, wire, and provisioned signing support.
//!
//! This architecture layer owns bounded x86 TPM interaction. It exposes no
//! arbitrary command path to Agents and keeps TPM policy out of kernel core.

mod acpi;
mod crb;
mod mmio;
mod public;
mod service;
mod signer;
mod wire;

pub use acpi::{parse_tpm2_acpi_table, Tpm2AcpiTable, Tpm2AcpiTableError, Tpm2StartMethod};
pub use crb::{CrbIo, CrbTransport, CrbTransportError};
pub use mmio::{CrbMmioError, VolatileCrbIo};
pub use public::TpmPublicError;
pub use service::{
    sign_retained_durable_request, KernelStateSigner, KernelStateSignerError,
    KernelStateSignerServiceError,
};
pub use signer::{
    ProvisionedTpmSigner, ProvisionedTpmSignerConfig, TpmSignerConfigError, TpmSignerError,
};
pub use wire::{
    encode_read_public, encode_sign_p256_digest, parse_p256_signature_response,
    parse_read_public_response, DigestSignCommand, ReadPublicResponse, TpmPersistentHandle,
    TpmWireError, MAX_TPM_NAME_BYTES, MAX_TPM_PUBLIC_BYTES, READ_PUBLIC_COMMAND_BYTES,
    SIGN_COMMAND_BYTES,
};
