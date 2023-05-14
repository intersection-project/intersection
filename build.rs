fn main() {
    built::write_built_file_with_opts(
        &built::Options::default().set_dependencies(true),
        std::env::var("CARGO_MANIFEST_DIR")
            .expect("Expected CARGO_MANIFEST_DIR in the environment")
            .as_ref(),
        &std::path::Path::new(
            &std::env::var("OUT_DIR").expect("Expected OUT_DIR in the environment"),
        )
        .join("built.rs"),
    )
    .expect("Failed to acquire build-time information");
    lalrpop::process_root().unwrap();
    println!("cargo:rerun-if-changed=src/grammar.lalrpop");
    println!("cargo:rerun-if-changed=.git"); // because of git sha in build data
    println!("cargo:rerun-if-changed=Cargo.lock"); // similar
    println!("cargo:rerun-if-changed=Cargo.toml"); // ditto
}
