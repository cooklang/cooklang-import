fn main() {
    // UniFFI scaffolding is generated via proc-macros in uniffi_bindings.rs
    // using uniffi::setup_scaffolding!() macro - no UDL file needed

    // Ensure we rebuild when these files change
    println!("cargo:rerun-if-changed=src/uniffi_bindings.rs");
    println!("cargo:rerun-if-changed=build.rs");
}
