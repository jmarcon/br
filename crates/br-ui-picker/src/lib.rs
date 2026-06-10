//! Minimal always-on-top picker window (PRD §5.1 item 4 / §9.2).
//!
//! Shows the URL plus a list of browser/profile targets. The user can pick a
//! target with the mouse, a number key (1-9), or arrow keys + Enter; Esc
//! cancels. Holding Shift while choosing opens the target in private mode.

use br_core::{BrowserTarget, Config};
use br_platform::PlatformIntegration;
use eframe::egui;

/// Shows the picker for `url` and, if the user makes a choice, launches the
/// selected browser/profile. Returns once the window is closed.
pub fn show_picker(url: &str, cfg: &Config) -> anyhow::Result<()> {
    let platform = br_platform::current();
    let mut targets = cfg.browsers.iter().filter(|b| !b.hidden).cloned().collect::<Vec<_>>();
    if targets.is_empty() {
        targets = platform.discover_browsers().unwrap_or_default();
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([420.0, 80.0 + 36.0 * targets.len().max(1) as f32])
            .with_always_on_top()
            .with_resizable(false)
            .with_title("br — open with"),
        ..Default::default()
    };

    let url = url.to_string();
    let cfg = cfg.clone();
    eframe::run_native(
        "br-picker",
        options,
        Box::new(move |_cc| Ok(Box::new(PickerApp::new(url, cfg, targets)))),
    )
    .map_err(|e| anyhow::anyhow!("failed to run picker UI: {e}"))
}

struct PickerApp {
    url: String,
    cfg: Config,
    targets: Vec<BrowserTarget>,
    selected: usize,
}

impl PickerApp {
    fn new(url: String, cfg: Config, targets: Vec<BrowserTarget>) -> Self {
        Self {
            url,
            cfg,
            targets,
            selected: 0,
        }
    }

    fn choose(&self, ctx: &egui::Context, index: usize, private: bool) {
        if let Some(target) = self.targets.get(index) {
            let platform = br_platform::current();
            let _ = br_platform::launch(&platform, &self.cfg, &target.id, &self.url, private);
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
}

impl eframe::App for PickerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label(format!("Open: {}", self.url));
            ui.separator();

            if self.targets.is_empty() {
                ui.label("No browsers configured or detected.");
                return;
            }

            for (i, target) in self.targets.iter().enumerate() {
                let label = format!("{}. {}", i + 1, target.name);
                let selected = i == self.selected;
                if ui.selectable_label(selected, label).clicked() {
                    self.choose(ctx, i, shift_held);
                }
            }

            ui.separator();
            ui.small("↑/↓ + Enter, or 1-9 to choose. Hold Shift for private mode. Esc to cancel.");
        });
    }
}
