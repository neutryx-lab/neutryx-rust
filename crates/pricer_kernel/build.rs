//! Build script for pricer_kernel.
//!
//! Validates LLVM 18 availability and configures Enzyme plugin support.
//!
//! # Environment Variables
//!
//! - `LLVM_CONFIG`: Path to llvm-config binary (optional, auto-detected)
//! - `ENZYME_LIB`: Path to LLVMEnzyme-18.so plugin (required for enzyme-ad feature)
//!
//! # Phase 3.0
//!
//! This build script provides validation and guidance but does not block builds
//! when LLVM/Enzyme are not available. Phase 4 will require full Enzyme integration.

use std::env;
use std::process::Command;

fn main() {
    // Emit rerun directives for environment variable changes
    println!("cargo:rerun-if-env-changed=LLVM_CONFIG");
    println!("cargo:rerun-if-env-changed=ENZYME_LIB");
    println!("cargo:rerun-if-env-changed=LLVM_SYS_180_PREFIX");

    // Validate LLVM version
    validate_llvm_version();

    // Configure Enzyme plugin if enzyme-ad feature is enabled
    #[cfg(feature = "enzyme-ad")]
    configure_enzyme_plugin();
}

/// Validates that LLVM 18 is available in the build environment.
///
/// Attempts to detect LLVM version via:
/// 1. LLVM_CONFIG environment variable
/// 2. llvm-config-18 command
/// 3. llvm-config command
///
/// Emits cargo:warning with installation guidance if LLVM 18 is not found.
fn validate_llvm_version() {
    let llvm_config = env::var("LLVM_CONFIG")
        .ok()
        .or_else(|| find_llvm_config());

    match llvm_config {
        Some(config_path) => {
            if let Ok(output) = Command::new(&config_path).arg("--version").output() {
                let version = String::from_utf8_lossy(&output.stdout);
                let version = version.trim();

                if version.starts_with("18.") {
                    println!("cargo:warning=LLVM 18 detected: {}", version);
                } else {
                    emit_llvm_version_warning(&format!(
                        "Found LLVM {} but LLVM 18 is required for Enzyme support",
                        version
                    ));
                }
            } else {
                emit_llvm_version_warning("Failed to execute llvm-config");
            }
        }
        None => {
            emit_llvm_version_warning("LLVM 18 not found in PATH");
        }
    }
}

/// Attempts to find llvm-config in common locations.
fn find_llvm_config() -> Option<String> {
    // Try version-specific command first
    for cmd in &["llvm-config-18", "llvm-config"] {
        if Command::new(cmd).arg("--version").output().is_ok() {
            return Some(cmd.to_string());
        }
    }

    // Check common installation paths on Unix-like systems
    #[cfg(unix)]
    {
        let common_paths = [
            "/usr/lib/llvm-18/bin/llvm-config",
            "/usr/local/opt/llvm@18/bin/llvm-config",
            "/opt/homebrew/opt/llvm@18/bin/llvm-config",
        ];

        for path in common_paths {
            if std::path::Path::new(path).exists() {
                return Some(path.to_string());
            }
        }
    }

    None
}

/// Emits a cargo warning with LLVM installation guidance.
fn emit_llvm_version_warning(message: &str) {
    println!("cargo:warning={}", message);
    println!("cargo:warning=");
    println!("cargo:warning=LLVM 18 Installation Guide:");
    println!("cargo:warning=  Ubuntu/Debian: wget https://apt.llvm.org/llvm.sh && chmod +x llvm.sh && sudo ./llvm.sh 18");
    println!("cargo:warning=  macOS: brew install llvm@18");
    println!("cargo:warning=  Windows: Download from https://github.com/llvm/llvm-project/releases");
    println!("cargo:warning=");
    println!("cargo:warning=Set LLVM_CONFIG environment variable if LLVM is installed in a non-standard location.");
    println!("cargo:warning=");
    println!("cargo:warning=Phase 3.0 uses placeholder implementation; actual Enzyme AD requires LLVM 18.");
}

/// Configures Enzyme plugin loading when enzyme-ad feature is enabled.
#[cfg(feature = "enzyme-ad")]
fn configure_enzyme_plugin() {
    match env::var("ENZYME_LIB") {
        Ok(enzyme_path) => {
            if std::path::Path::new(&enzyme_path).exists() {
                println!("cargo:warning=Enzyme plugin found at: {}", enzyme_path);
                // Note: RUSTFLAGS must be set externally; build.rs cannot modify them
                println!("cargo:warning=Ensure RUSTFLAGS includes: -C llvm-args=-load={}", enzyme_path);
            } else {
                emit_enzyme_warning(&format!("ENZYME_LIB path does not exist: {}", enzyme_path));
            }
        }
        Err(_) => {
            emit_enzyme_warning("ENZYME_LIB environment variable not set");
        }
    }
}

/// Emits a cargo warning with Enzyme installation guidance.
#[cfg(feature = "enzyme-ad")]
fn emit_enzyme_warning(message: &str) {
    println!("cargo:warning={}", message);
    println!("cargo:warning=");
    println!("cargo:warning=Enzyme Plugin Installation:");
    println!("cargo:warning=  1. Clone: git clone https://github.com/EnzymeAD/Enzyme");
    println!("cargo:warning=  2. Build: mkdir build && cd build && cmake .. -DLLVM_DIR=/path/to/llvm-18 && make");
    println!("cargo:warning=  3. Set: export ENZYME_LIB=/path/to/LLVMEnzyme-18.so");
    println!("cargo:warning=  4. Build: export RUSTFLAGS=\"-C llvm-args=-load=$ENZYME_LIB\" && cargo build");
    println!("cargo:warning=");
    println!("cargo:warning=See https://enzyme.mit.edu/ for detailed installation instructions.");
}
