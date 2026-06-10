//! Small helpers shared across platform implementations.

/// Classifies a browser by its display name into `"chromium"`, `"firefox"`, or `"generic"`.
pub fn classify(name: &str) -> &'static str {
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

/// Normalizes a display name into a lowercase, hyphen-separated id fragment.
pub fn slug(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

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
