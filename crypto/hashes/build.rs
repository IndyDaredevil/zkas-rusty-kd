use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=src/asm");

    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();

    // The crate already offers a `no-asm` feature that selects a pure-Rust keccak,
    // but this script never read it: it compiled the hand-written assembly for any
    // x86_64 target and then panicked `unimplemented!("Unsupported OS")` for any OS
    // outside macos/linux/windows. That made the crate unbuildable for
    // x86_64-linux-android and x86_64-apple-ios — the Intel Android emulator and
    // the Intel Mac simulator — with no way for a consumer to opt out. Honouring
    // the feature is the whole fix; a build that does not request it is unchanged.
    let no_asm = env::var("CARGO_FEATURE_NO_ASM").is_ok();
    if target_arch == "x86_64" && !no_asm {
        let mut builder = cc::Build::new();
        builder.flag("-c");
        match target_os.as_str() {
            "macos" => builder.file("src/asm/keccakf1600_x86-64-osx.s"),
            "linux" => builder.file("src/asm/keccakf1600_x86-64-elf.s"),
            "windows" if target_env == "gnu" => builder.file("src/asm/keccakf1600_x86-64-mingw64.s"),
            "windows" if target_env == "msvc" => builder.file("src/asm/keccakf1600_x86-64-msvc.asm"),
            _ => unimplemented!("Unsupported OS"),
        };
        builder.compile("libkeccak.a");
    }
    Ok(())
}
