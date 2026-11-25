#[cfg(target_os = "macos")]
mod mac;

#[cfg(target_os = "macos")]
fn main() {
    mac::run();
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("spelly-mac is the macOS shell.");
}
