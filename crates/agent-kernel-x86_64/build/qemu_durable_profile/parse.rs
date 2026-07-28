//! Parser and fail-closed validation for the generated durable profile.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use super::fail;

const REQUIRED_KEYS: [&str; 21] = [
    "version",
    "root_resource",
    "storage_resource",
    "base_lba",
    "policy_generation",
    "tpm_handle",
    "tpm_command",
    "tpm_name_hex",
    "state_public_key_sec1_hex",
    "pcr_selection_hex",
    "pcr_digest_hex",
    "state_signer_package",
    "state_signer_public_key_hex",
    "state_signer_agent",
    "archive_authority",
    "storage_authority",
    "state_signer_nonce",
    "through_sequence",
    "call_data_generation",
    "state_signer_return_offsets",
    "reserved",
];

pub(super) fn parse_role(value: &str) -> u8 {
    match value {
        "writer" => 1,
        "recovery" => 2,
        _ => fail("AGENT_KERNEL_QEMU_DURABLE_ROLE must be writer or recovery"),
    }
}

pub(super) fn parse_profile(path: &Path) -> BTreeMap<String, String> {
    let source = fs::read_to_string(path)
        .unwrap_or_else(|error| fail(&format!("cannot read {}: {error}", path.display())));
    let mut fields = BTreeMap::new();
    for (index, raw) in source.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            fail(&format!(
                "{}:{}: expected key=value",
                path.display(),
                index + 1
            ));
        };
        let key = key.trim();
        let value = value.trim();
        if key.is_empty() || value.is_empty() {
            fail(&format!(
                "{}:{}: empty key or value",
                path.display(),
                index + 1
            ));
        }
        if !REQUIRED_KEYS.contains(&key) {
            fail(&format!(
                "{}:{}: unknown field {key}",
                path.display(),
                index + 1
            ));
        }
        if fields.insert(key.to_owned(), value.to_owned()).is_some() {
            fail(&format!(
                "{}:{}: duplicate field {key}",
                path.display(),
                index + 1
            ));
        }
    }
    fields
}

pub(super) struct ValidatedProfile {
    pub(super) root_resource: u64,
    pub(super) storage_resource: u64,
    pub(super) base_lba: u64,
    pub(super) policy_generation: u64,
    pub(super) tpm_handle: u32,
    pub(super) tpm_name: [u8; 34],
    pub(super) state_public_key: [u8; 33],
    pub(super) pcr_selection: [u8; 3],
    pub(super) pcr_digest: [u8; 32],
    pub(super) state_signer_package: PathBuf,
    pub(super) state_signer_public_key: [u8; 32],
    pub(super) state_signer_agent: u64,
    pub(super) archive_authority: u64,
    pub(super) storage_authority: u64,
    pub(super) state_signer_nonce: u64,
    pub(super) through_sequence: u64,
    pub(super) call_data_generation: u64,
    pub(super) state_signer_return_offsets: [u32; 6],
}

impl ValidatedProfile {
    pub(super) fn parse(profile_path: &Path, mut fields: BTreeMap<String, String>) -> Self {
        if take(&mut fields, "version") != "1" {
            fail("version must be 1");
        }
        let root_resource = decimal(&take(&mut fields, "root_resource"), "root_resource");
        let storage_resource = decimal(&take(&mut fields, "storage_resource"), "storage_resource");
        let base_lba = decimal(&take(&mut fields, "base_lba"), "base_lba");
        let policy_generation =
            decimal(&take(&mut fields, "policy_generation"), "policy_generation");
        let tpm_handle = hexadecimal_u32(&take(&mut fields, "tpm_handle"), "tpm_handle");
        if take(&mut fields, "tpm_command") != "sign-v184" {
            fail("tpm_command must be sign-v184");
        }
        let tpm_name = hex_array::<34>(&take(&mut fields, "tpm_name_hex"), "tpm_name_hex");
        let state_public_key = hex_array::<33>(
            &take(&mut fields, "state_public_key_sec1_hex"),
            "state_public_key_sec1_hex",
        );
        let pcr_selection =
            hex_array::<3>(&take(&mut fields, "pcr_selection_hex"), "pcr_selection_hex");
        let pcr_digest = hex_array::<32>(&take(&mut fields, "pcr_digest_hex"), "pcr_digest_hex");
        let package_value = take(&mut fields, "state_signer_package");
        let state_signer_package = resolve_path(profile_path, &package_value);
        let state_signer_public_key = hex_array::<32>(
            &take(&mut fields, "state_signer_public_key_hex"),
            "state_signer_public_key_hex",
        );
        let state_signer_agent = decimal(
            &take(&mut fields, "state_signer_agent"),
            "state_signer_agent",
        );
        let archive_authority =
            decimal(&take(&mut fields, "archive_authority"), "archive_authority");
        let storage_authority =
            decimal(&take(&mut fields, "storage_authority"), "storage_authority");
        let state_signer_nonce = unsigned(
            &take(&mut fields, "state_signer_nonce"),
            "state_signer_nonce",
        );
        let through_sequence = decimal(&take(&mut fields, "through_sequence"), "through_sequence");
        let call_data_generation = decimal(
            &take(&mut fields, "call_data_generation"),
            "call_data_generation",
        );
        let state_signer_return_offsets =
            return_offsets(&take(&mut fields, "state_signer_return_offsets"));
        fields.remove("reserved");
        if !fields.is_empty() {
            fail("profile contains unsupported fields");
        }

        validate_identifiers(
            root_resource,
            storage_resource,
            policy_generation,
            state_signer_agent,
            archive_authority,
            storage_authority,
            state_signer_nonce,
        );
        if through_sequence != 64 || call_data_generation != 1 {
            fail("V26 requires through_sequence=64 and call_data_generation=1");
        }
        if base_lba & 127 != 0 {
            fail("base_lba must be aligned to 128 sectors");
        }
        if !(0x8100_0000..=0x81ff_ffff).contains(&tpm_handle) {
            fail("tpm_handle must be in the persistent handle range");
        }
        if tpm_name[..2] != [0x00, 0x0b] {
            fail("tpm_name_hex must carry a SHA-256 TPM Name");
        }
        if !matches!(state_public_key[0], 0x02 | 0x03)
            || state_public_key[1..].iter().all(|byte| *byte == 0)
        {
            fail("state_public_key_sec1_hex must be a compressed P-256 key");
        }
        if pcr_selection.iter().all(|byte| *byte == 0) {
            fail("pcr_selection_hex must select at least one PCR");
        }
        if pcr_digest.iter().all(|byte| *byte == 0)
            || state_signer_public_key.iter().all(|byte| *byte == 0)
        {
            fail("PCR digest and StateSigner public key must be nonzero");
        }
        validate_package(&state_signer_package);

        Self {
            root_resource,
            storage_resource,
            base_lba,
            policy_generation,
            tpm_handle,
            tpm_name,
            state_public_key,
            pcr_selection,
            pcr_digest,
            state_signer_package,
            state_signer_public_key,
            state_signer_agent,
            archive_authority,
            storage_authority,
            state_signer_nonce,
            through_sequence,
            call_data_generation,
            state_signer_return_offsets,
        }
    }
}

