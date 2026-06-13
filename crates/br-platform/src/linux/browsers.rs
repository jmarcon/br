use crate::desktop_entry::{self, DesktopEntry};
use crate::util::slug;
use br_core::BrowserTarget;
use std::path::{Path, PathBuf};

/// Discovers installed browsers by scanning `.desktop` files in the standard
/// freedesktop application directories for entries that handle
/// `x-scheme-handler/http(s)` (RF-39).
pub fn discover() -> Vec<BrowserTarget> {
    let mut seen_ids = std::collections::HashSet::new();
    let mut targets = Vec::new();

    for dir in application_dirs() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("desktop") {
                continue;
            }
            let Ok(contents) = std::fs::read_to_string(&path) else {
                continue;
            };
            let Some(parsed) = desktop_entry::parse(&contents) else {
                continue;
            };
            if !parsed.is_browser() || parsed.no_display {
                continue;
            }
            let Some(executable) = parsed.executable() else {
                continue;
            };

            let id = slug(&parsed.name);
            if !seen_ids.insert(id.clone()) {
                continue;
            }

            let kind = classify_desktop_entry(&parsed);
            targets.push(BrowserTarget {
                id,
                name: parsed.name,
                kind,
                executable,
                args: vec![],
                profile_dir: None,
                profile_name: None,
                icon: None,
                hidden: false,
            });
        }
    }

    targets
}

fn classify_desktop_entry(entry: &DesktopEntry) -> String {
    crate::util::classify(&entry.name).to_string()
}

/// Standard freedesktop locations for `.desktop` files, in priority order
/// (user-local entries first so they can override system-wide ones).
fn application_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Some(data_home) = std::env::var_os("XDG_DATA_HOME") {
        dirs.push(PathBuf::from(data_home).join("applications"));
    } else if let Some(home) = dirs::home_dir() {
        dirs.push(home.join(".local/share/applications"));
    }

    if let Some(data_dirs) = std::env::var_os("XDG_DATA_DIRS") {
        for dir in std::env::split_paths(&data_dirs) {
            dirs.push(dir.join("applications"));
        }
    } else {
        dirs.push(PathBuf::from("/usr/local/share/applications"));
        dirs.push(PathBuf::from("/usr/share/applications"));
    }

    dirs.into_iter().filter(|d| Path::new(d).is_dir()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_known_browsers() {
        let firefox = desktop_entry::parse(
            "[Desktop Entry]\nName=Firefox\nExec=firefox %u\nMimeType=x-scheme-handler/http;",
        )
        .unwrap();
        assert_eq!(classify_desktop_entry(&firefox), "firefox");

        let chrome = desktop_entry::parse(
            "[Desktop Entry]\nName=Google Chrome\nExec=google-chrome %U\nMimeType=x-scheme-handler/https;",
        )
        .unwrap();
        assert_eq!(classify_desktop_entry(&chrome), "chromium");
    }
}
