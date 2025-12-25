use std::process::Command;

fn main() {
    // Build the frontend
    let status = Command::new("pnpm")
        .args(&["build"])
        .current_dir("../frontend")
        .status()
        .expect("Failed to build frontend");

    if !status.success() {
        panic!("Frontend build failed");
    }

    tauri_build::build();

    // Set the config path for Tauri
    unsafe {
        std::env::set_var(
            "TAURI_CONFIG",
            "/home/orkarfabianthewise/code/kekahyde/src-tauri/tauri.conf.json",
        );
    }
}
