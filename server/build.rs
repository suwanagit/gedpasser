//! Make sure `web/dist` exists so the crate compiles before the frontend has
//! been built, and rebuild when the frontend output changes.

fn main() {
    let dist = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../web/dist");
    if !dist.exists() {
        std::fs::create_dir_all(&dist).expect("create web/dist");
    }
    println!("cargo:rerun-if-changed=../web/dist");
}
