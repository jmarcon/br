//! Parsing of macOS `Info.plist` data (used by the macOS platform integration
//! to identify browsers via their `CFBundleURLTypes` declarations, RF-39).

/// Relevant subset of an application bundle's `Info.plist`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleInfo {
    pub name: String,
    pub bundle_id: String,
    pub url_schemes: Vec<String>,
}

impl BundleInfo {
    /// Whether this bundle declares itself a handler for `http`/`https` URLs.
    pub fn is_browser(&self) -> bool {
        self.url_schemes
            .iter()
            .any(|s| s.eq_ignore_ascii_case("http") || s.eq_ignore_ascii_case("https"))
    }
}

/// Extracts bundle name, identifier, and declared URL schemes from a parsed
/// `plist::Value` (the root dictionary of an `Info.plist`).
#[cfg(target_os = "macos")]
pub fn from_plist(value: &plist::Value, fallback_name: &str) -> Option<BundleInfo> {
    let dict = value.as_dictionary()?;

    let bundle_id = dict
        .get("CFBundleIdentifier")
        .and_then(|v| v.as_string())
        .map(str::to_string)?;

    let name = dict
        .get("CFBundleDisplayName")
        .or_else(|| dict.get("CFBundleName"))
        .and_then(|v| v.as_string())
        .map(str::to_string)
        .unwrap_or_else(|| fallback_name.to_string());

    let mut url_schemes = Vec::new();
    if let Some(types) = dict.get("CFBundleURLTypes").and_then(|v| v.as_array()) {
        for entry in types {
            if let Some(schemes) = entry
                .as_dictionary()
                .and_then(|d| d.get("CFBundleURLSchemes"))
                .and_then(|v| v.as_array())
            {
                for scheme in schemes {
                    if let Some(s) = scheme.as_string() {
                        url_schemes.push(s.to_string());
                    }
                }
            }
        }
    }

    Some(BundleInfo {
        name,
        bundle_id,
        url_schemes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn browser_bundle() -> BundleInfo {
        BundleInfo {
            name: "Safari".to_string(),
            bundle_id: "com.apple.Safari".to_string(),
            url_schemes: vec!["https".to_string(), "http".to_string()],
        }
    }

    #[test]
    fn detects_browser_by_url_scheme() {
        assert!(browser_bundle().is_browser());
    }

    #[test]
    fn non_browser_bundle_is_not_a_browser() {
        let bundle = BundleInfo {
            name: "Preview".to_string(),
            bundle_id: "com.apple.Preview".to_string(),
            url_schemes: vec!["pdf".to_string()],
        };
        assert!(!bundle.is_browser());
    }

    #[test]
    fn url_scheme_match_is_case_insensitive() {
        let bundle = BundleInfo {
            name: "Example".to_string(),
            bundle_id: "com.example.app".to_string(),
            url_schemes: vec!["HTTP".to_string()],
        };
        assert!(bundle.is_browser());
    }
}
