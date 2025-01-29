use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=./assets");
    // only "./template" is supposed to contain Askama templates that would affect tailwind
    println!("cargo:rerun-if-changed=./templates");

    let mut cmd = Command::new("tailwindcss");
    cmd.args([
        "-c",
        "tailwind.config.js",
        "-i",
        "styles/tailwind.css",
        "-o",
        "assets/main.css",
    ]);
    if cfg!(not(debug_assertions)) {
        cmd.arg("--minify");
    }
    if !cmd.status().expect("failed to run tailwindcss").success() {
        panic!("tailwindcss failed");
    }
}
