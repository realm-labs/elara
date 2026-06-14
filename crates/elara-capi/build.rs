use std::path::PathBuf;

fn main() {
    let include_dir = PathBuf::from("include")
        .canonicalize()
        .expect("C API include directory must exist");
    println!("cargo:rerun-if-changed=include/lua.h");
    println!("cargo:rerun-if-changed=include/lauxlib.h");
    println!("cargo:rerun-if-changed=include/lualib.h");
    println!("cargo:include={}", include_dir.display());
    println!(
        "cargo:rustc-env=ELARA_CAPI_INCLUDE_DIR={}",
        include_dir.display()
    );
}
