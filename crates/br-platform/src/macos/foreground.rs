/// Best-effort name of the frontmost application, used as `source_app` for
/// rule matching (RF-37 / "best effort" per PRD §12).
///
/// Shells out to `osascript` to ask `System Events` for the name of the
/// frontmost process, since the Cocoa `NSWorkspace` APIs aren't bound here.
pub fn get_foreground_app_name() -> Option<String> {
    let output = std::process::Command::new("osascript")
        .args([
            "-e",
            "tell application \"System Events\" to get name of first application process whose frontmost is true",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}
