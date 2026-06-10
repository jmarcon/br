use crate::RegisterOutcome;

const LAUNCH_AGENT_LABEL: &str = "com.browserrouter.daemon";

fn launch_agent_path() -> anyhow::Result<std::path::PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("could not determine home directory"))?;
    Ok(home
        .join("Library/LaunchAgents")
        .join(format!("{LAUNCH_AGENT_LABEL}.plist")))
}

/// Enables/disables `br-daemon` autostart by writing/removing a LaunchAgent
/// plist under `~/Library/LaunchAgents` and (un)loading it via `launchctl`.
pub fn set_autostart(enabled: bool) -> anyhow::Result<()> {
    let plist_path = launch_agent_path()?;

    if enabled {
        if let Some(parent) = plist_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let exe = std::env::current_exe()?;
        let daemon_exe = exe.with_file_name("br-daemon");
        let contents = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{LAUNCH_AGENT_LABEL}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
</dict>
</plist>
"#,
            daemon_exe.display()
        );
        std::fs::write(&plist_path, contents)?;
        let _ = std::process::Command::new("launchctl")
            .args(["load", "-w"])
            .arg(&plist_path)
            .status();
    } else if plist_path.exists() {
        let _ = std::process::Command::new("launchctl")
            .args(["unload", "-w"])
            .arg(&plist_path)
            .status();
        std::fs::remove_file(&plist_path)?;
    }
    Ok(())
}

/// Checks whether the LaunchAgent plist is present.
pub fn is_autostart_enabled() -> anyhow::Result<bool> {
    Ok(launch_agent_path()?.exists())
}

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
