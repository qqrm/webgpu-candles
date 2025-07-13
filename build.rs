fn main() {
    println!("cargo:rustc-env=INSTA_WORKSPACE_ROOT={}", env!("CARGO_MANIFEST_DIR"));
}
