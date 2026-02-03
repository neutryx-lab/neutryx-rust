//! Build script for `service_gateway`.

use std::{path::Path, process::Command};

/// Entry point for build script.
fn main() {
    // gRPC proto compilation will be added when grpc feature is implemented
    // tonic_build::compile_protos("proto/pricing.proto").unwrap();

    // Build demo GUI when demo feature is enabled
    #[cfg(feature = "demo")]
    build_demo_gui();
}

#[cfg(feature = "demo")]
fn build_demo_gui() {
    use std::fs;
    use std::time::SystemTime;

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_root = Path::new(&manifest_dir).parent().unwrap().parent().unwrap();
    let gui_static_dir = workspace_root.join("demo/gui/static");
    let gui_dist_dir = workspace_root.join("demo/gui/dist");
    let node_modules = gui_static_dir.join("node_modules");
    let src_dir = gui_static_dir.join("src");

    // Skip if static directory doesn't exist
    if !gui_static_dir.exists() {
        println!(
            "cargo:warning=Demo GUI source not found at {:?}",
            gui_static_dir
        );
        return;
    }

    // Rerun if source files change or dist is missing
    println!("cargo:rerun-if-changed=../../demo/gui/static/src");
    println!("cargo:rerun-if-changed=../../demo/gui/static/index.html");
    println!("cargo:rerun-if-changed=../../demo/gui/static/package.json");
    println!("cargo:rerun-if-changed=../../demo/gui/static/vite.config.ts");
    println!("cargo:rerun-if-changed=../../demo/gui/static/tailwind.config.js");

    // Determine npm command (npm on Unix, npm.cmd on Windows)
    let npm = if cfg!(windows) { "npm.cmd" } else { "npm" };

    // Install dependencies if node_modules doesn't exist
    if !node_modules.exists() {
        println!("cargo:warning=Installing demo GUI dependencies...");
        let status = Command::new(npm)
            .arg("install")
            .current_dir(&gui_static_dir)
            .status();

        match status {
            Ok(s) if s.success() => {}
            Ok(s) => {
                println!("cargo:warning=npm install failed with status: {}", s);
                return;
            }
            Err(e) => {
                println!("cargo:warning=Failed to run npm install: {}", e);
                println!("cargo:warning=Make sure Node.js is installed");
                return;
            }
        }
    }

    // Check if rebuild is needed
    let needs_rebuild = needs_gui_rebuild(&gui_dist_dir, &src_dir);

    if needs_rebuild {
        println!("cargo:warning=Building demo GUI (source files changed)...");
        let status = Command::new(npm)
            .arg("run")
            .arg("build")
            .current_dir(&gui_static_dir)
            .status();

        match status {
            Ok(s) if s.success() => {
                println!("cargo:warning=Demo GUI built successfully");
            }
            Ok(s) => {
                println!("cargo:warning=npm run build failed with status: {}", s);
            }
            Err(e) => {
                println!("cargo:warning=Failed to run npm build: {}", e);
            }
        }
    }

    /// Check if GUI rebuild is needed by comparing source and dist modification times.
    fn needs_gui_rebuild(dist_dir: &Path, src_dir: &Path) -> bool {
        // Rebuild if dist doesn't exist
        let dist_index = dist_dir.join("index.html");
        if !dist_dir.exists() || !dist_index.exists() {
            return true;
        }

        // Get dist modification time
        let dist_mtime = match fs::metadata(&dist_index).and_then(|m| m.modified()) {
            Ok(t) => t,
            Err(_) => return true,
        };

        // Check if any source file is newer than dist
        if let Ok(entries) = fs::read_dir(src_dir) {
            for entry in entries.flatten() {
                if is_newer_than(&entry.path(), dist_mtime) {
                    return true;
                }
            }
        }

        false
    }

    /// Recursively check if path (file or directory) has any file newer than reference time.
    fn is_newer_than(path: &Path, reference: SystemTime) -> bool {
        if path.is_file() {
            if let Ok(meta) = fs::metadata(path) {
                if let Ok(mtime) = meta.modified() {
                    return mtime > reference;
                }
            }
        } else if path.is_dir() {
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.flatten() {
                    if is_newer_than(&entry.path(), reference) {
                        return true;
                    }
                }
            }
        }
        false
    }
}
