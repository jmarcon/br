use crate::filters::{apply_filters, glob_match};
use crate::model::{
    Action, Config, DefaultAction, MatchCondition, QueryParamMatch, RoutingContext,
    RoutingDecision, RoutingExplanation, Rule,
};
use regex::Regex;
use std::cmp::Reverse;
use url::Url;

/// Applies filters then evaluates rules in priority order, returning the routing decision.
pub fn route(url: &str, ctx: &RoutingContext, config: &Config) -> RoutingDecision {
    explain(url, ctx, config).decision
}

/// Like [`route`], but also reports which rule (if any) was matched. Used by `br rules test`.
pub fn explain(url: &str, ctx: &RoutingContext, config: &Config) -> RoutingExplanation {
    let normalized = apply_filters(url, &config.filters);

    let mut rules: Vec<&Rule> = config.rules.iter().filter(|r| r.enabled).collect();
    rules.sort_by_key(|rule| Reverse(rule.priority));

    for rule in rules {
        if matches(&rule.match_, &normalized, ctx) {
            let decision = decision_for_action(&rule.action, &config.general.default_action.0);
            return RoutingExplanation {
                normalized_url: normalized,
                matched_rule: Some(rule.id.clone()),
                decision,
            };
        }
    }

    RoutingExplanation {
        normalized_url: normalized,
        matched_rule: None,
        decision: default_decision(&config.general.default_action.0),
    }
}

fn matches(cond: &MatchCondition, url: &str, ctx: &RoutingContext) -> bool {
    if !cond.url_pattern.is_empty() && !any_pattern_matches(&cond.url_pattern, url) {
        return false;
    }

    if !cond.host.is_empty() {
        let host = Url::parse(url)
            .ok()
            .and_then(|u| u.host_str().map(|h| h.to_string()))
            .unwrap_or_default();
        if !cond.host.iter().any(|p| glob_match(p, &host)) {
            return false;
        }
    }

    if !cond.path.is_empty() {
        let path = Url::parse(url)
            .ok()
            .map(|u| u.path().to_string())
            .unwrap_or_default();
        if !any_pattern_matches(&cond.path, &path) {
            return false;
        }
    }

    if !cond.query_param.is_empty() && !query_params_match(url, &cond.query_param) {
        return false;
    }

    if !cond.source_app.is_empty() {
        match &ctx.source_app {
            Some(app) => {
                if !cond.source_app.iter().any(|p| source_app_matches(p, app)) {
                    return false;
                }
            }
            None => return false,
        }
    }

    if !cond.modifier_keys.is_empty()
        && !cond
            .modifier_keys
            .iter()
            .all(|wanted| ctx.modifier_keys.iter().any(|held| key_eq(wanted, held)))
    {
        return false;
    }

    true
}

/// A pattern matches if it's a glob match, or (when prefixed `regex:`) a regex match.
fn any_pattern_matches(patterns: &[String], url: &str) -> bool {
    patterns.iter().any(|p| {
        if let Some(re_src) = p.strip_prefix("regex:") {
            Regex::new(re_src)
                .map(|re| re.is_match(url))
                .unwrap_or(false)
        } else {
            glob_match(p, url)
        }
    })
}

fn source_app_matches(pattern: &str, app: &str) -> bool {
    if let Some(re_src) = pattern.strip_prefix("regex:") {
        Regex::new(re_src)
            .map(|re| re.is_match(app))
            .unwrap_or(false)
    } else if pattern.contains('*') {
        glob_match(pattern, app)
    } else {
        pattern.eq_ignore_ascii_case(app)
    }
}

fn query_params_match(url: &str, conditions: &[QueryParamMatch]) -> bool {
    let Ok(parsed) = Url::parse(url) else {
        return false;
    };
    let params: Vec<(String, String)> = parsed
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();

    conditions.iter().all(|condition| match condition {
        QueryParamMatch::Name(name) => params.iter().any(|(k, _)| k == name),
        QueryParamMatch::Condition { name, value } => params.iter().any(|(k, v)| {
            if k != name {
                return false;
            }
            value
                .as_ref()
                .map(|expected| any_pattern_matches(std::slice::from_ref(expected), v))
                .unwrap_or(true)
        }),
    })
}