fn validate_identifiers(
    root_resource: u64,
    storage_resource: u64,
    policy_generation: u64,
    state_signer_agent: u64,
    archive_authority: u64,
    storage_authority: u64,
    state_signer_nonce: u64,
) {
    if root_resource == 0
        || storage_resource == 0
        || root_resource == storage_resource
        || policy_generation == 0
        || state_signer_agent == 0
        || archive_authority == 0
        || storage_authority == 0
        || state_signer_nonce == 0
    {
        fail("resource, policy, Agent, and authority identifiers must be nonzero and distinct");
    }
}

fn validate_package(path: &Path) {
    let package = fs::read(path)
        .unwrap_or_else(|error| fail(&format!("cannot read {}: {error}", path.display())));
    if package.len() < 32 || package.get(..8) != Some(b"AGNTIMG\0") {
        fail("state_signer_package must be an Agent Image package");
    }
}

fn take(fields: &mut BTreeMap<String, String>, key: &str) -> String {
    fields
        .remove(key)
        .unwrap_or_else(|| fail(&format!("missing required field {key}")))
}

fn decimal(value: &str, field: &str) -> u64 {
    value
        .parse()
        .unwrap_or_else(|_| fail(&format!("{field} must be an unsigned decimal integer")))
}

fn unsigned(value: &str, field: &str) -> u64 {
    if let Some(hexadecimal) = value.strip_prefix("0x") {
        u64::from_str_radix(hexadecimal, 16)
            .unwrap_or_else(|_| fail(&format!("{field} must be an unsigned integer")))
    } else {
        decimal(value, field)
    }
}

fn hexadecimal_u32(value: &str, field: &str) -> u32 {
    let Some(value) = value.strip_prefix("0x") else {
        fail(&format!("{field} must use a 0x hexadecimal prefix"));
    };
    u32::from_str_radix(value, 16)
        .unwrap_or_else(|_| fail(&format!("{field} must be a u32 hexadecimal integer")))
}

fn hex_array<const N: usize>(value: &str, field: &str) -> [u8; N] {
    if value.len() != N * 2 || !value.is_ascii() {
        fail(&format!(
            "{field} must contain exactly {} hexadecimal bytes",
            N
        ));
    }
    let mut bytes = [0; N];
    for (index, byte) in bytes.iter_mut().enumerate() {
        let offset = index * 2;
        *byte = u8::from_str_radix(&value[offset..offset + 2], 16)
            .unwrap_or_else(|_| fail(&format!("{field} contains non-hexadecimal data")));
    }
    bytes
}

fn return_offsets(value: &str) -> [u32; 6] {
    let mut offsets = [0; 6];
    let mut count = 0;
    for raw in value.split(',') {
        if count == offsets.len() {
            fail("state_signer_return_offsets must contain exactly six offsets");
        }
        let offset: u32 = raw.trim().parse().unwrap_or_else(|_| {
            fail("state_signer_return_offsets must contain decimal u32 values")
        });
        if offset == 0 || offset > 65_536 || offsets[..count].contains(&offset) {
            fail("state_signer_return_offsets must be unique offsets inside Agent code");
        }
        offsets[count] = offset;
        count += 1;
    }
    if count != offsets.len() {
        fail("state_signer_return_offsets must contain exactly six offsets");
    }
    offsets
}

fn resolve_path(profile_path: &Path, value: &str) -> PathBuf {
    let path = PathBuf::from(value);
    let resolved = if path.is_absolute() {
        path
    } else {
        profile_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(path)
    };
    resolved
        .canonicalize()
        .unwrap_or_else(|error| fail(&format!("cannot resolve {}: {error}", resolved.display())))
}
