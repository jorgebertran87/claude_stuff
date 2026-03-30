fn main() {
    let root = std::path::Path::new(".");
    let src_prompt = root.join("system_prompt");
    let src_example = root.join("system_prompt.example");
    let dest = std::path::Path::new("src/infrastructure/prompt");

    if src_prompt.exists() {
        std::fs::copy(&src_prompt, &dest)
            .expect("failed to copy system_prompt to src/infrastructure/prompt");
    } else if !dest.exists() {
        std::fs::copy(&src_example, &dest)
            .expect("system_prompt missing and system_prompt.example copy failed");
    }

    println!("cargo:rerun-if-changed=system_prompt");
    println!("cargo:rerun-if-changed=system_prompt.example");
}
