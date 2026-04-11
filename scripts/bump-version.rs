use std::env;
use std::fs;
use std::process::Command;

fn main() {
    let version = env::args()
        .skip(1)
        .find(|a| a != "--")
        .unwrap_or_else(|| {
            eprintln!("Error: VERSION argument required.");
            eprintln!("Usage: just bump-version x.y.z");
            std::process::exit(1);
        });

    let current = read_version("Cargo.toml", "version = \"");

    println!("Bumping version from {current} to {version}...");

    update_file("Cargo.toml", "version = \"", &version);
    update_file("gui/package.json", "\"version\": \"", &version);

    let cargo = if cfg!(windows) { "cargo.exe" } else { "cargo" };
    let status = Command::new(cargo)
        .args(["generate-lockfile"])
        .status()
        .expect("Failed to run cargo");
    if !status.success() {
        std::process::exit(1);
    }

    let npm = if cfg!(windows) { "npm.cmd" } else { "npm" };
    let status = Command::new(npm)
        .args(["install", "--package-lock-only"])
        .current_dir("gui")
        .status()
        .expect("Failed to run npm");
    if !status.success() {
        std::process::exit(1);
    }

    println!("Version updated to {version} in:");
    println!("  - Cargo.toml (workspace)");
    println!("  - Cargo.lock");
    println!("  - gui/package.json");
    println!("  - gui/package-lock.json");
}

fn read_version(path: &str, prefix: &str) -> String {
    let content = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("Failed to read {path}: {e}"));
    let start = content
        .find(prefix)
        .unwrap_or_else(|| panic!("Pattern '{prefix}' not found in {path}"));
    let after = start + prefix.len();
    let end = content[after..]
        .find('"')
        .unwrap_or_else(|| panic!("Closing quote not found after '{prefix}' in {path}"));
    content[after..after + end].to_string()
}

fn update_file(path: &str, prefix: &str, version: &str) {
    let content = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("Failed to read {path}: {e}"));
    let start = content
        .find(prefix)
        .unwrap_or_else(|| panic!("Pattern '{prefix}' not found in {path}"));
    let after = start + prefix.len();
    let end = content[after..]
        .find('"')
        .unwrap_or_else(|| panic!("Closing quote not found after '{prefix}' in {path}"));
    let mut result = String::with_capacity(content.len());
    result.push_str(&content[..after]);
    result.push_str(version);
    result.push_str(&content[after + end..]);
    fs::write(path, result).unwrap_or_else(|e| panic!("Failed to write {path}: {e}"));
}
