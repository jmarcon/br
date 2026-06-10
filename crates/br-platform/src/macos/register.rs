use crate::RegisterOutcome;

/// macOS requires the user to pick the default browser via System Settings
/// (`LSSetDefaultHandlerForURLScheme` requires Cocoa bindings not used here),
/// so registration always opens the relevant settings pane and asks the user
/// to complete the change manually (RF-37).
pub fn is_default_handler() -> anyhow::Result<bool> {
    Ok(false)
}

pub fn register_as_default_handler() -> anyhow::Result<RegisterOutcome> {
    let _ = std::process::Command::new("open")
        .args(["x-apple.systempreferences:com.apple.preference.general"])
        .status();

    Ok(RegisterOutcome::NeedsManualConfirmation {
        instructions: "br cannot register itself as the default browser automatically on \
macOS. System Settings > General > Default web browser has been opened — select \
'BrowserRouter' (or its app name) from the list."
            .to_string(),
    })
}
