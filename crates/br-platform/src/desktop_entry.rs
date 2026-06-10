//! Parsing of freedesktop `.desktop` entry files (used by the Linux platform
//! integration to discover browsers and their launch commands, RF-39).

/// A parsed `.desktop` file's `[Desktop Entry]` section, relevant fields only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopEntry {
    pub name: String,
    pub exec: String,
    pub mime_types: Vec<String>,
    pub no_display: bool,
}

impl DesktopEntry {
    /// Whether this entry declares itself a handler for `http`/`https` URLs.
    pub fn is_browser(&self) -> bool {
        self.mime_types
            .iter()
            .any(|m| m == "x-scheme-handler/http" || m == "x-scheme-handler/https")
    }

    /// Returns the executable path/name from `Exec=`, with `%`-field codes
    /// (`%u`, `%U`, `%f`, `%F`, etc.) and any arguments stripped.
    pub fn executable(&self) -> Option<String> {
        self.exec.split_whitespace().next().map(|s| s.to_string())
    }
}

/// Parses the `[Desktop Entry]` section of a `.desktop` file's contents.
/// Returns `None` if the file has no `[Desktop Entry]` section or is missing `Name`/`Exec`.
pub fn parse(contents: &str) -> Option<DesktopEntry> {
    let mut in_section = false;
    let mut name = None;
    let mut exec = None;
    let mut mime_types = Vec::new();
    let mut no_display = false;

    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(section) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            in_section = section == "Desktop Entry";
            continue;
        }
        if !in_section {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();
            match key {
                "Name" => name = Some(value.to_string()),
                "Exec" => exec = Some(value.to_string()),
                "NoDisplay" => no_display = value.eq_ignore_ascii_case("true"),
                "MimeType" => {
                    mime_types = value
                        .split(';')
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_string)
                        .collect();
                }
                _ => {}
            }
        }
    }

    Some(DesktopEntry {
        name: name?,
        exec: exec?,
        mime_types,
        no_display,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_browser_desktop_entry() {
        let contents = "\
[Desktop Entry]
Version=1.0
Type=Application
Name=Firefox
Exec=firefox %u
MimeType=text/html;x-scheme-handler/http;x-scheme-handler/https;
NoDisplay=false
";
        let entry = parse(contents).unwrap();
        assert_eq!(entry.name, "Firefox");
        assert_eq!(entry.executable(), Some("firefox".to_string()));
        assert!(entry.is_browser());
        assert!(!entry.no_display);
    }

    #[test]
    fn non_browser_entry_is_not_a_browser() {
        let contents = "\
[Desktop Entry]
Name=GIMP
Exec=gimp %U
MimeType=image/png;image/jpeg;
";
        let entry = parse(contents).unwrap();
        assert!(!entry.is_browser());
    }

    #[test]
    fn missing_required_fields_returns_none() {
        let contents = "\
[Desktop Entry]
Name=Incomplete
";
        assert!(parse(contents).is_none());
    }

    #[test]
    fn ignores_other_sections() {
        let contents = "\
[Desktop Action NewWindow]
Name=New Window
Exec=firefox --new-window %u

[Desktop Entry]
Name=Firefox
Exec=firefox %u
MimeType=x-scheme-handler/http;
";
        let entry = parse(contents).unwrap();
        assert_eq!(entry.name, "Firefox");
        assert_eq!(entry.exec, "firefox %u");
    }
}
