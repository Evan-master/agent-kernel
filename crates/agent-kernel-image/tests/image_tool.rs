use std::path::PathBuf;

use agent_kernel_image::{qemu_bios_args, BuildPaths};

#[test]
fn build_paths_preserve_kernel_and_image_paths() {
    let paths = BuildPaths::new("kernel.elf", "agent.img");

    assert_eq!(paths.kernel, PathBuf::from("kernel.elf"));
    assert_eq!(paths.image, PathBuf::from("agent.img"));
}

#[test]
fn qemu_bios_args_include_serial_and_debug_exit() {
    let args = qemu_bios_args("agent.img");

    assert!(args.contains(&"-drive".to_string()));
    assert!(args.contains(&"format=raw,file=agent.img".to_string()));
    assert!(args.contains(&"-serial".to_string()));
    assert!(args.contains(&"stdio".to_string()));
    assert!(args.iter().any(|arg| arg.contains("isa-debug-exit")));
}
