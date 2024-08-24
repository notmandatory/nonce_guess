use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=assets");
    // only "src/web/template" is supposed to contain maud templates that would affect tailwind
    println!("cargo:rerun-if-changed=src/web/template");

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
