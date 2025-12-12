fn main() {
    // Ensure we rebuild when build.rs changes
    println!("cargo:rerun-if-changed=build.rs");

    // UniFFI scaffolding is generated via proc-macros in uniffi_bindings.rs
    // using uniffi::setup_scaffolding!() macro - no UDL file needed
    #[cfg(feature = "uniffi")]
    println!("cargo:rerun-if-changed=src/uniffi_bindings.rs");
}
