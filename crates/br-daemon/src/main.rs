//! Optional background daemon for `br` (PRD §8.1/§8.3): keeps the config and
//! discovered browsers warm in memory, hot-reloads the config file when it
//! changes on disk, and serves `OPEN` requests from `br open` over a loopback
//! TCP socket to avoid per-link cold-start cost.

use anyhow::{Context, Result};
use br_core::{config, engine, filters, model::RoutingContext, model::RoutingDecision, Config};
use br_daemon::{OpenRequest, PORT};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

struct ConfigState {
    config: Config,
    path: PathBuf,
    last_modified: Option<SystemTime>,
}

impl ConfigState {
    fn load(path: PathBuf) -> Self {
        let (config, _err) = config::load_or_default(&path);
        let last_modified = file_mtime(&path);
        Self {
            config,
            path,
            last_modified,
        }
    }

    /// Reloads the config from disk if the file's mtime has changed since last load.
    fn reload_if_changed(&mut self) {
        let mtime = file_mtime(&self.path);
        if mtime != self.last_modified {
            let (config, err) = config::load_or_default(&self.path);
            if let Some(err) = err {
                eprintln!("br-daemon: failed to reload config, keeping previous: {err}");
            } else {
                eprintln!("br-daemon: reloaded config from {}", self.path.display());
                self.config = config;
            }
            self.last_modified = mtime;
        }
    }
}

fn file_mtime(path: &PathBuf) -> Option<SystemTime> {
    std::fs::metadata(path).ok()?.modified().ok()
}

fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("br")
        .join("config.toml")
}

fn main() -> Result<()> {
    let path = config_path();
    println!("br-daemon: watching config at {}", path.display());

    let state = Arc::new(Mutex::new(ConfigState::load(path)));

    let listener = TcpListener::bind(("127.0.0.1", PORT))
        .with_context(|| format!("failed to bind 127.0.0.1:{PORT} — is br-daemon already running?"))?;
    println!("br-daemon: listening on 127.0.0.1:{PORT}");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let state = Arc::clone(&state);
                std::thread::spawn(move || {
                    if let Err(err) = handle_connection(stream, &state) {
                        eprintln!("br-daemon: connection error: {err}");
                    }
                });
            }
            Err(err) => eprintln!("br-daemon: accept error: {err}"),
        }
    }

    Ok(())
}

fn handle_connection(mut stream: TcpStream, state: &Arc<Mutex<ConfigState>>) -> Result<()> {
    let mut buf = [0u8; 4096];
    let n = stream.read(&mut buf)?;
    let line = String::from_utf8_lossy(&buf[..n]);

    let Some(request) = OpenRequest::decode(&line) else {
        stream.write_all(b"ERR invalid request\n")?;
        return Ok(());
    };

    let response = handle_open(request, state);
    stream.write_all(response.as_bytes())?;
    Ok(())
}

fn handle_open(request: OpenRequest, state: &Arc<Mutex<ConfigState>>) -> &'static str {
    let mut guard = match state.lock() {
        Ok(guard) => guard,
        Err(_) => return "ERR poisoned state\n",
    };
    guard.reload_if_changed();
    let cfg = guard.config.clone();
    drop(guard);

    let normalized = filters::apply_filters(&request.url, &cfg.filters);
    let platform = br_platform::current();

    let decision = if let Some(app) = &request.app {
        RoutingDecision::OpenWith {
            target: app.clone(),
            private: request.private,
        }
    } else {
        let ctx = RoutingContext {
            source_app: request.source_app.clone(),
        };
        engine::route(&normalized, &ctx, &cfg)
    };

    match decision {
        RoutingDecision::OpenWith { target, private } => {
            match br_platform::launch(&platform, &cfg, &target, &normalized, private || request.private) {
                Ok(()) => "OK\n",
                Err(err) => {
                    eprintln!("br-daemon: launch failed: {err}");
                    "ERR launch failed\n"
                }
            }
        }
        RoutingDecision::OpenWithAll { targets, private } => {
            for target in &targets {
                let _ = br_platform::launch(&platform, &cfg, target, &normalized, private || request.private);
            }
            "OK\n"
        }
        RoutingDecision::AskUser => match br_ui_picker::show_picker(&normalized, &cfg) {
            Ok(()) => "OK\n",
            Err(err) => {
                eprintln!("br-daemon: picker failed: {err}");
                "ERR picker failed\n"
            }
        },
        RoutingDecision::Block => "OK\n",
    }
}