fn key_eq(a: &str, b: &str) -> bool {
    let normalize = |key: &str| match key.to_ascii_lowercase().as_str() {
        "cmd" | "command" | "meta" => "cmd".to_string(),
        "ctrl" | "control" => "ctrl".to_string(),
        "option" | "alt" => "alt".to_string(),
        "shift" => "shift".to_string(),
        other => other.to_string(),
    };
    normalize(a) == normalize(b)
}

fn decision_for_action(action: &Action, default_action: &DefaultAction) -> RoutingDecision {
    if action.ask {
        return RoutingDecision::AskUser;
    }
    if action.block {
        return RoutingDecision::Block;
    }
    if !action.open_with_all.is_empty() {
        return RoutingDecision::OpenWithAll {
            targets: action.open_with_all.clone(),
            private: action.private,
        };
    }
    if let Some(target) = &action.open_with {
        return RoutingDecision::OpenWith {
            target: target.clone(),
            private: action.private,
        };
    }
    default_decision(default_action)
}

fn default_decision(default_action: &DefaultAction) -> RoutingDecision {
    match default_action {
        DefaultAction::Ask => RoutingDecision::AskUser,
        DefaultAction::OpenWith(id) => RoutingDecision::OpenWith {
            target: id.clone(),
            private: false,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::General;

    fn rule(id: &str, priority: i64, url_pattern: &str, action: Action) -> Rule {
        Rule {
            id: id.to_string(),
            name: id.to_string(),
            enabled: true,
            priority,
            match_: MatchCondition {
                url_pattern: vec![url_pattern.to_string()],
                ..Default::default()
            },
            action,
        }
    }

    fn config_with_rules(rules: Vec<Rule>) -> Config {
        Config {
            config_version: 1,
            general: General::default(),
            browsers: vec![],
            filters: vec![],
            rules,
        }
    }

    #[test]
    fn picks_highest_priority_matching_rule() {
        let config = config_with_rules(vec![
            rule(
                "low",
                1,
                "*://*.example.com/*",
                Action {
                    open_with: Some("low-target".to_string()),
                    ..Default::default()
                },
            ),
            rule(
                "high",
                100,
                "*://*.example.com/*",
                Action {
                    open_with: Some("high-target".to_string()),
                    ..Default::default()
                },
            ),
        ]);

        let decision = route(
            "https://www.example.com/page",
            &RoutingContext::default(),
            &config,
        );
        assert_eq!(
            decision,
            RoutingDecision::OpenWith {
                target: "high-target".to_string(),
                private: false,
            }
        );
    }

    #[test]
    fn regex_pattern_matching() {
        let config = config_with_rules(vec![rule(
            "r1",
            10,
            r"regex:^https://mail\.google\.com/.*",
            Action {
                open_with: Some("chrome-work".to_string()),
                ..Default::default()
            },
        )]);

        let decision = route(
            "https://mail.google.com/mail/u/0/",
            &RoutingContext::default(),
            &config,
        );
        assert_eq!(
            decision,
            RoutingDecision::OpenWith {
                target: "chrome-work".to_string(),
                private: false,
            }
        );

        let decision = route("https://other.com/", &RoutingContext::default(), &config);
        assert_eq!(decision, RoutingDecision::AskUser);
    }

    #[test]
    fn source_app_match() {
        let config = config_with_rules(vec![rule(
            "r1",
            10,
            "*",
            Action {
                open_with: Some("chrome-work".to_string()),
                ..Default::default()
            },
        )]);
        let mut config = config;
        config.rules[0].match_.source_app = vec!["Slack".to_string()];

        let ctx_match = RoutingContext {
            source_app: Some("Slack".to_string()),
            modifier_keys: Vec::new(),
        };
        assert_eq!(
            route("https://example.com", &ctx_match, &config),
            RoutingDecision::OpenWith {
                target: "chrome-work".to_string(),
                private: false,
            }
        );

        let ctx_other = RoutingContext {
            source_app: Some("Outlook".to_string()),
            modifier_keys: Vec::new(),
        };
        assert_eq!(
            route("https://example.com", &ctx_other, &config),
            RoutingDecision::AskUser
        );
    }

    #[test]
    fn ask_action_forces_picker_even_if_matched() {
        let config = config_with_rules(vec![rule(
            "ask-rule",
            10,
            "*",
            Action {
                ask: true,
                ..Default::default()
            },
        )]);
        assert_eq!(
            route("https://example.com", &RoutingContext::default(), &config),
            RoutingDecision::AskUser
        );
    }

    #[test]
    fn block_action() {
        let config = config_with_rules(vec![rule(
            "block-rule",
            10,
            "*://*.ads.example.com/*",
            Action {
                block: true,
                ..Default::default()
            },
        )]);
        assert_eq!(
            route(
                "https://x.ads.example.com/x",
                &RoutingContext::default(),
                &config
            ),
            RoutingDecision::Block
        );
    }

    #[test]
    fn open_with_all_action() {
        let config = config_with_rules(vec![rule(
            "multi",
            10,
            "*",
            Action {
                open_with_all: vec!["chrome".to_string(), "firefox".to_string()],
                private: false,
                ..Default::default()
            },
        )]);
        assert_eq!(
            route("https://example.com", &RoutingContext::default(), &config),
            RoutingDecision::OpenWithAll {
                targets: vec!["chrome".to_string(), "firefox".to_string()],
                private: false,
            }
        );
    }

    #[test]
    fn no_match_falls_back_to_default_action() {
        let mut config = config_with_rules(vec![]);
        assert_eq!(
            route("https://example.com", &RoutingContext::default(), &config),
            RoutingDecision::AskUser
        );

        config.general.default_action =
            crate::model::DefaultActionSerde(DefaultAction::OpenWith("chrome".to_string()));
        assert_eq!(
            route("https://example.com", &RoutingContext::default(), &config),
            RoutingDecision::OpenWith {
                target: "chrome".to_string(),
                private: false,
            }
        );
    }

    #[test]
    fn filters_applied_before_rule_matching() {
        let config_str = r#"
config_version = 1

[[filters]]
id = "strip"
enabled = true
strip_query_params = ["utm_*"]

[[rules]]
id = "clean-only"
enabled = true
priority = 10
match = { url_pattern = ["https://example.com/page"] }
action = { open_with = "chrome" }
"#;
        let config: Config = toml::from_str(config_str).unwrap();
        let explanation = explain(
            "https://example.com/page?utm_source=newsletter",
            &RoutingContext::default(),
            &config,
        );
        assert_eq!(explanation.normalized_url, "https://example.com/page");
        assert_eq!(explanation.matched_rule, Some("clean-only".to_string()));
    }

    #[test]
    fn path_query_and_modifier_matches() {
        let config_str = r#"
config_version = 1

[[rules]]
id = "debug-with-shift"
enabled = true
priority = 10
match = { host = ["example.com"], path = ["/debug/*"], query_param = [{ name = "mode", value = "preview" }], modifier_keys = ["shift"] }
action = { open_with = "chrome" }

[[browsers]]
id = "chrome"
name = "Chrome"
kind = "chromium"
executable = "chrome"
"#;
        let config: Config = toml::from_str(config_str).unwrap();
        let ctx = RoutingContext {
            source_app: None,
            modifier_keys: vec!["Shift".to_string()],
        };
        let decision = route("https://example.com/debug/page?mode=preview", &ctx, &config);
        assert_eq!(
            decision,
            RoutingDecision::OpenWith {
                target: "chrome".to_string(),
                private: false,
            }
        );
    }

    #[test]
    fn invalid_filter_field_does_not_block_rule() {
        // a rule with no patterns matches everything (AND of empty constraints)
        let config = config_with_rules(vec![Rule {
            id: "catch-all".to_string(),
            name: "catch all".to_string(),
            enabled: true,
            priority: 0,
            match_: MatchCondition::default(),
            action: Action {
                ask: true,
                ..Default::default()
            },
        }]);
        assert_eq!(
            route("https://example.com", &RoutingContext::default(), &config),
            RoutingDecision::AskUser
        );
    }

    #[test]
    fn disabled_rules_are_skipped() {
        let mut r = rule(
            "disabled",
            100,
            "*",
            Action {
                open_with: Some("chrome".to_string()),
                ..Default::default()
            },
        );
        r.enabled = false;
        let config = config_with_rules(vec![r]);
        assert_eq!(
            route("https://example.com", &RoutingContext::default(), &config),
            RoutingDecision::AskUser
        );
    }
}
