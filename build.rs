use std::process::Command;

fn main() {
    let output = Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
        .expect("failed to execute rustup");
    let installed = String::from_utf8_lossy(&output.stdout);
    if !installed.lines().any(|l| l.trim() == "wasm32-unknown-unknown") {
        panic!(
            "wasm32-unknown-unknown target not installed; run `rustup target add wasm32-unknown-unknown`"
        );
    }
}
