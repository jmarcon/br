//! Minimal always-on-top picker window (PRD §5.1 item 4 / §9.2).
//!
//! Shows the URL plus a list of browser/profile targets. The user can pick a
//! target with the mouse, a number key (1-9), or arrow keys + Enter; Esc
//! cancels. Holding Shift while choosing opens the target in private mode.

use br_core::i18n::{tr, Key};
use br_core::{BrowserTarget, Config, DefaultAction, MatchCondition, Rule};
use br_platform::PlatformIntegration;
use eframe::egui;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Shows the picker for `url` and, if the user makes a choice, launches the
/// selected browser/profile. Returns once the window is closed.
///
/// If `config_path` is provided, the picker offers an "Always open
/// `domain.com` in this app" action per target, which appends a host-match
/// rule to the config and saves it.
pub fn show_picker(url: &str, cfg: &Config, config_path: Option<&Path>) -> anyhow::Result<()> {
    let platform = br_platform::current();
    let mut targets = cfg.browsers.iter().filter(|b| !b.hidden).cloned().collect::<Vec<_>>();
    if targets.is_empty() {
        targets = platform.discover_browsers().unwrap_or_default();
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([460.0, 80.0 + 44.0 * targets.len().max(1) as f32])
            .with_always_on_top()
            .with_resizable(false)
            .with_title("br — open with"),
        ..Default::default()
    };

    let url = url.to_string();
    let cfg = cfg.clone();
    let config_path = config_path.map(Path::to_path_buf);
    eframe::run_native(
        "br-picker",
        options,
        Box::new(move |_cc| Ok(Box::new(PickerApp::new(url, cfg, targets, config_path)))),
    )
    .map_err(|e| anyhow::anyhow!("failed to run picker UI: {e}"))
}

/// Extracts the host from `url` (e.g. `https://example.com/path` -> `example.com`).
fn extract_host(url: &str) -> Option<String> {
    let without_scheme = url.split("://").nth(1).unwrap_or(url);
    let host = without_scheme
        .split(['/', '?', '#'])
        .next()
        .unwrap_or(without_scheme);
    let host = host.rsplit('@').next().unwrap_or(host);
    if host.is_empty() {
        None
    } else {
        Some(host.to_string())
    }
}

struct PickerApp {
    url: String,
    cfg: Config,
    targets: Vec<BrowserTarget>,
    selected: usize,
    config_path: Option<PathBuf>,
    opened_at: Instant,
    status: Option<String>,
}

impl PickerApp {
    fn new(url: String, cfg: Config, targets: Vec<BrowserTarget>, config_path: Option<PathBuf>) -> Self {
        Self {
            url,
            cfg,
            targets,
            selected: 0,
            config_path,
            opened_at: Instant::now(),
            status: None,
        }
    }

    fn choose(&self, ctx: &egui::Context, index: usize, private: bool) {
        if let Some(target) = self.targets.get(index) {
            let platform = br_platform::current();
            let _ = br_platform::launch(&platform, &self.cfg, &target.id, &self.url, private);
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    /// Applies the configured timeout fallback (PRD §5.1 item 4): if
    /// `picker_timeout_ms` has elapsed, either open with the configured
    /// default browser or simply close (when the default action is "ask").
    fn apply_timeout(&mut self, ctx: &egui::Context) {
        let timeout_ms = self.cfg.general.picker_timeout_ms;
        if timeout_ms == 0 {
            return;
        }
        if self.opened_at.elapsed().as_millis() < timeout_ms as u128 {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
            return;
        }
        if let DefaultAction::OpenWith(target_id) = &self.cfg.general.default_action.0 {
            if let Some(index) = self.targets.iter().position(|t| &t.id == target_id) {
                self.choose(ctx, index, false);
                return;
            }
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    /// Creates (and saves) a rule that always routes `self.url`'s host to
    /// `target.id` (PRD §5.1 item 4: "Sempre abrir domínio.com neste app").
    fn always_open_in(&mut self, target: &BrowserTarget) {
        let Some(host) = extract_host(&self.url) else {
            self.status = Some("Could not determine the domain for this URL.".to_string());
            return;
        };
        let Some(config_path) = &self.config_path else {
            self.status = Some("No config path available; cannot save rule.".to_string());
            return;
        };

        let next_priority = self.cfg.rules.iter().map(|r| r.priority).max().unwrap_or(0) + 10;
        self.cfg.rules.push(Rule {
            id: format!("always-{host}"),
            name: format!("Always open {host} in {}", target.name),
            enabled: true,
            priority: next_priority,
            match_: MatchCondition {
                host: vec![host.clone()],
                ..Default::default()
            },
            action: br_core::Action {
                open_with: Some(target.id.clone()),
                ..Default::default()
            },
        });

        match toml::to_string_pretty(&self.cfg)
            .map_err(anyhow::Error::from)
            .and_then(|contents| std::fs::write(config_path, contents).map_err(Into::into))
        {
            Ok(()) => {
                self.status = Some(format!("Always opening {host} in {}.", target.name));
            }
            Err(err) => self.status = Some(format!("Failed to save rule: {err}")),
        }
    }
}

impl eframe::App for PickerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.apply_timeout(ctx);

        let shift_held = ctx.input(|i| i.modifiers.shift);

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
            self.selected = (self.selected + 1).min(self.targets.len().saturating_sub(1));
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
            self.selected = self.selected.saturating_sub(1);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Enter)) && !self.targets.is_empty() {
            self.choose(ctx, self.selected, shift_held);
        }
        for (i, key) in [
            egui::Key::Num1,
            egui::Key::Num2,
            egui::Key::Num3,
            egui::Key::Num4,
            egui::Key::Num5,
            egui::Key::Num6,
            egui::Key::Num7,
            egui::Key::Num8,
            egui::Key::Num9,
        ]
        .into_iter()
        .enumerate()
        {
            if ctx.input(|i| i.key_pressed(key)) && i < self.targets.len() {
                self.choose(ctx, i, shift_held);
            }
        }

        let lang = self.cfg.general.language.clone();
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label(format!("{} {}", tr(Key::PickerOpenWith, &lang), self.url));
            ui.separator();

            if self.targets.is_empty() {
                ui.label("No browsers configured or detected.");
                return;
            }

            let mut always_open: Option<usize> = None;
            for (i, target) in self.targets.iter().enumerate() {
                ui.horizontal(|ui| {
                    let label = format!("{}. {}", i + 1, target.name);
                    let selected = i == self.selected;
                    if ui.selectable_label(selected, label).clicked() {
                        self.choose(ctx, i, shift_held);
                    }
                    if self.config_path.is_some() {
                        let host = extract_host(&self.url).unwrap_or_default();
                        if ui
                            .small_button(format!("Always open {host} here"))
                            .clicked()
                        {
                            always_open = Some(i);
                        }
                    }
                });
            }
            if let Some(i) = always_open {
                let target = self.targets[i].clone();
                self.always_open_in(&target);
            }

            ui.separator();
            ui.small(tr(Key::PickerHelp, &lang));
            if let Some(status) = &self.status {
                ui.colored_label(egui::Color32::LIGHT_GREEN, status);
            }
        });
    }
}
