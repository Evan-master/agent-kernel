//! Build-time selection for the public QEMU durable proof profile.
//!
//! The generated module contains public device and signer material only. TPM
//! private state and the ephemeral Agent-image signing key stay in the host
//! proof workspace.

#[path = "qemu_durable_profile/parse.rs"]
mod parse;
#[path = "qemu_durable_profile/render.rs"]
mod render;

use std::{
    env,
    path::{Path, PathBuf},
};

use parse::{parse_profile, parse_role, ValidatedProfile};

const FEATURE_ENV: &str = "CARGO_FEATURE_QEMU_DURABLE_PROOF";
const ROLE_ENV: &str = "AGENT_KERNEL_QEMU_DURABLE_ROLE";
const PROFILE_ENV: &str = "AGENT_KERNEL_QEMU_DURABLE_PROFILE";
const OUTPUT_FILE: &str = "qemu_durable_profile.rs";

pub(crate) fn generate() {
    println!("cargo:rerun-if-env-changed={FEATURE_ENV}");
    println!("cargo:rerun-if-env-changed={ROLE_ENV}");
    println!("cargo:rerun-if-env-changed={PROFILE_ENV}");

    let output = output_path();
    if env::var_os(FEATURE_ENV).is_none() {
        render::write_disabled(&output);
        return;
    }

    let role = env::var(ROLE_ENV).ok();
    let profile = env::var(PROFILE_ENV).ok();
    match (role, profile) {
        (None, None) => render::write_disabled(&output),
        (Some(_), None) | (None, Some(_)) => {
            fail("durable role and profile must be configured together")
        }
        (Some(role), Some(profile)) => generate_enabled(&output, &role, &profile),
    }
}

fn generate_enabled(output: &Path, role: &str, profile: &str) {
    let role = parse_role(role);
    let profile_path = PathBuf::from(profile);
    println!("cargo:rerun-if-changed={}", profile_path.display());
    let fields = parse_profile(&profile_path);
    let validated = ValidatedProfile::parse(&profile_path, fields);
    println!(
        "cargo:rerun-if-changed={}",
        validated.state_signer_package.display()
    );
    render::write_enabled(output, role, &validated);
}

fn output_path() -> PathBuf {
    let output = env::var_os("OUT_DIR").unwrap_or_else(|| fail("OUT_DIR is missing"));
    PathBuf::from(output).join(OUTPUT_FILE)
}

fn fail(message: &str) -> ! {
    panic!("QEMU durable profile error: {message}")
}
