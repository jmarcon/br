use super::PROG_ID;
use crate::RegisterOutcome;
use winreg::enums::*;
use winreg::RegKey;

const APP_REG_NAME: &str = "BrowserRouter";
const CAPABILITIES_PATH: &str = r"Software\BrowserRouter\Capabilities";

/// Checks `HKCU\...\UrlAssociations\http\UserChoice` for `br`'s `ProgId`.
///
/// Windows 10/11 only trust this key when set via the user's explicit choice
/// in Settings, so this reflects whether the user has actually completed that step.
pub fn is_default_handler() -> anyhow::Result<bool> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    for scheme in ["http", "https"] {
        let path = format!(
            r"Software\Microsoft\Windows\Shell\Associations\UrlAssociations\{scheme}\UserChoice"
        );
        let prog_id: anyhow::Result<String> = hkcu
            .open_subkey(&path)
            .and_then(|k| k.get_value("ProgId"))
            .map_err(Into::into);

        match prog_id {
            Ok(id) if id == PROG_ID => continue,
            _ => return Ok(false),
        }
    }
    Ok(true)
}

/// Registers `br` under `HKCU\Software\Classes` and `RegisteredApplications`
/// (RF-37), then opens the Windows "Default apps" settings page so the user
/// can complete the manual confirmation step Windows requires.
pub fn register_as_default_handler() -> anyhow::Result<RegisterOutcome> {
    let exe = std::env::current_exe()?;
    let exe_str = exe.to_string_lossy();
    let command = format!("\"{exe_str}\" open \"%1\"");

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    // 1. ProgId definition: HKCU\Software\Classes\BrowserRouter.HTTP
    let (prog_key, _) = hkcu.create_subkey(format!(r"Software\Classes\{PROG_ID}"))?;
    prog_key.set_value("", &"BrowserRouter")?;
    let (shell_key, _) = prog_key.create_subkey(r"shell\open\command")?;
    shell_key.set_value("", &command)?;

    // 2. Capabilities block describing what br can handle.
    let (cap_key, _) = hkcu.create_subkey(CAPABILITIES_PATH)?;
    cap_key.set_value("ApplicationName", &"BrowserRouter")?;
    cap_key.set_value(
        "ApplicationDescription",
        &"Routes links to the right browser/profile based on rules",
    )?;
    let (url_assoc_key, _) = cap_key.create_subkey("URLAssociations")?;
    url_assoc_key.set_value("http", &PROG_ID)?;
    url_assoc_key.set_value("https", &PROG_ID)?;

    // 3. Register the application so it appears in "Default apps".
    let (registered_apps, _) = hkcu.create_subkey(r"Software\RegisteredApplications")?;
    registered_apps.set_value(APP_REG_NAME, &CAPABILITIES_PATH)?;

    // 4. Open Settings so the user can pick `br` (Windows requires manual confirmation).
    let _ = std::process::Command::new("cmd")
        .args(["/C", "start", "", "ms-settings:defaultapps"])
        .spawn();

    Ok(RegisterOutcome::NeedsManualConfirmation {
        instructions: format!(
            "br has been registered as a candidate handler ({PROG_ID}). \
Windows 'Default apps' settings has been opened — search for 'BrowserRouter' \
and set it as the default for HTTP and HTTPS."
        ),
    })
}
