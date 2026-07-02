//! Host-side image construction helpers for Agent Kernel.
//!
//! This crate owns BIOS image creation and QEMU argument construction. It is a
//! host tool and must not be imported by no_std kernel crates.

use std::{
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildPaths {
    pub kernel: PathBuf,
    pub image: PathBuf,
}

impl BuildPaths {
    pub fn new(kernel: impl Into<PathBuf>, image: impl Into<PathBuf>) -> Self {
        Self {
            kernel: kernel.into(),
            image: image.into(),
        }
    }
}

pub fn create_bios_image(paths: &BuildPaths) -> Result<(), Box<dyn std::error::Error>> {
    bootloader::DiskImageBuilder::new(paths.kernel.clone()).create_bios_image(&paths.image)?;
    Ok(())
}

pub fn qemu_bios_args(image: impl AsRef<Path>) -> Vec<String> {
    let drive = format!("format=raw,file={}", image.as_ref().display());

    vec![
        "-drive".to_string(),
        drive,
        "-serial".to_string(),
        "stdio".to_string(),
        "-display".to_string(),
        "none".to_string(),
        "-no-reboot".to_string(),
        "-no-shutdown".to_string(),
        "-device".to_string(),
        "isa-debug-exit,iobase=0xf4,iosize=0x04".to_string(),
    ]
}

pub fn qemu_bios_command(image: impl AsRef<Path>) -> Command {
    let mut command = Command::new("qemu-system-x86_64");
    command.args(qemu_bios_args(image));
    command
}
