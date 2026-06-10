use br_core::BrowserTarget;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use winreg::enums::*;
use winreg::RegKey;

/// Discovers installed browsers via the registry, plus their profiles
/// (Chromium `Local State`, Firefox `profiles.ini`).
pub fn discover() -> Vec<BrowserTarget> {
    let mut targets = Vec::new();

    for client in discover_start_menu_clients() {
        let kind = classify(&client.name);
        match kind {
            "chromium" => targets.extend(chromium_profiles(&client)),
            "firefox" => targets.extend(firefox_profiles(&client)),
            _ => targets.push(BrowserTarget {
                id: slug(&client.name),
                name: client.name,
                kind: "generic".to_string(),
                executable: client.executable,
                args: vec![],
                profile_dir: None,
                profile_name: None,
                icon: None,
                hidden: false,
            }),
        }
    }

    targets
}

struct InstalledClient {
    name: String,
    executable: String,
}

/// Reads `HKLM\SOFTWARE\Clients\StartMenuInternet` (and its WOW6432Node mirror)
/// for installed browsers, resolving each to its `shell\open\command` executable.
fn discover_start_menu_clients() -> Vec<InstalledClient> {
    let mut clients = Vec::new();
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

    for base in [
        r"SOFTWARE\Clients\StartMenuInternet",
        r"SOFTWARE\WOW6432Node\Clients\StartMenuInternet",
    ] {
        let Ok(key) = hklm.open_subkey(base) else {
            continue;
        };
        for name in key.enum_keys().flatten() {
            let Ok(sub) = key.open_subkey(&name) else {
                continue;
            };
            let display_name: String = sub
                .open_subkey("")
                .ok()
                .and_then(|k| k.get_value("").ok())
                .unwrap_or_else(|| name.clone());

            let Ok(cmd_key) = sub.open_subkey(r"shell\open\command") else {
                continue;
            };
            let Ok(command): Result<String, _> = cmd_key.get_value("") else {
                continue;
            };

            if let Some(exe) = extract_executable(&command) {
                if clients.iter().any(|c: &InstalledClient| c.executable == exe) {
                    continue;
                }
                clients.push(InstalledClient {
                    name: display_name,
                    executable: exe,
                });
            }
        }
    }

    clients
}

/// Extracts the executable path from a `shell\open\command` value, which is
/// typically `"C:\Path\To\App.exe"` optionally followed by arguments.
fn extract_executable(command: &str) -> Option<String> {
    let trimmed = command.trim();
    if let Some(rest) = trimmed.strip_prefix('"') {
        return rest.split('"').next().map(|s| s.to_string());
    }
    // Unquoted paths may contain spaces (e.g. "C:\Program Files\...\iexplore.exe" %1):
    // take everything up to and including the first ".exe".
    if let Some(idx) = trimmed.to_lowercase().find(".exe") {
        return Some(trimmed[..idx + 4].to_string());
    }
    trimmed.split_whitespace().next().map(|s| s.to_string())
}

fn classify(name: &str) -> &'static str {
    let lower = name.to_lowercase();
    if lower.contains("firefox") || lower.contains("librewolf") || lower.contains("zen") {
        "firefox"
    } else if lower.contains("chrome")
        || lower.contains("edge")
        || lower.contains("brave")
        || lower.contains("vivaldi")
        || lower.contains("opera")
        || lower.contains("chromium")
        || lower.contains("arc")
    {
        "chromium"
    } else {
        "generic"
    }
}

