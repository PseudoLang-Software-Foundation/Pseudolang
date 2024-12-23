use std::env;
use std::fs;
use std::path::Path;

fn main() {
    static CARGO_PKG_VERSION: &str = "CARGO_PKG_VERSION";
    static OUT_DIR: &str = "OUT_DIR";

    let version = env::var(CARGO_PKG_VERSION).unwrap();
    let out_dir = env::var(OUT_DIR).unwrap();
    let dest_path = Path::new(&out_dir).join("version.rs");

    fs::write(
        &dest_path,
        format!("pub const VERSION: &str = \"{}\";\n", version),
    )
    .unwrap();

    println!("cargo:rerun-if-changed=Cargo.toml");
}
