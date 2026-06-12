use crate::model::Config;
use anyhow::{Context, Result};
use std::path::Path;

pub const CURRENT_CONFIG_VERSION: u32 = 1;

/// Parses a TOML config string, validating it for known issues.
pub fn parse(toml_str: &str) -> Result<Config> {
    let config: Config = toml::from_str(toml_str).context("failed to parse config TOML")?;
    validate(&config)?;
    Ok(config)
}

/// Loads a config from disk. Returns the fail-safe default config (which always
/// shows the picker) if the file is missing or invalid, alongside the error if any.
pub fn load_or_default(path: &Path) -> (Config, Option<anyhow::Error>) {
    match std::fs::read_to_string(path) {
        Ok(contents) => match parse(&contents) {
            Ok(config) => (config, None),
            Err(err) => (Config::default(), Some(err)),
        },
        Err(err) => (Config::default(), Some(err.into())),
    }
}

/// Performs structural validation beyond what serde checks: duplicate ids, dangling
/// references from rules to browser targets, and config_version compatibility.
pub fn validate(config: &Config) -> Result<()> {
    if config.config_version > CURRENT_CONFIG_VERSION {
        anyhow::bail!(
            "config_version {} is newer than supported version {}",
            config.config_version,
            CURRENT_CONFIG_VERSION
        );
    }

    let mut browser_ids = std::collections::HashSet::new();
    for browser in &config.browsers {
        if !browser_ids.insert(browser.id.as_str()) {
            anyhow::bail!("duplicate browser id: {}", browser.id);
        }
    }

    let mut rule_ids = std::collections::HashSet::new();
    for rule in &config.rules {
        if !rule_ids.insert(rule.id.as_str()) {
            anyhow::bail!("duplicate rule id: {}", rule.id);
        }

        let mut targets: Vec<&str> = rule
            .action
            .open_with_all
            .iter()
            .map(String::as_str)
            .collect();
        if let Some(t) = &rule.action.open_with {
            targets.push(t);
        }
        for target in targets {
            if !browser_ids.contains(target) {
                anyhow::bail!(
                    "rule '{}' references unknown browser target '{}'",
                    rule.id,
                    target
                );
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_example_config() {
        let toml_str = r##"
config_version = 1

[general]
default_action = "ask"
picker_timeout_ms = 0
picker_position = "cursor"
theme = "system"
picker_background = "bubbles"
picker_background_color = "#1453aa"
picker_window_opacity = 1.0
picker_acrylic = false
picker_icon_size = 72
picker_padding = 20
picker_width = 720
picker_height = 460
language = "pt-BR"
start_on_login = true
log_level = "warn"

[[browsers]]
id = "chrome-default"
name = "Google Chrome"
kind = "chromium"
executable = "auto"

[[filters]]
id = "strip-tracking"
enabled = true
strip_query_params = ["utm_*", "gclid"]

[[rules]]
id = "fallback"
name = "Default: ask"
enabled = true
priority = 0
match = { url_pattern = ["*"] }
action = { ask = true }
"##;
        let config = parse(toml_str).unwrap();
        assert_eq!(config.config_version, 1);
        assert_eq!(config.browsers.len(), 1);
        assert_eq!(config.rules.len(), 1);
    }

    #[test]
    fn rejects_duplicate_rule_ids() {
        let toml_str = r#"
config_version = 1

[[rules]]
id = "dup"
priority = 1
match = { url_pattern = ["*"] }
action = { ask = true }

[[rules]]
id = "dup"
priority = 0
match = { url_pattern = ["*"] }
action = { ask = true }
"#;
        assert!(parse(toml_str).is_err());
    }

    #[test]
    fn rejects_dangling_browser_reference() {
        let toml_str = r#"
config_version = 1

[[rules]]
id = "r1"
priority = 1
match = { url_pattern = ["*"] }
action = { open_with = "does-not-exist" }
"#;
        assert!(parse(toml_str).is_err());
    }

    #[test]
    fn rejects_future_config_version() {
        let toml_str = "config_version = 999\n";
        assert!(parse(toml_str).is_err());
    }

    #[test]
    fn invalid_config_falls_back_to_default() {
        let dir = std::env::temp_dir().join("br-test-config-invalid");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        std::fs::write(&path, "config_version = 999\n").unwrap();

        let (config, err) = load_or_default(&path);
        assert!(err.is_some());
        assert_eq!(config.rules.len(), 0);
        // default config with no rules falls back to "ask"
        let decision = crate::engine::route(
            "https://example.com",
            &crate::model::RoutingContext::default(),
            &config,
        );
        assert_eq!(decision, crate::model::RoutingDecision::AskUser);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn missing_config_falls_back_to_default() {
        let path = std::env::temp_dir().join("br-test-config-missing-12345/config.toml");
        let (config, err) = load_or_default(&path);
        assert!(err.is_some());
        assert_eq!(config.config_version, 0);
    }
}
