use std::fs;

fn main() {
    let content = fs::read_to_string("Cargo.toml").expect("Failed to read Cargo.toml");
    let prefix = "version = \"";
    let start = content.find(prefix).expect("Version not found in Cargo.toml");
    let after = start + prefix.len();
    let end = content[after..].find('"').expect("Closing quote not found");
    println!("{}", &content[after..after + end]);
}
