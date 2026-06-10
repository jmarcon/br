use crate::RegisterOutcome;

const DESKTOP_FILE_NAME: &str = "browserrouter.desktop";
const AUTOSTART_FILE_NAME: &str = "browserrouter-daemon.desktop";

/// Enables/disables `br-daemon` autostart by writing/removing a `.desktop`
/// file under `~/.config/autostart` (freedesktop autostart spec).
pub fn set_autostart(enabled: bool) -> anyhow::Result<()> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("could not determine XDG config directory"))?;
    let autostart_dir = config_dir.join("autostart");
    let autostart_file = autostart_dir.join(AUTOSTART_FILE_NAME);

    if enabled {
        std::fs::create_dir_all(&autostart_dir)?;
        let exe = std::env::current_exe()?;
        let daemon_exe = exe.with_file_name("br-daemon");
        let contents = format!(
            "[Desktop Entry]\n\
Type=Application\n\
Name=BrowserRouter Daemon\n\
Exec={}\n\
Terminal=false\n\
NoDisplay=true\n\
X-GNOME-Autostart-enabled=true\n",
            daemon_exe.display()
        );
        std::fs::write(&autostart_file, contents)?;
    } else if autostart_file.exists() {
        std::fs::remove_file(&autostart_file)?;
    }
    Ok(())
}

/// Checks whether the autostart `.desktop` file exists.
pub fn is_autostart_enabled() -> anyhow::Result<bool> {
    let Some(config_dir) = dirs::config_dir() else {
        return Ok(false);
    };
    Ok(config_dir.join("autostart").join(AUTOSTART_FILE_NAME).exists())
}

/// Checks `xdg-settings get default-web-browser` against `br`'s `.desktop` file.
pub fn is_default_handler() -> anyhow::Result<bool> {
    let output = std::process::Command::new("xdg-settings")
        .args(["get", "default-web-browser"])
        .output()?;
    if !output.status.success() {
        return Ok(false);
    }
    let current = String::from_utf8_lossy(&output.stdout);
    Ok(current.trim() == DESKTOP_FILE_NAME)
}

/// Installs a `.desktop` file declaring `br` as an `x-scheme-handler/http(s)`
/// handler under `~/.local/share/applications`, then registers it via
/// `xdg-mime default` and `xdg-settings set default-web-browser` (RF-37).
pub fn register_as_default_handler() -> anyhow::Result<RegisterOutcome> {
    let exe = std::env::current_exe()?;
    let exe_str = exe.to_string_lossy();

    let apps_dir = dirs::data_local_dir()
        .ok_or_else(|| anyhow::anyhow!("could not determine XDG data directory"))?
        .join("applications");
    std::fs::create_dir_all(&apps_dir)?;

    let desktop_file = apps_dir.join(DESKTOP_FILE_NAME);
    let contents = format!(
        "[Desktop Entry]\n\
Type=Application\n\
Name=BrowserRouter\n\
Comment=Routes links to the right browser/profile based on rules\n\
Exec={exe_str} open %u\n\
Terminal=false\n\
NoDisplay=true\n\
MimeType=x-scheme-handler/http;x-scheme-handler/https;\n"
    );
    std::fs::write(&desktop_file, contents)?;

    let _ = std::process::Command::new("xdg-mime")
        .args(["default", DESKTOP_FILE_NAME, "x-scheme-handler/http"])
        .status();
    let _ = std::process::Command::new("xdg-mime")
        .args(["default", DESKTOP_FILE_NAME, "x-scheme-handler/https"])
        .status();
    let set_status = std::process::Command::new("xdg-settings")
        .args(["set", "default-web-browser", DESKTOP_FILE_NAME])
        .status();

    match set_status {
        Ok(status) if status.success() => Ok(RegisterOutcome::Registered),
        _ => Ok(RegisterOutcome::NeedsManualConfirmation {
            instructions: format!(
                "br has been installed as {} but could not be set as the default \
browser automatically. Run `xdg-settings set default-web-browser {DESKTOP_FILE_NAME}` \
or set it via your desktop environment's settings.",
                desktop_file.display()
            ),
        }),
    }
}
