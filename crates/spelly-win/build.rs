// Embeds the cat icon into spelly.exe. Only does anything when building on Windows.
fn main() {
    #[cfg(windows)]
    {
        if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
            let _ = winresource::WindowsResource::new().set_icon("../../assets/icon.ico").compile();
        }
    }
}