fn slug(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

#[derive(Deserialize)]
struct LocalState {
    profile: ProfileSection,
}

#[derive(Deserialize)]
struct ProfileSection {
    #[serde(default)]
    info_cache: HashMap<String, ProfileInfo>,
}

#[derive(Deserialize)]
struct ProfileInfo {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    user_name: Option<String>,
    #[serde(default)]
    gaia_name: Option<String>,
}

/// Reads `Local State` for a Chromium-based browser and returns one [`BrowserTarget`]
/// per profile. Falls back to a single "default profile" target if `Local State`
/// is missing or unparseable, so the browser is never silently dropped.
fn chromium_profiles(client: &InstalledClient) -> Vec<BrowserTarget> {
    let base_id = slug(&client.name);

    let Some(local_state_path) = chromium_local_state_path(&client.executable) else {
        return vec![fallback_target(client, &base_id)];
    };

    let Ok(contents) = std::fs::read_to_string(&local_state_path) else {
        return vec![fallback_target(client, &base_id)];
    };

    let Ok(state) = serde_json::from_str::<LocalState>(&contents) else {
        return vec![fallback_target(client, &base_id)];
    };

    if state.profile.info_cache.is_empty() {
        return vec![fallback_target(client, &base_id)];
    }

    let mut profiles: Vec<(String, ProfileInfo)> = state.profile.info_cache.into_iter().collect();
    profiles.sort_by(|a, b| a.0.cmp(&b.0));

    profiles
        .into_iter()
        .map(|(dir, info)| {
            let display_name = info
                .name
                .or(info.gaia_name)
                .or(info.user_name)
                .unwrap_or_else(|| dir.clone());
            BrowserTarget {
                id: format!("{base_id}-{}", slug(&dir)),
                name: format!("{} ({display_name})", client.name),
                kind: "chromium".to_string(),
                executable: client.executable.clone(),
                args: vec![],
                profile_dir: Some(dir),
                profile_name: None,
                icon: None,
                hidden: false,
            }
        })
        .collect()
}

fn fallback_target(client: &InstalledClient, base_id: &str) -> BrowserTarget {
    BrowserTarget {
        id: base_id.to_string(),
        name: client.name.clone(),
        kind: "chromium".to_string(),
        executable: client.executable.clone(),
        args: vec![],
        profile_dir: None,
        profile_name: None,
        icon: None,
        hidden: false,
    }
}

/// Maps a Chromium browser executable to its `User Data\Local State` path under
/// `%LOCALAPPDATA%`, based on well-known per-vendor data directories.
fn chromium_local_state_path(executable: &str) -> Option<PathBuf> {
    let local_app_data = dirs::data_local_dir()?;
    let exe_lower = executable.to_lowercase();

    let user_data_dir = if exe_lower.contains("msedge") {
        "Microsoft\\Edge\\User Data"
    } else if exe_lower.contains("brave") {
        "BraveSoftware\\Brave-Browser\\User Data"
    } else if exe_lower.contains("vivaldi") {
        "Vivaldi\\User Data"
    } else if exe_lower.contains("opera") {
        return None; // Opera uses a different profile layout; not yet supported
    } else {
        "Google\\Chrome\\User Data"
    };

    let path = local_app_data.join(user_data_dir).join("Local State");
    path.exists().then_some(path)
}

/// Reads `profiles.ini` for Firefox-family browsers and returns one [`BrowserTarget`]
/// per profile. Falls back to a single "default profile" target if `profiles.ini`
/// is missing or unparseable.
fn firefox_profiles(client: &InstalledClient) -> Vec<BrowserTarget> {
    let base_id = slug(&client.name);

    let Some(profiles_ini) = firefox_profiles_ini_path(&client.name) else {
        return vec![fallback_target_firefox(client, &base_id)];
    };

    let Ok(contents) = std::fs::read_to_string(&profiles_ini) else {
        return vec![fallback_target_firefox(client, &base_id)];
    };

    let profiles = parse_profiles_ini(&contents);
    if profiles.is_empty() {
        return vec![fallback_target_firefox(client, &base_id)];
    }

    profiles
        .into_iter()
        .map(|name| BrowserTarget {
            id: format!("{base_id}-{}", slug(&name)),
            name: format!("{} ({name})", client.name),
            kind: "firefox".to_string(),
            executable: client.executable.clone(),
            args: vec![],
            profile_dir: None,
            profile_name: Some(name),
            icon: None,
            hidden: false,
        })
        .collect()
}

fn fallback_target_firefox(client: &InstalledClient, base_id: &str) -> BrowserTarget {
    BrowserTarget {
        id: base_id.to_string(),
        name: client.name.clone(),
        kind: "firefox".to_string(),
        executable: client.executable.clone(),
        args: vec![],
        profile_dir: None,
        profile_name: None,
        icon: None,
        hidden: false,
    }
}

fn firefox_profiles_ini_path(name: &str) -> Option<PathBuf> {
    let app_data = dirs::config_dir()?; // %APPDATA% on Windows
    let lower = name.to_lowercase();
    let vendor_dir = if lower.contains("librewolf") {
        "LibreWolf"
    } else if lower.contains("zen") {
        "zen"
    } else {
        "Mozilla\\Firefox"
    };
    let path = app_data.join(vendor_dir).join("profiles.ini");
    path.exists().then_some(path)
}

/// Parses `profiles.ini`, returning the `Name=` of each `[ProfileN]` section.
fn parse_profiles_ini(contents: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut in_profile_section = false;
    for line in contents.lines() {
        let line = line.trim();
        if let Some(section) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            in_profile_section = section.starts_with("Profile");
            continue;
        }
        if in_profile_section {
            if let Some(value) = line.strip_prefix("Name=") {
                names.push(value.trim().to_string());
            }
        }
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_profiles_ini_names() {
        let ini = "\
[Profile1]
Name=default-release
IsRelative=1
Path=xxxxxxxx.default-release

[Profile0]
Name=Work
IsRelative=1
Path=yyyyyyyy.work

[General]
StartWithLastProfile=1
";
        let names = parse_profiles_ini(ini);
        assert_eq!(names, vec!["default-release".to_string(), "Work".to_string()]);
    }

    #[test]
    fn extracts_executable_from_quoted_command() {
        assert_eq!(
            extract_executable(r#""C:\Program Files\Google\Chrome\Application\chrome.exe""#),
            Some(r"C:\Program Files\Google\Chrome\Application\chrome.exe".to_string())
        );
    }

    #[test]
    fn extracts_executable_with_trailing_args() {
        assert_eq!(
            extract_executable(r#""C:\Browser\app.exe" -- "%1""#),
            Some(r"C:\Browser\app.exe".to_string())
        );
    }

    #[test]
    fn classify_browsers() {
        assert_eq!(classify("Google Chrome"), "chromium");
        assert_eq!(classify("Microsoft Edge"), "chromium");
        assert_eq!(classify("Mozilla Firefox"), "firefox");
        assert_eq!(classify("Internet Explorer"), "generic");
    }

    #[test]
    fn slug_normalizes_names() {
        assert_eq!(slug("Google Chrome"), "google-chrome");
        assert_eq!(slug("Firefox (Work)"), "firefox--work");
    }
}
