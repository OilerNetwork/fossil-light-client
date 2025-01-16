fn main() {
    // Get Python library path
    let output = std::process::Command::new("python3-config")
        .arg("--prefix")
        .output()
        .expect("Failed to execute python3-config");
    let python_prefix = String::from_utf8(output.stdout)
        .expect("Invalid UTF-8")
        .trim()
        .to_string();

    // Get Python version
    let python_version = std::process::Command::new("python3")
        .args([
            "-c",
            "import sys; print(f'{sys.version_info.major}.{sys.version_info.minor}')",
        ])
        .output()
        .expect("Failed to get Python version")
        .stdout;
    let python_version = String::from_utf8(python_version)
        .expect("Invalid UTF-8")
        .trim()
        .to_string();

    // Add Python library directory to search path
    println!("cargo:rustc-link-search={}/lib", python_prefix);
    println!("cargo:rustc-link-search=/opt/homebrew/lib");

    // Link against specific Python version
    println!("cargo:rustc-link-lib=python{}", python_version);

    // Additional required libraries from python3-config --ldflags
    println!("cargo:rustc-link-lib=intl");
    println!("cargo:rustc-link-lib=dl");

    // On macOS, we need these frameworks
    println!("cargo:rustc-link-lib=framework=CoreFoundation");
    println!("cargo:rustc-link-lib=framework=System");

    // Add rpath
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}/lib", python_prefix);
    println!("cargo:rustc-link-arg=-Wl,-rpath,/opt/homebrew/lib");

    // Tell cargo to invalidate the built crate whenever the wrapper changes
    println!("cargo:rerun-if-changed=wrapper.h");
}
