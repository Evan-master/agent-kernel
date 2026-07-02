use std::{env, process};

use agent_kernel_image::{create_bios_image, BuildPaths};

fn main() {
    let mut args = env::args().skip(1);
    let Some(kernel) = args.next() else {
        eprintln!("usage: agent-kernel-image <kernel-elf> <output-bios-img>");
        process::exit(2);
    };
    let Some(image) = args.next() else {
        eprintln!("usage: agent-kernel-image <kernel-elf> <output-bios-img>");
        process::exit(2);
    };

    let paths = BuildPaths::new(kernel, image);
    if let Err(error) = create_bios_image(&paths) {
        eprintln!("failed to create BIOS image: {error}");
        process::exit(1);
    }

    println!("{}", paths.image.display());
}
