use uniffi_build::generate_scaffolding;
use std::path::PathBuf;

fn main() {
    let udl_path = PathBuf::from("ech_doh_h3_sdk.udl");
    
    // Tell cargo to rerun if the UDL file changes
    println!("cargo:rerun-if-changed={}", udl_path.display());
    println!("cargo:rerun-if-changed=src/lib.rs");
    
    // Generate scaffolding
    generate_scaffolding(&udl_path)
        .expect("Failed to generate UniFFI scaffolding");
}