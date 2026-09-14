use uniffi_build::{ScaffoldingOptions, generate_scaffolding};
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let udl_path = PathBuf::from("ech_doh_h3_sdk.udl");
    
    // Tell cargo to rerun if the UDL file changes
    println!("cargo:rerun-if-changed={}", udl_path.display());
    println!("cargo:rerun-if-changed=src/lib.rs");
    
    // Generate scaffolding
    generate_scaffolding(&udl_path, &out_dir, ScaffoldingOptions::default())
        .expect("Failed to generate UniFFI scaffolding");
}