use std::env;
use std::path::PathBuf;

fn main() {
    let cargo_manifest_directory = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace_toml_path = cargo_manifest_directory.parent().unwrap().join("Cargo.toml");

    // Tell Cargo to rerun this script if the workspace Cargo.toml changes
    println!("cargo:rerun-if-changed={}", workspace_toml_path.display());

    let cargo_toml = cargo_toml::Manifest::from_path(workspace_toml_path)
        .expect("Failed to read workspace manifest");

    println!(
        "Workspace manifest: {:#?}", cargo_toml
    );

    let workspace = cargo_toml.workspace
        .expect("Failed to get workspace from workspace manifest");

    let reqwest_dep = workspace.dependencies.get("reqwest")
        .expect("Failed to find reqwest dependency in workspace manifest");

    // Extract the reqwest version requirement (e.g. "0.12.9")
    let req_version = reqwest_dep.try_req()
        .expect("could not find reqwest version requirement in Cargo.toml");

    println!("cargo:rustc-env=REQWEST_VERSION={req_version}");
}
