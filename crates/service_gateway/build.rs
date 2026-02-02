//! Build script for `service_gateway`.

use std::path::Path;
use std::process::Command;

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
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_root = Path::new(&manifest_dir).parent().unwrap().parent().unwrap();
    let gui_static_dir = workspace_root.join("demo/gui/static");
    let gui_dist_dir = workspace_root.join("demo/gui/dist");
    let node_modules = gui_static_dir.join("node_modules");

    // Skip if static directory doesn't exist
    if !gui_static_dir.exists() {
        println!("cargo:warning=Demo GUI source not found at {:?}", gui_static_dir);
        return;
    }

    // Rerun if source files change
    println!("cargo:rerun-if-changed=../../demo/gui/static/src");
    println!("cargo:rerun-if-changed=../../demo/gui/static/index.html");
    println!("cargo:rerun-if-changed=../../demo/gui/static/package.json");

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

    // Build if dist doesn't exist
    if !gui_dist_dir.exists() || !gui_dist_dir.join("index.html").exists() {
        println!("cargo:warning=Building demo GUI...");
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
}
