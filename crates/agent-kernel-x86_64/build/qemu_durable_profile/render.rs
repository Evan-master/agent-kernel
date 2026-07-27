//! Rust-source rendering for disabled, writer, and recovery profiles.

use std::{fs, path::Path};

use super::{fail, ValidatedProfile};

pub(super) fn write_disabled(output: &Path) {
    let source = format!(
        "{}\n{}\n{}\n{}\n{}\n",
        scalar_constants(0, 0, 0, 0, 0, 0, 0, 0, 0),
        array_constants(&[0; 34], &[0; 33], &[0; 3], &[0; 32], &[0; 32]),
        state_signer_constants(0, 0, 0),
        return_offset_constant(&[0; 6]),
        "pub(crate) static QEMU_STATE_SIGNER_PACKAGE: &[u8] = &[];"
    );
    write(output, source);
}

pub(super) fn write_enabled(output: &Path, role: u8, profile: &ValidatedProfile) {
    let package = format!("{:?}", profile.state_signer_package);
    let source = format!(
        "{}\n{}\n{}\n{}\npub(crate) static QEMU_STATE_SIGNER_PACKAGE: &[u8] = include_bytes!({package});\n",
        scalar_constants(
            role,
            profile.root_resource,
            profile.storage_resource,
            profile.base_lba,
            profile.policy_generation,
            profile.tpm_handle,
            profile.state_signer_agent,
            profile.archive_authority,
            profile.storage_authority,
        ),
        array_constants(
            &profile.tpm_name,
            &profile.state_public_key,
            &profile.pcr_selection,
            &profile.pcr_digest,
            &profile.state_signer_public_key,
        ),
        state_signer_constants(
            profile.state_signer_nonce,
            profile.through_sequence,
            profile.call_data_generation,
        ),
        return_offset_constant(&profile.state_signer_return_offsets),
    );
    write(output, source);
}

fn state_signer_constants(nonce: u64, through: u64, generation: u64) -> String {
    format!(
        "pub(crate) const QEMU_STATE_SIGNER_NONCE: u64 = {nonce:#018x};\n\
         pub(crate) const QEMU_DURABLE_THROUGH_SEQUENCE: u64 = {through};\n\
         pub(crate) const QEMU_DURABLE_CALL_DATA_GENERATION: u64 = {generation};"
    )
}

fn return_offset_constant(offsets: &[u32; 6]) -> String {
    format!(
        "pub(crate) const QEMU_STATE_SIGNER_RETURN_OFFSETS: [u32; 6] = {:?};",
        offsets
    )
}

#[allow(clippy::too_many_arguments)]
fn scalar_constants(
    role: u8,
    root: u64,
    storage: u64,
    base_lba: u64,
    generation: u64,
    handle: u32,
    agent: u64,
    archive_authority: u64,
    storage_authority: u64,
) -> String {
    format!(
        "pub(crate) const QEMU_DURABLE_ROLE: u8 = {role};\n\
         pub(crate) const QEMU_DURABLE_ROOT_RESOURCE: u64 = {root};\n\
         pub(crate) const QEMU_DURABLE_STORAGE_RESOURCE: u64 = {storage};\n\
         pub(crate) const QEMU_DURABLE_BASE_LBA: u64 = {base_lba};\n\
         pub(crate) const QEMU_DURABLE_POLICY_GENERATION: u64 = {generation};\n\
         pub(crate) const QEMU_DURABLE_TPM_HANDLE: u32 = {handle:#010x};\n\
         pub(crate) const QEMU_STATE_SIGNER_AGENT: u64 = {agent};\n\
         pub(crate) const QEMU_ARCHIVE_AUTHORITY: u64 = {archive_authority};\n\
         pub(crate) const QEMU_STORAGE_AUTHORITY: u64 = {storage_authority};"
    )
}

fn array_constants(
    name: &[u8; 34],
    public_key: &[u8; 33],
    selection: &[u8; 3],
    digest: &[u8; 32],
    image_public_key: &[u8; 32],
) -> String {
    format!(
        "pub(crate) const QEMU_DURABLE_TPM_NAME: [u8; 34] = {};\n\
         pub(crate) const QEMU_DURABLE_STATE_PUBLIC_KEY: [u8; 33] = {};\n\
         pub(crate) const QEMU_DURABLE_PCR_SELECTION: [u8; 3] = {};\n\
         pub(crate) const QEMU_DURABLE_PCR_DIGEST: [u8; 32] = {};\n\
         pub(crate) const QEMU_STATE_SIGNER_PUBLIC_KEY: [u8; 32] = {};",
        rust_array(name),
        rust_array(public_key),
        rust_array(selection),
        rust_array(digest),
        rust_array(image_public_key),
    )
}

fn rust_array(bytes: &[u8]) -> String {
    let values: Vec<String> = bytes.iter().map(|byte| format!("{byte:#04x}")).collect();
    format!("[{}]", values.join(", "))
}

fn write(output: &Path, source: String) {
    fs::write(output, source)
        .unwrap_or_else(|error| fail(&format!("cannot write {}: {error}", output.display())));
}
