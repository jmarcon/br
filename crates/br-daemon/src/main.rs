//! Optional background daemon for `br` (PRD §8.1/§8.3): keeps the config and
//! discovered browsers warm in memory, hot-reloads the config file when it
//! changes on disk, and serves `OPEN` requests from `br open` over a loopback
//! TCP socket to avoid per-link cold-start cost.

#![cfg_attr(windows, windows_subsystem = "windows")]

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

#[cfg(windows)]
fn main() -> Result<()> {
    let (listener, state) = bind_listener()?;
    std::thread::spawn(move || {
        if let Err(err) = accept_connections(listener, state) {
            eprintln!("br-daemon: listener failed: {err}");
        }
    });

    if let Err(err) = windows_tray::run() {
        eprintln!("br-daemon: tray failed: {err}");
        std::thread::park();
    }

    Ok(())
}

#[cfg(not(windows))]
fn main() -> Result<()> {
    let (listener, state) = bind_listener()?;
    accept_connections(listener, state)
}

fn bind_listener() -> Result<(TcpListener, Arc<Mutex<ConfigState>>)> {
    let path = config_path();
    println!("br-daemon: watching config at {}", path.display());

    let state = Arc::new(Mutex::new(ConfigState::load(path)));

    let listener = TcpListener::bind(("127.0.0.1", PORT)).with_context(|| {
        format!("failed to bind 127.0.0.1:{PORT} — is br-daemon already running?")
    })?;
    println!("br-daemon: listening on 127.0.0.1:{PORT}");

    Ok((listener, state))
}

fn accept_connections(listener: TcpListener, state: Arc<Mutex<ConfigState>>) -> Result<()> {
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
    let path = guard.path.clone();
    drop(guard);

    let platform = br_platform::current();

    let decision = if let Some(app) = &request.app {
        RoutingDecision::OpenWith {
            target: app.clone(),
            private: request.private,
        }
    } else {
        let ctx = RoutingContext {
            source_app: request.source_app.clone(),
            modifier_keys: request.modifier_keys.clone(),
        };
        engine::route(&request.url, &ctx, &cfg)
    };
    let normalized = filters::apply_filters(&request.url, &cfg.filters);

    match decision {
        RoutingDecision::OpenWith { target, private } => {
            match br_platform::launch(
                &platform,
                &cfg,
                &target,
                &normalized,
                private || request.private,
            ) {
                Ok(()) => "OK\n",
                Err(err) => {
                    eprintln!("br-daemon: launch failed: {err}");
                    "ERR launch failed\n"
                }
            }
        }
        RoutingDecision::OpenWithAll { targets, private } => {
            for target in &targets {
                let _ = br_platform::launch(
                    &platform,
                    &cfg,
                    target,
                    &normalized,
                    private || request.private,
                );
            }
            "OK\n"
        }
        RoutingDecision::AskUser => match br_ui_picker::show_picker(&normalized, &cfg, Some(&path))
        {
            Ok(()) => "OK\n",
            Err(err) => {
                eprintln!("br-daemon: picker failed: {err}");
                "ERR picker failed\n"
            }
        },
        RoutingDecision::Block => "OK\n",
    }
}

#[cfg(windows)]
mod windows_tray {
    use anyhow::{Context, Result};
    use br_platform::PlatformIntegration;
    use image::GenericImageView;
    use std::os::windows::process::CommandExt;
    use std::process::Command;
    use tray_icon::{
        menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem},
        Icon, TrayIcon, TrayIconBuilder,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, GetMessageW, TranslateMessage, MSG,
    };

    const LOGO_BYTES: &[u8] = include_bytes!("../../../docs/logo_icon_transparent.png");
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    pub fn run() -> Result<()> {
        let _tray = TrayState::new()?;
        unsafe {
            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        Ok(())
    }

    struct TrayState {
        _tray_icon: TrayIcon,
        _menu: Menu,
        _running_item: MenuItem,
        _settings_item: MenuItem,
        _register_item: MenuItem,
        _quit_item: MenuItem,
    }

    impl TrayState {
        fn new() -> Result<Self> {
            let menu = Menu::new();
            let running_item = MenuItem::new("br is running", false, None);
            let settings_item = MenuItem::with_id("open-settings", "Open settings", true, None);
            let register_item =
                MenuItem::with_id("register-default", "Register default browser", true, None);
            let quit_item = MenuItem::with_id("quit", "Quit br", true, None);
            let separator_one = PredefinedMenuItem::separator();
            let separator_two = PredefinedMenuItem::separator();

            menu.append_items(&[
                &running_item,
                &separator_one,
                &settings_item,
                &register_item,
                &separator_two,
                &quit_item,
            ])
            .context("failed to build tray menu")?;

            install_menu_handler(
                settings_item.id().clone(),
                register_item.id().clone(),
                quit_item.id().clone(),
            );

            let tray_icon = TrayIconBuilder::new()
                .with_menu(Box::new(menu.clone()))
                .with_tooltip("BrowserRouter is running")
                .with_icon(load_icon()?)
                .build()
                .context("failed to create tray icon")?;

            Ok(Self {
                _tray_icon: tray_icon,
                _menu: menu,
                _running_item: running_item,
                _settings_item: settings_item,
                _register_item: register_item,
                _quit_item: quit_item,
            })
        }
    }

    fn install_menu_handler(settings_id: MenuId, register_id: MenuId, quit_id: MenuId) {
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            if event.id == settings_id {
                if let Err(err) = open_settings() {
                    eprintln!("br-daemon: failed to open settings: {err}");
                }
            } else if event.id == register_id {
                if let Err(err) = br_platform::current().register_as_default_handler() {
                    eprintln!("br-daemon: failed to register default browser: {err}");
                }
            } else if event.id == quit_id {
                std::process::exit(0);
            }
        }));
    }

    fn load_icon() -> Result<Icon> {
        let image = image::load_from_memory(LOGO_BYTES).context("failed to load tray icon")?;
        let rgba = image.resize(64, 64, image::imageops::FilterType::Lanczos3);
        let (width, height) = rgba.dimensions();
        Icon::from_rgba(rgba.into_rgba8().into_raw(), width, height)
            .context("failed to prepare tray icon")
    }

    fn open_settings() -> Result<()> {
        let settings = std::env::current_exe()
            .context("failed to locate daemon executable")?
            .with_file_name("br-settings.exe");
        Command::new(settings)
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .context("failed to launch br-settings")?;
        Ok(())
    }
}
