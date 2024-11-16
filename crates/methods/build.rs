use std::env;

fn main() {
    // Determine whether to skip building the guest code
    let skip_build =
        env::var("RISC0_SKIP_BUILD").is_ok() || env::var("CARGO_PRIMARY_PACKAGE").is_err();

    if skip_build {
        println!("cargo:warning=Skipping guest code build");
        // Set custom cfg flag to indicate that the guest code is not built
        println!("cargo:rustc-cfg=guest_code_not_built");
        // Inform the compiler about the custom cfg name to suppress warnings
        println!("cargo:rustc-check-cfg=cfg(guest_code_not_built)");
    } else {
        // Build the guest code as usual
        risc0_build::embed_methods();
    }
}
