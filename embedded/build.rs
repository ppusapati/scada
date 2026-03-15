use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    // Only include linker script for STM32H743 target
    let target = env::var("TARGET").unwrap_or_default();
    if target.contains("thumbv7em") {
        let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
        let memory_x = fs::read("memory.x").expect("memory.x not found");
        fs::write(out_dir.join("memory.x"), memory_x).unwrap();
        println!("cargo:rustc-link-search={}", out_dir.display());
        println!("cargo:rerun-if-changed=memory.x");
    }
    println!("cargo:rerun-if-changed=build.rs");
}
