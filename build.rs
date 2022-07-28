use std::fs;
fn main() {
    // If the target lib changes, copy it.
    println!("cargo:rerun-if-changed=src/target/mod.rs");

    fs::copy("./src/target/mod.rs", "./target/target.rs").expect("Could not copy files");
}
