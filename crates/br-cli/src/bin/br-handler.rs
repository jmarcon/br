#![cfg_attr(windows, windows_subsystem = "windows")]

use br_core::{config, engine, model::RoutingContext, RoutingDecision};
use br_platform::PlatformIntegration;
use std::path::{Path, PathBuf};

fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("br")
        .join("config.toml")
}

fn request_url() -> Option<String> {
    let mut args = std::env::args().skip(1);
    match args.next()?.as_str() {
        "open" => args.next(),
        first => Some(first.to_string()),
    }
}

fn route_url(path: &Path, url: &str) -> anyhow::Result<()> {
    let source_app = br_platform::current().get_foreground_app_name();
    let request = br_daemon::OpenRequest {
        url: url.to_string(),
        source_app: source_app.clone(),
        app: None,
        private: false,
        modifier_keys: Vec::new(),
    };
    if matches!(br_daemon::client::try_dispatch(&request), Ok(true)) {
        return Ok(());
    }

    let (cfg, _err) = config::load_or_default(path);
    let ctx = RoutingContext {
        source_app,
        modifier_keys: Vec::new(),
    };
    let decision = engine::route(url, &ctx, &cfg);
    let normalized = br_core::filters::apply_filters(url, &cfg.filters);
    let platform = br_platform::current();

    match decision {
        RoutingDecision::OpenWith { target, private } => {
            br_platform::launch(&platform, &cfg, &target, &normalized, private)?;
        }
        RoutingDecision::OpenWithAll { targets, private } => {
            for target in targets {
                br_platform::launch(&platform, &cfg, &target, &normalized, private)?;
            }
        }
        RoutingDecision::AskUser => {
            br_ui_picker::show_picker(&normalized, &cfg, Some(path))?;
        }
        RoutingDecision::Block => {}
    }

    Ok(())
}

fn main() {
    let Some(url) = request_url() else {
        return;
    };
    let _ = route_url(&config_path(), &url);
}
