use std::path::PathBuf;

fn main() {
    // Copy schema/agent-exec.schema.json to OUT_DIR/../ (i.e., target/debug/ or target/release/)
    // so the binary can locate it at runtime during development and testing.
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");
    // OUT_DIR is typically target/debug/build/<crate>-<hash>/out; go up 3 levels to reach
    // target/debug/ (or target/release/).
    let out_path = PathBuf::from(&out_dir);
    let bin_dir = out_path
        .ancestors()
        .nth(3)
        .expect("cannot determine bin directory from OUT_DIR")
        .to_path_buf();

    let src = PathBuf::from("schema").join("agent-exec.schema.json");
    let dst_dir = bin_dir.join("schema");
    let dst = dst_dir.join("agent-exec.schema.json");

    // Create the destination directory if needed.
    std::fs::create_dir_all(&dst_dir).expect("create schema dir in output");
    std::fs::copy(&src, &dst)
        .unwrap_or_else(|e| panic!("failed to copy {} to {}: {e}", src.display(), dst.display()));

    // Re-run if the schema file changes.
    println!("cargo:rerun-if-changed=schema/agent-exec.schema.json");
    println!("cargo:rerun-if-changed=build.rs");
}
