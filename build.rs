fn main() {
    built::write_built_file().expect("Failed to acquire build-time information");
    lalrpop::process_root().unwrap();
    println!("cargo:rerun-if-changed=src/grammar.lalrpop");
    println!("cargo:rerun-if-changed=.git"); // because of git sha in build data
    println!("cargo:rerun-if-changed=Cargo.lock"); // similar
    println!("cargo:rerun-if-changed=Cargo.toml"); // ditto
}
