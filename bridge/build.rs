use std::path::PathBuf;

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let obj = PathBuf::from(manifest).join("assets").join("icon.o");
        println!("cargo:rerun-if-changed={}", obj.display());
        println!("cargo:rustc-link-arg-bins={}", obj.display());
    }
}
