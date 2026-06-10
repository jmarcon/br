use anyhow::{Context, Result};
use br_core::model::RoutingDecision;
use br_core::{config, engine, model::RoutingContext};
use br_platform::PlatformIntegration;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

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
    /// Browser/profile discovery
    Browsers {
        #[command(subcommand)]
        command: BrowsersCommands,
    },
    /// Open the settings UI (browsers, filters, rules)
    Settings,
    /// Check whether the background daemon (br-daemon) is running
    DaemonStatus,
    /// Register br as the default http/https handler
    Register,
    /// Remove br as the default http/https handler (manual step on most OSes)
    Unregister,
}

#[derive(Subcommand)]
enum BrowsersCommands {
    /// List browsers/profiles detected on this system
    List,
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Print the resolved configuration
    Show,
    /// Validate the configuration file
    Validate,
    /// Export the configuration to a file (or stdout if no path is given)
    Export {
        /// Destination file (defaults to stdout)
        output: Option<PathBuf>,
    },
    /// Import a configuration file, replacing the current one
    Import {
        /// Source file to import
        input: PathBuf,
    },
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
    /// Add a new rule
    Add {
        /// Unique rule id
        id: String,
        /// Display name (defaults to the id)
        #[arg(long)]
        name: Option<String>,
        /// Priority (higher runs first); defaults to one above the current maximum
        #[arg(long)]
        priority: Option<i64>,
        /// URL glob/regex pattern(s) to match (repeatable)
        #[arg(long = "url-pattern")]
        url_pattern: Vec<String>,
        /// Source application name(s) to match (repeatable)
        #[arg(long = "source-app")]
        source_app: Vec<String>,
        /// Browser/profile id to open matching URLs with
        #[arg(long = "open-with")]
        open_with: Option<String>,
        /// Open in private/incognito mode
        #[arg(long)]
        private: bool,
        /// Block matching URLs instead of opening them
        #[arg(long)]
        block: bool,
        /// Create the rule disabled
        #[arg(long)]
        disabled: bool,
    },
    /// Remove a rule by id
    Rm {
        /// Id of the rule to remove
        id: String,
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
            ConfigCommands::Export { output } => cmd_config_export(&path, output),
            ConfigCommands::Import { input } => cmd_config_import(&path, &input, cli.json),
        },
        Commands::Rules { command } => match command {
            RulesCommands::List => cmd_rules_list(&path, cli.json),
            RulesCommands::Test { url, source_app } => {
                cmd_rules_test(&path, &url, source_app, cli.json)
            }
            RulesCommands::Add {
                id,
                name,
                priority,
                url_pattern,
                source_app,
                open_with,
                private,
                block,
                disabled,
            } => cmd_rules_add(
                &path, id, name, priority, url_pattern, source_app, open_with, private, block,
                disabled, cli.json,
            ),
            RulesCommands::Rm { id } => cmd_rules_rm(&path, &id, cli.json),
        },
        Commands::Browsers { command } => match command {
            BrowsersCommands::List => cmd_browsers_list(cli.json),
        },
        Commands::Settings => br_ui_settings::run(Some(path)),
        Commands::DaemonStatus => {
            let running = br_daemon::client::is_running();
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&serde_json::json!({"running": running}))?);
            } else {
                println!("br-daemon: {}", if running { "running" } else { "not running" });
            }
            Ok(())
        }
        Commands::Register => cmd_register(cli.json),
        Commands::Unregister => cmd_unregister(),
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
    let source_app_for_dispatch = source_app
        .clone()
        .or_else(|| br_platform::current().get_foreground_app_name());

    let dispatch_request = br_daemon::OpenRequest {
        url: url.to_string(),
        source_app: source_app_for_dispatch.clone(),
        app: app.clone(),
        private,
    };
    if matches!(br_daemon::client::try_dispatch(&dispatch_request), Ok(true)) {
        if json {
            println!("{}", serde_json::to_string_pretty(&serde_json::json!({"handled_by": "daemon"}))?);
        } else {
            println!("handled by br-daemon");
        }
        return Ok(());
    }

    let (cfg, _err) = config::load_or_default(path);

    let decision = if let Some(app) = app {
        RoutingDecision::OpenWith {
            target: app,
            private,
        }
    } else {
        let ctx = RoutingContext {
            source_app: source_app_for_dispatch,
        };
        engine::route(url, &ctx, &cfg)
    };

    let normalized = br_core::filters::apply_filters(url, &cfg.filters);

    let platform = br_platform::current();
    match &decision {
        RoutingDecision::OpenWith { target, private } => {
            br_platform::launch(&platform, &cfg, target, &normalized, *private)?;
        }
        RoutingDecision::OpenWithAll { targets, private } => {
            for target in targets {
                br_platform::launch(&platform, &cfg, target, &normalized, *private)?;
            }
        }
        RoutingDecision::AskUser => {
            br_ui_picker::show_picker(&normalized, &cfg)?;
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

fn cmd_doctor(path: &PathBuf, json: bool) -> Result<()> {
    let (cfg, err) = config::load_or_default(path);
    let platform = br_platform::current();
    let is_default = platform.is_default_handler().unwrap_or(false);
    let discovered = platform.discover_browsers().unwrap_or_default();

    let info = serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "os": std::env::consts::OS,
        "config_path": path.display().to_string(),
        "config_valid": err.is_none(),
        "config_error": err.map(|e| e.to_string()),
        "is_default_handler": is_default,
        "browsers_detected": discovered.len(),
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
        println!("default handler: {}", if is_default { "yes" } else { "no (run `br register`)" });
        println!("browsers detected: {}", discovered.len());
        println!("browsers configured: {}", cfg.browsers.len());
        println!("rules configured: {}", cfg.rules.len());
        println!("filters configured: {}", cfg.filters.len());
    }
    Ok(())
}

fn cmd_browsers_list(json: bool) -> Result<()> {
    let discovered = br_platform::current().discover_browsers()?;
    if json {
        println!("{}", serde_json::to_string_pretty(&discovered)?);
    } else if discovered.is_empty() {
        println!("no browsers detected");
    } else {
        for b in &discovered {
            let mut extra = String::new();
            if let Some(p) = &b.profile_dir {
                extra = format!(" [profile_dir={p}]");
            } else if let Some(p) = &b.profile_name {
                extra = format!(" [profile_name={p}]");
            }
            println!("{:<24} {} ({}){}", b.id, b.name, b.kind, extra);
            println!("    executable: {}", b.executable);
        }
    }
    Ok(())
}

fn cmd_register(json: bool) -> Result<()> {
    let outcome = br_platform::current().register_as_default_handler()?;
    match outcome {
        br_platform::RegisterOutcome::Registered => {
            if json {
                println!("{}", serde_json::json!({"status": "registered"}));
            } else {
                println!("br is now the default http/https handler.");
            }
        }
        br_platform::RegisterOutcome::NeedsManualConfirmation { instructions } => {
            if json {
                println!(
                    "{}",
                    serde_json::json!({"status": "needs_manual_confirmation", "instructions": instructions})
                );
            } else {
                println!("{instructions}");
            }
        }
    }
    Ok(())
}

fn cmd_unregister() -> Result<()> {
    println!(
        "Unregistering is not automated yet. Open your OS's default-apps settings \
and choose a different browser for http/https."
    );
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

fn cmd_config_export(path: &PathBuf, output: Option<PathBuf>) -> Result<()> {
    let (cfg, err) = config::load_or_default(path);
    if let Some(err) = err {
        eprintln!("warning: {err}");
    }
    let contents = toml::to_string_pretty(&cfg).context("serializing configuration")?;
    match output {
        Some(output) => {
            std::fs::write(&output, contents)
                .with_context(|| format!("failed to write {}", output.display()))?;
            println!("exported configuration to {}", output.display());
        }
        None => print!("{contents}"),
    }
    Ok(())
}

fn cmd_config_import(path: &PathBuf, input: &PathBuf, json: bool) -> Result<()> {
    let contents = std::fs::read_to_string(input)
        .with_context(|| format!("failed to read {}", input.display()))?;
    let cfg = config::parse(&contents).context("imported file is not a valid configuration")?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context("creating config directory")?;
    }
    std::fs::write(path, toml::to_string_pretty(&cfg)?).context("writing config file")?;

    if json {
        println!(
            "{}",
            serde_json::json!({"imported": true, "rules": cfg.rules.len(), "browsers": cfg.browsers.len()})
        );
    } else {
        println!(
            "imported configuration from {} into {} ({} rules, {} browsers)",
            input.display(),
            path.display(),
            cfg.rules.len(),
            cfg.browsers.len()
        );
    }
    Ok(())
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

#[allow(clippy::too_many_arguments)]
fn cmd_rules_add(
    path: &PathBuf,
    id: String,
    name: Option<String>,
    priority: Option<i64>,
    url_pattern: Vec<String>,
    source_app: Vec<String>,
    open_with: Option<String>,
    private: bool,
    block: bool,
    disabled: bool,
    json: bool,
) -> Result<()> {
    let (mut cfg, _err) = config::load_or_default(path);

    if cfg.rules.iter().any(|r| r.id == id) {
        anyhow::bail!("a rule with id '{id}' already exists");
    }

    let priority = priority.unwrap_or_else(|| {
        cfg.rules.iter().map(|r| r.priority).max().unwrap_or(0) + 10
    });

    let rule = br_core::Rule {
        id: id.clone(),
        name: name.unwrap_or_else(|| id.clone()),
        enabled: !disabled,
        priority,
        match_: br_core::MatchCondition {
            url_pattern,
            source_app,
            ..Default::default()
        },
        action: br_core::Action {
            ask: open_with.is_none() && !block,
            open_with,
            private,
            block,
            ..Default::default()
        },
    };

    cfg.rules.push(rule);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context("creating config directory")?;
    }
    std::fs::write(path, toml::to_string_pretty(&cfg)?).context("writing config file")?;

    if json {
        println!("{}", serde_json::json!({"added": id}));
    } else {
        println!("added rule '{id}'");
    }
    Ok(())
}

fn cmd_rules_rm(path: &PathBuf, id: &str, json: bool) -> Result<()> {
    let (mut cfg, _err) = config::load_or_default(path);
    let before = cfg.rules.len();
    cfg.rules.retain(|r| r.id != id);
    if cfg.rules.len() == before {
        anyhow::bail!("no rule with id '{id}' found");
    }

    std::fs::write(path, toml::to_string_pretty(&cfg)?).context("writing config file")?;

    if json {
        println!("{}", serde_json::json!({"removed": id}));
    } else {
        println!("removed rule '{id}'");
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
