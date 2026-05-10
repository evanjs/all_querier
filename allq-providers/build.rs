fn main() {
    let lock = std::fs::read_to_string("../Cargo.lock")
        .expect("failed to read Cargo.lock");

    let manifest = std::fs::read_to_string("Cargo.toml")
        .expect("failed to read Cargo.toml");

    // Extract the reqwest version requirement from Cargo.toml (e.g. "0.12.9")
    let req_version_req = manifest
        .lines()
        .find_map(|line| {
            let line = line.trim();
            if !line.starts_with("reqwest") {
                return None;
            }
            // e.g. `reqwest = { version = "0.12.9", ... }` or `reqwest = "0.12.9"`
            let after = line.splitn(2, '=').nth(1)?.trim();
            // Find the version string value
            let start = after.find('"')? + 1;
            let rest = &after[start..];
            let end = rest.find('"')?;
            Some(rest[..end].to_owned())
        })
        .expect("could not find reqwest version in Cargo.toml");

    // Determine the major.minor prefix to match against Cargo.lock
    let prefix = req_version_req
        .splitn(3, '.')
        .take(2)
        .collect::<Vec<_>>()
        .join(".");

    let reqwest_version = lock
        .split("\n[[package]]")
        .find_map(|section| {
            let name_line = section.lines().find(|l| l.starts_with("name ="))?;
            let name = name_line
                .split_once('=')
                .map(|(_, v)| v.trim().trim_matches('"'))?;
            if name != "reqwest" {
                return None;
            }
            let version_line = section.lines().find(|l| l.starts_with("version ="))?;
            let version = version_line
                .split_once('=')
                .map(|(_, v)| v.trim().trim_matches('"'))?;
            if version.starts_with(&prefix) {
                Some(version.to_owned())
            } else {
                None
            }
        })
        .expect("could not find matching reqwest version in Cargo.lock");

    println!("cargo:rustc-env=REQWEST_VERSION={reqwest_version}");
    println!("cargo:rerun-if-changed=../Cargo.lock");
    println!("cargo:rerun-if-changed=Cargo.toml");
}
