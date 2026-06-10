use anyhow::{Context, Result};
use br_core::model::RoutingDecision;
use br_core::{config, engine, model::RoutingContext};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::Command;

#[derive(Parser)]
#[command(name = "br", version, about = "BrowserRouter — link/protocol router")]
struct Cli {
    /// Path to config.toml (defaults to the platform config directory)
    #[arg(long, global = true)]
    config: Option<PathBuf>,

    /// Output machine-readable JSON
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Route a URL and launch the resulting browser/profile
    Open {
        url: String,
        /// Force a specific browser/profile id, ignoring rules
        #[arg(long)]
        app: Option<String>,
        /// Open in private/incognito mode
        #[arg(long)]
        private: bool,
        /// Name of the app the link originated from (best-effort context for rules)
        #[arg(long = "source-app")]
        source_app: Option<String>,
    },
    /// Diagnose configuration and environment
    Doctor,
    /// Configuration management
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
    /// Rule management and testing
    Rules {
        #[command(subcommand)]
        command: RulesCommands,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Print the resolved configuration
    Show,
    /// Validate the configuration file
    Validate,
}

#[derive(Subcommand)]
enum RulesCommands {
    /// List all configured rules
    List,
    /// Show which rule (if any) would handle a URL, without opening anything
    Test {
        url: String,
        #[arg(long = "source-app")]
        source_app: Option<String>,
    },
}

fn config_path(override_path: &Option<PathBuf>) -> PathBuf {
    if let Some(path) = override_path {
        return path.clone();
    }
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("br")
        .join("config.toml")
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let path = config_path(&cli.config);

    match cli.command {
        Commands::Open {
            url,
            app,
            private,
            source_app,
        } => cmd_open(&path, &url, app, private, source_app, cli.json),
        Commands::Doctor => cmd_doctor(&path, cli.json),
        Commands::Config { command } => match command {
            ConfigCommands::Show => cmd_config_show(&path, cli.json),
            ConfigCommands::Validate => cmd_config_validate(&path, cli.json),
        },
        Commands::Rules { command } => match command {
            RulesCommands::List => cmd_rules_list(&path, cli.json),
            RulesCommands::Test { url, source_app } => {
                cmd_rules_test(&path, &url, source_app, cli.json)
            }
        },
    }
}

fn cmd_open(
    path: &PathBuf,
    url: &str,
    app: Option<String>,
    private: bool,
    source_app: Option<String>,
    json: bool,
) -> Result<()> {
    let (cfg, _err) = config::load_or_default(path);

    let decision = if let Some(app) = app {
        RoutingDecision::OpenWith {
            target: app,
            private,
        }
    } else {
        let ctx = RoutingContext { source_app };
        engine::route(url, &ctx, &cfg)
    };

    let normalized = br_core::filters::apply_filters(url, &cfg.filters);

    match &decision {
        RoutingDecision::OpenWith { target, private } => {
            launch(&cfg, target, &normalized, *private)?;
        }
        RoutingDecision::OpenWithAll { targets, private } => {
            for target in targets {
                launch(&cfg, target, &normalized, *private)?;
            }
        }
        RoutingDecision::AskUser => {
            // No picker UI implemented yet (v0.1): report the decision instead of opening anything.
        }
        RoutingDecision::Block => {}
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&decision_json(&decision, &normalized))?);
    } else {
        println!("url: {normalized}");
        println!("decision: {}", describe_decision(&decision));
    }
    Ok(())
}

/// Launches the browser/profile referenced by `target_id` with `url`.
fn launch(cfg: &br_core::Config, target_id: &str, url: &str, private: bool) -> Result<()> {
    let target = cfg
        .browsers
        .iter()
        .find(|b| b.id == target_id)
        .with_context(|| format!("unknown browser target '{target_id}'"))?;

    let executable = if target.executable == "auto" {
        anyhow::bail!(
            "browser '{}' has executable = \"auto\"; automatic detection is not yet implemented",
            target.id
        );
    } else {
        &target.executable
    };

    let mut command = Command::new(executable);
    command.args(&target.args);

    if let Some(profile_dir) = &target.profile_dir {
        command.arg(format!("--profile-directory={profile_dir}"));
    }
    if let Some(profile_name) = &target.profile_name {
        command.arg("-P").arg(profile_name);
    }
    if private {
        match target.kind.as_str() {
            "chromium" => {
                command.arg("--incognito");
            }
            "firefox" => {
                command.arg("--private-window");
            }
            _ => {}
        }
    }
    command.arg(url);

    command
        .spawn()
        .with_context(|| format!("failed to launch '{executable}'"))?;
    Ok(())
}

