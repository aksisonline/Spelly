//! "Start Spelly when I sign in": a registry Run key on Windows, a LaunchAgent on macOS.

use std::path::PathBuf;

/// The background app that sits next to this window's executable.
fn daemon_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    // macOS bundle: <Spelly.app>/Contents/Helpers/Spelly Window.app/Contents/MacOS/spelly-ui,
    // and the background app is <Spelly.app>/Contents/MacOS/Spelly.
    #[cfg(target_os = "macos")]
    if let Some(main) = exe.ancestors().nth(5).map(|c| c.join("MacOS/Spelly")).filter(|p| p.exists()) {
        return Some(main);
    }
    let dir = exe.parent()?.to_path_buf();
    #[cfg(windows)]
    let name = "spelly.exe";
    #[cfg(target_os = "macos")]
    let name = "Spelly";
    #[cfg(not(any(windows, target_os = "macos")))]
    let name = "spelly";
    Some(dir.join(name))
}

#[cfg(windows)]
const KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";

#[cfg(windows)]
pub fn is_enabled() -> bool {
    std::process::Command::new("reg")
        .args(["query", KEY, "/v", "Spelly"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(windows)]
pub fn set_enabled(on: bool) {
    use std::process::Command;
    if on {
        if let Some(p) = daemon_path() {
            let _ = Command::new("reg")
                .args(["add", KEY, "/v", "Spelly", "/t", "REG_SZ", "/d", &format!("\"{}\" --background", p.display()), "/f"])
                .output();
        }
    } else {
        let _ = Command::new("reg").args(["delete", KEY, "/v", "Spelly", "/f"]).output();
    }
}

#[cfg(target_os = "macos")]
fn plist_path() -> PathBuf {
    PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join("Library/LaunchAgents/dev.spelly.app.plist")
}

#[cfg(target_os = "macos")]
pub fn is_enabled() -> bool {
    plist_path().exists()
}

#[cfg(target_os = "macos")]
pub fn set_enabled(on: bool) {
    if on {
        if let Some(p) = daemon_path() {
            let plist = format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\"><dict>\n  <key>Label</key><string>dev.spelly.app</string>\n  <key>ProgramArguments</key><array><string>{}</string><string>--background</string></array>\n  <key>RunAtLoad</key><true/>\n</dict></plist>\n",
                p.display()
            );
            let _ = std::fs::create_dir_all(plist_path().parent().unwrap());
            let _ = std::fs::write(plist_path(), plist);
        }
    } else {
        let _ = std::fs::remove_file(plist_path());
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn is_enabled() -> bool {
    false
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn set_enabled(_on: bool) {
    let _ = daemon_path();
}

/// Start the background app if it isn't running (the "Start Spelly" button).
/// On macOS this goes through LaunchServices, so Spelly itself (not this window) is the app
/// that asks for the Accessibility permission.
pub fn start_daemon() {
    #[cfg(target_os = "macos")]
    if let Some(app) = std::env::current_exe()
        .ok()
        .and_then(|e| e.ancestors().nth(6).map(|p| p.to_path_buf()))
        .filter(|p| p.extension().is_some_and(|x| x == "app"))
    {
        let _ = std::process::Command::new("open").arg(app).args(["--args", "--background"]).spawn();
        return;
    }
    if let Some(p) = daemon_path().filter(|p| p.exists()) {
        let _ = std::process::Command::new(p).arg("--background").spawn();
    }
}
