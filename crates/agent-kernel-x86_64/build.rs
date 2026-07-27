//! Build entry point for architecture-specific generated configuration.

#[path = "build/qemu_durable_profile.rs"]
mod qemu_durable_profile;

fn main() {
    qemu_durable_profile::generate();
}
