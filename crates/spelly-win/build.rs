// Embeds the kitten icon (resource id 1) into spelly.exe. Works on Windows (rc.exe) and when
// cross-compiling from macOS/Linux (llvm-rc).
fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let _ = embed_resource::compile("spelly.rc", embed_resource::NONE);
    }
    println!("cargo:rerun-if-changed=spelly.rc");
    println!("cargo:rerun-if-changed=../../assets/icon.ico");
}