fn cmd_doctor(path: &PathBuf, json: bool) -> Result<()> {
    let (cfg, err) = config::load_or_default(path);
    let info = serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "os": std::env::consts::OS,
        "config_path": path.display().to_string(),
        "config_valid": err.is_none(),
        "config_error": err.map(|e| e.to_string()),
        "browsers_configured": cfg.browsers.len(),
        "rules_configured": cfg.rules.len(),
        "filters_configured": cfg.filters.len(),
    });
    if json {
        println!("{}", serde_json::to_string_pretty(&info)?);
    } else {
        println!("br {}", env!("CARGO_PKG_VERSION"));
        println!("os: {}", std::env::consts::OS);
        println!("config path: {}", path.display());
        println!("config valid: {}", info["config_valid"]);
        if let Some(e) = info["config_error"].as_str() {
            println!("config error: {e}");
        }
        println!("browsers configured: {}", cfg.browsers.len());
        println!("rules configured: {}", cfg.rules.len());
        println!("filters configured: {}", cfg.filters.len());
    }
    Ok(())
}

fn cmd_config_show(path: &PathBuf, json: bool) -> Result<()> {
    let (cfg, err) = config::load_or_default(path);
    if let Some(err) = err {
        eprintln!("warning: {err}");
    }
    if json {
        println!("{}", serde_json::to_string_pretty(&cfg)?);
    } else {
        println!("{}", toml::to_string_pretty(&cfg)?);
    }
    Ok(())
}

fn cmd_config_validate(path: &PathBuf, json: bool) -> Result<()> {
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    match config::parse(&contents) {
        Ok(cfg) => {
            if json {
                println!(
                    "{}",
                    serde_json::json!({"valid": true, "rules": cfg.rules.len(), "browsers": cfg.browsers.len()})
                );
            } else {
                println!("config is valid ({} rules, {} browsers)", cfg.rules.len(), cfg.browsers.len());
            }
            Ok(())
        }
        Err(err) => {
            if json {
                println!("{}", serde_json::json!({"valid": false, "error": err.to_string()}));
                Ok(())
            } else {
                Err(err)
            }
        }
    }
}

fn cmd_rules_list(path: &PathBuf, json: bool) -> Result<()> {
    let (cfg, _err) = config::load_or_default(path);
    let mut rules = cfg.rules.clone();
    rules.sort_by(|a, b| b.priority.cmp(&a.priority));

    if json {
        println!("{}", serde_json::to_string_pretty(&rules)?);
    } else {
        for rule in &rules {
            println!(
                "[{:>4}] {} ({}) {}",
                rule.priority,
                rule.id,
                if rule.enabled { "enabled" } else { "disabled" },
                rule.name
            );
        }
    }
    Ok(())
}

fn cmd_rules_test(path: &PathBuf, url: &str, source_app: Option<String>, json: bool) -> Result<()> {
    let (cfg, _err) = config::load_or_default(path);
    let ctx = RoutingContext { source_app };
    let explanation = engine::explain(url, &ctx, &cfg);

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "normalized_url": explanation.normalized_url,
                "matched_rule": explanation.matched_rule,
                "decision": decision_json(&explanation.decision, &explanation.normalized_url),
            }))?
        );
    } else {
        println!("normalized url: {}", explanation.normalized_url);
        match &explanation.matched_rule {
            Some(id) => println!("matched rule: {id}"),
            None => println!("matched rule: (none — using default action)"),
        }
        println!("decision: {}", describe_decision(&explanation.decision));
    }
    Ok(())
}

fn describe_decision(decision: &RoutingDecision) -> String {
    match decision {
        RoutingDecision::OpenWith { target, private } => {
            format!("open_with {target}{}", if *private { " (private)" } else { "" })
        }
        RoutingDecision::OpenWithAll { targets, private } => format!(
            "open_with_all [{}]{}",
            targets.join(", "),
            if *private { " (private)" } else { "" }
        ),
        RoutingDecision::AskUser => "ask".to_string(),
        RoutingDecision::Block => "block".to_string(),
    }
}

fn decision_json(decision: &RoutingDecision, _url: &str) -> serde_json::Value {
    match decision {
        RoutingDecision::OpenWith { target, private } => {
            serde_json::json!({"type": "open_with", "target": target, "private": private})
        }
        RoutingDecision::OpenWithAll { targets, private } => {
            serde_json::json!({"type": "open_with_all", "targets": targets, "private": private})
        }
        RoutingDecision::AskUser => serde_json::json!({"type": "ask"}),
        RoutingDecision::Block => serde_json::json!({"type": "block"}),
    }
}
