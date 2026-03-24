fn main() {
    let dir = std::path::Path::new("src/infrastructure");
    let prompt = dir.join("prompt");
    let example = dir.join("prompt.example");

    if !prompt.exists() {
        std::fs::copy(&example, &prompt)
            .expect("prompt file missing and prompt.example copy failed");
    }

    println!("cargo:rerun-if-changed=src/infrastructure/prompt");
    println!("cargo:rerun-if-changed=src/infrastructure/prompt.example");
}
