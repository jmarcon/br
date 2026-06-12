use crate::model::Filter;
use regex::Regex;
use url::Url;

/// Applies all enabled filters in order to `url`, returning the normalized URL.
///
/// Unparseable URLs are returned unchanged so that filters never block routing.
pub fn apply_filters(input: &str, filters: &[Filter]) -> String {
    let mut current = input.to_string();
    for filter in filters {
        if !filter.enabled {
            continue;
        }
        if !filter.strip_query_params.is_empty() {
            current = strip_query_params(&current, &filter.strip_query_params);
        }
        if filter.upgrade_http_to_https {
            current = upgrade_https(&current, &filter.exceptions);
        }
        if let (Some(pattern), Some(replacement)) =
            (&filter.rewrite_pattern, &filter.rewrite_replacement)
        {
            current = rewrite(&current, pattern, replacement);
        }
    }
    current
}

/// Removes query parameters matching any of `patterns` (glob `*` wildcards supported).
fn strip_query_params(input: &str, patterns: &[String]) -> String {
    let Ok(mut url) = Url::parse(input) else {
        return input.to_string();
    };

    let kept: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(k, _)| !patterns.iter().any(|p| glob_match(p, k)))
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();

    if kept.is_empty() {
        url.set_query(None);
    } else {
        let mut serializer = url::form_urlencoded::Serializer::new(String::new());
        for (k, v) in &kept {
            serializer.append_pair(k, v);
        }
        url.set_query(Some(&serializer.finish()));
    }
    url.to_string()
}

/// Upgrades `http://` URLs to `https://`, unless the host matches an exception pattern.
fn upgrade_https(input: &str, exceptions: &[String]) -> String {
    let Ok(mut url) = Url::parse(input) else {
        return input.to_string();
    };
    if url.scheme() != "http" {
        return input.to_string();
    }
    let host = url.host_str().unwrap_or("");
    if exceptions.iter().any(|p| glob_match(p, host)) {
        return input.to_string();
    }
    let _ = url.set_scheme("https");
    url.to_string()
}

/// Applies a regex rewrite, supporting named capture groups and `${name}` substitution.
fn rewrite(input: &str, pattern: &str, replacement: &str) -> String {
    let Ok(re) = Regex::new(pattern) else {
        return input.to_string();
    };
    re.replace(input, replacement).into_owned()
}

/// Matches `text` against a glob `pattern` where `*` matches any sequence of characters.
pub fn glob_match(pattern: &str, text: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 1 {
        return pattern == text;
    }

    let mut pos = 0;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        if i == 0 {
            if !text[pos..].starts_with(part) {
                return false;
            }
            pos += part.len();
        } else if i == parts.len() - 1 {
            return text[pos..].ends_with(part);
        } else {
            match text[pos..].find(part) {
                Some(found) => pos += found + part.len(),
                None => return false,
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn filter(strip: &[&str], upgrade: bool, exceptions: &[&str]) -> Filter {
        Filter {
            id: "f".to_string(),
            enabled: true,
            strip_query_params: strip.iter().map(|s| s.to_string()).collect(),
            upgrade_http_to_https: upgrade,
            exceptions: exceptions.iter().map(|s| s.to_string()).collect(),
            rewrite_pattern: None,
            rewrite_replacement: None,
        }
    }

    #[test]
    fn strips_tracking_params_preserving_others() {
        let f = filter(&["utm_*", "gclid", "fbclid"], false, &[]);
        let out = apply_filters(
            "https://example.com/page?utm_source=x&id=42&gclid=abc&fbclid=def",
            &[f],
        );
        assert_eq!(out, "https://example.com/page?id=42");
    }

    #[test]
    fn upgrades_http_to_https() {
        let f = filter(&[], true, &["*.local", "localhost"]);
        let out = apply_filters("http://example.com/", &[f]);
        assert_eq!(out, "https://example.com/");
    }

    #[test]
    fn upgrade_respects_exceptions() {
        let f = filter(&[], true, &["*.local", "localhost"]);
        assert_eq!(
            apply_filters("http://localhost/", std::slice::from_ref(&f)),
            "http://localhost/"
        );
        assert_eq!(
            apply_filters("http://app.local/", &[f]),
            "http://app.local/"
        );
    }

    #[test]
    fn regex_rewrite_with_named_groups() {
        let mut f = filter(&[], false, &[]);
        f.rewrite_pattern = Some(r"https://old\.example\.com/(?P<path>.*)".to_string());
        f.rewrite_replacement = Some("https://new.example.com/${path}".to_string());
        let out = apply_filters("https://old.example.com/foo/bar", &[f]);
        assert_eq!(out, "https://new.example.com/foo/bar");
    }

    #[test]
    fn unparseable_url_passed_through() {
        let f = filter(&["utm_*"], true, &[]);
        assert_eq!(apply_filters("not a url", &[f]), "not a url");
    }

    #[test]
    fn glob_matching() {
        assert!(glob_match("utm_*", "utm_source"));
        assert!(!glob_match("utm_*", "id"));
        assert!(glob_match("*.example.com", "www.example.com"));
        assert!(glob_match("*", "anything"));
        assert!(glob_match("exact", "exact"));
        assert!(!glob_match("exact", "exactly"));
    }
}
