use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let target_dir =
        PathBuf::from(std::env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| "target".to_string()));
    let build_mode = std::env::var("PROFILE").unwrap(); // "debug" or "release"
    let output_dir = target_dir.join(build_mode);

    println!("cargo:rerun-if-changed=src/ap_trampoline.asm");

    // Make sure the target directory exists
    std::fs::create_dir_all(&output_dir).expect("Failed to create target directory");

    //  Assemble the trampoline as an ELF64 object.
    let asm_src = "kernel/src/smp/ap_trampoline.asm";
    let asm_obj = output_dir.join("ap_trampoline.o");
    let status = Command::new("nasm")
        .args(&["-f", "elf64", asm_src, "-o"])
        .arg(&asm_obj)
        .status()
        .expect("failed to run nasm");
    assert!(status.success(), "nasm failed");

    // Convert the ELF object to a flat binary using objcopy.
    let trampoline_bin = output_dir.join("ap_trampoline.bin");
    let status = Command::new("llvm-objcopy") // or "objcopy" if available
        .args(&[
            "-O",
            "binary",
            asm_obj.to_str().unwrap(),
            trampoline_bin.to_str().unwrap(),
        ])
        .status()
        .expect("failed to run objcopy");
    assert!(status.success(), "objcopy failed");

    // Expose the binary path as an environment variable
    println!(
        "cargo:rustc-env=AP_TRAMPOLINE_BIN={}",
        trampoline_bin.display()
    );

    // Locate kernel binary
    let kernel_bin_name = format!("CARGO_BIN_FILE_{}_{}", "RUST_OS", "rust-os");
    let kernel = PathBuf::from(std::env::var_os(kernel_bin_name).expect("Kernel binary not found"));

    // UEFI and BIOS disk iamges
    let uefi_path = output_dir.join("uefi.img");
    bootloader::UefiBoot::new(&kernel)
        .create_disk_image(&uefi_path)
        .expect("Failed to create UEFI disk image");

    let bios_path = output_dir.join("bios.img");
    bootloader::BiosBoot::new(&kernel)
        .create_disk_image(&bios_path)
        .expect("Failed to create BIOS disk image");

    let kernel_trampoline = PathBuf::from("kernel/src/smp/ap_trampoline.bin");
    fs::copy(&trampoline_bin, &kernel_trampoline)
        .expect("Failed to copy ap_trampoline.bin into the kernel crate");

    // paths for linking and runtime
    println!("cargo:rustc-link-search={}", output_dir.display());
    println!("cargo:rustc-env=UEFI_PATH={}", uefi_path.display());
    println!("cargo:rustc-env=BIOS_PATH={}", bios_path.display());
}
