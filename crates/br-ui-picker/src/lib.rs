//! Always-on-top visual browser/profile picker.

use br_core::i18n::{tr, Key};
use br_core::{BrowserTarget, Config, DefaultAction, MatchCondition, Rule};
use br_platform::PlatformIntegration;
use eframe::egui;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

const LOGO_BYTES: &[u8] = include_bytes!("../../../docs/logo_icon_transparent.png");

fn app_icon_data() -> Option<egui::IconData> {
    let image = image::load_from_memory(LOGO_BYTES).ok()?.to_rgba8();
    let (width, height) = image.dimensions();
    Some(egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    })
}

pub fn show_picker(url: &str, cfg: &Config, config_path: Option<&Path>) -> anyhow::Result<()> {
    let platform = br_platform::current();
    let mut targets = cfg
        .browsers
        .iter()
        .filter(|b| !b.hidden)
        .cloned()
        .collect::<Vec<_>>();
    if targets.is_empty() {
        targets = platform.discover_browsers().unwrap_or_default();
    }

    let mut picker_cfg = cfg.clone();
    if picker_cfg.browsers.is_empty() {
        picker_cfg.browsers = targets.clone();
    }

    let options = eframe::NativeOptions {
        viewport: {
            let mut viewport = egui::ViewportBuilder::default()
                .with_inner_size([
                    picker_cfg.general.picker_width.clamp(520.0, 1200.0),
                    picker_cfg.general.picker_height.clamp(360.0, 900.0),
                ])
                .with_min_inner_size([520.0, 360.0])
                .with_always_on_top()
                .with_resizable(true)
                .with_decorations(false)
                .with_transparent(true)
                .with_title("br - open with");
            if let Some(icon) = app_icon_data() {
                viewport = viewport.with_icon(icon);
            }
            viewport
        },
        ..Default::default()
    };

    let url = url.to_string();
    let config_path = config_path.map(Path::to_path_buf);
    eframe::run_native(
        "br-picker",
        options,
        Box::new(move |_cc| {
            Ok(Box::new(PickerApp::new(
                url,
                picker_cfg,
                targets,
                config_path,
            )))
        }),
    )
    .map_err(|e| anyhow::anyhow!("failed to run picker UI: {e}"))
}

fn extract_host(url: &str) -> Option<String> {
    let without_scheme = url.split("://").nth(1).unwrap_or(url);
    let host = without_scheme
        .split(['/', '?', '#'])
        .next()
        .unwrap_or(without_scheme);
    let host = host.rsplit('@').next().unwrap_or(host);
    (!host.is_empty()).then(|| host.to_string())
}

fn pick_background_image_file() -> Option<String> {
    rfd::FileDialog::new()
        .add_filter("Images", &["png", "jpg", "jpeg", "ico"])
        .pick_file()
        .map(|path| path.display().to_string())
}

struct PickerApp {
    url: String,
    cfg: Config,
    targets: Vec<BrowserTarget>,
    selected: usize,
    config_path: Option<PathBuf>,
    opened_at: Instant,
    status: Option<String>,
    show_options: bool,
    background_override: Option<String>,
    textures: HashMap<String, egui::TextureHandle>,
    options_color_buffer: String,
    grid_columns: usize,
}

impl PickerApp {
    fn new(
        url: String,
        cfg: Config,
        targets: Vec<BrowserTarget>,
        config_path: Option<PathBuf>,
    ) -> Self {
        let options_color_buffer = cfg.general.picker_background_color.clone();
        Self {
            url,
            cfg,
            targets,
            selected: 0,
            config_path,
            opened_at: Instant::now(),
            status: None,
            show_options: false,
            background_override: None,
            textures: HashMap::new(),
            options_color_buffer,
            grid_columns: 3,
        }
    }

    fn choose(&self, ctx: &egui::Context, index: usize, private: bool) {
        if let Some(target) = self.targets.get(index) {
            let platform = br_platform::current();
            let _ = br_platform::launch(&platform, &self.cfg, &target.id, &self.url, private);
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    fn open_settings(&mut self) {
        let Ok(exe) = std::env::current_exe() else {
            self.status = Some("Could not locate executable.".to_string());
            return;
        };
        let settings = exe.with_file_name(if cfg!(windows) {
            "br-settings.exe"
        } else {
            "br-settings"
        });
        let mut command = std::process::Command::new(settings);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        match command.spawn() {
            Ok(_) => self.status = Some("Settings opened.".to_string()),
            Err(err) => self.status = Some(format!("Could not open settings: {err}")),
        }
    }

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

    fn always_open_selected(&mut self) {
        let Some(target) = self.targets.get(self.selected).cloned() else {
            return;
        };
        let Some(host) = extract_host(&self.url) else {
            self.status = Some("Could not determine domain.".to_string());
            return;
        };
        let Some(config_path) = &self.config_path else {
            self.status = Some("No config path available.".to_string());
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
                open_with: Some(target.id),
                ..Default::default()
            },
        });

        match toml::to_string_pretty(&self.cfg)
            .map_err(anyhow::Error::from)
            .and_then(|contents| std::fs::write(config_path, contents).map_err(Into::into))
        {
            Ok(()) => self.status = Some(format!("Always opening {host} here.")),
            Err(err) => self.status = Some(format!("Failed to save rule: {err}")),
        }
    }

    fn handle_keys(&mut self, ctx: &egui::Context, shift_held: bool) {
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
            self.selected = (self.selected + 1).min(self.targets.len().saturating_sub(1));
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
            self.selected = self.selected.saturating_sub(1);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
            self.selected =
                (self.selected + self.grid_columns).min(self.targets.len().saturating_sub(1));
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
            self.selected = self.selected.saturating_sub(self.grid_columns);
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
    }

    fn header(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, lang: &str) {
        let header_rect = egui::Rect::from_min_size(
            ui.min_rect().min,
            egui::vec2((ui.available_width() - 198.0).max(120.0), 46.0),
        );
        let drag = ui.interact(
            header_rect,
            ui.id().with("drag-header"),
            egui::Sense::drag(),
        );
        if drag.drag_started() {
            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }

        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(header_title(&self.url, tr(Key::PickerOpenWith, lang)))
                    .size(22.0)
                    .color(egui::Color32::WHITE)
                    .strong(),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let opacity = picker_opacity(&self.cfg);
                if icon_button(ui, HeaderIcon::Close, "Close", opacity).clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                if icon_button(ui, HeaderIcon::Settings, "Settings", opacity).clicked() {
                    self.open_settings();
                }
                if icon_button(ui, HeaderIcon::Menu, "Options", opacity).clicked() {
                    self.show_options = !self.show_options;
                }
            });
        });
    }
}

impl eframe::App for PickerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.apply_timeout(ctx);
        ctx.set_visuals(egui::Visuals::dark());
        let shift_held = ctx.input(|i| i.modifiers.shift);
        self.handle_keys(ctx, shift_held);

        let lang = self.cfg.general.language.clone();
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(egui::Color32::TRANSPARENT))
            .show(ctx, |ui| {
                let panel_rect = ui.max_rect().shrink2(egui::vec2(12.0, 12.0));
                let background = self
                    .background_override
                    .as_deref()
                    .unwrap_or(&self.cfg.general.picker_background);
                draw_panel_background(ui, panel_rect, &self.cfg, background, &mut self.textures);
                draw_acrylic_overlay(
                    ui,
                    panel_rect,
                    self.cfg.general.picker_acrylic,
                    picker_opacity(&self.cfg),
                );
                ui.allocate_new_ui(
                    egui::UiBuilder::new().max_rect(panel_rect.shrink(20.0)),
                    |ui| {
                        self.header(ui, ctx, &lang);
                        ui.add_space(self.cfg.general.picker_padding.clamp(0.0, 56.0));

                        if self.targets.is_empty() {
                            ui.vertical_centered(|ui| {
                                ui.add_space(64.0);
                                ui.label(
                                    egui::RichText::new("No browsers configured or detected.")
                                        .color(egui::Color32::WHITE),
                                );
                            });
                            return;
                        }

                        let content_rect = browser_grid_rect(ui.max_rect(), panel_rect, &self.cfg);
                        let mut chosen = None;
                        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(content_rect), |ui| {
                            chosen = self.browser_grid(ui, content_rect);
                        });

                        if let Some(status) = &self.status {
                            ui.add_space(10.0);
                            ui.vertical_centered(|ui| {
                                ui.small(egui::RichText::new(status).color(success_color()));
                            });
                        }

                        if let Some(i) = chosen {
                            self.choose(ctx, i, shift_held);
                        }
                    },
                );
                if self.show_options {
                    self.options_menu(ui, ctx, panel_rect);
                }
                self.footer(ui, panel_rect);
            });
    }
}

impl PickerApp {
    fn browser_grid(&mut self, ui: &mut egui::Ui, content_rect: egui::Rect) -> Option<usize> {
        let tile_width = tile_width(&self.cfg);
        let tile_height = tile_height(&self.cfg);
        let gap = self.cfg.general.picker_padding.clamp(0.0, 56.0);
        let columns = columns_for_width(content_rect.width(), tile_width, gap);
        self.grid_columns = columns;

        let rows = self.targets.len().div_ceil(columns);
        let grid_height = rows as f32 * tile_height + rows.saturating_sub(1) as f32 * 8.0;
        let top_pad = ((content_rect.height() - grid_height) / 2.0).max(0.0);
        let scroll_id = (
            "browser-grid",
            columns,
            (content_rect.width() / 8.0).round() as i32,
            (content_rect.height() / 8.0).round() as i32,
        );

        let mut chosen = None;
        egui::ScrollArea::vertical()
            .id_salt(scroll_id)
            .max_height(content_rect.height())
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(top_pad);
                for (row_index, row_targets) in self.targets.chunks(columns).enumerate() {
                    let row_width = row_targets.len() as f32 * tile_width
                        + row_targets.len().saturating_sub(1) as f32 * gap;
                    let left_pad = ((content_rect.width() - row_width) / 2.0).max(0.0);
                    ui.horizontal(|ui| {
                        ui.add_space(left_pad);
                        ui.spacing_mut().item_spacing.x = gap;
                        for (col, target) in row_targets.iter().enumerate() {
                            let i = row_index * columns + col;
                            let response = profile_tile(
                                ui,
                                target,
                                i + 1,
                                i == self.selected,
                                &mut self.textures,
                                &self.cfg,
                            );
                            if response.clicked() {
                                chosen = Some(i);
                            }
                            if response.hovered() {
                                self.selected = i;
                            }
                        }
                    });
                    ui.add_space(8.0);
                }
            });
        chosen
    }

    fn save_config(&mut self) {
        let Some(config_path) = &self.config_path else {
            self.status = Some("No config path available.".to_string());
            return;
        };
        match toml::to_string_pretty(&self.cfg)
            .map_err(anyhow::Error::from)
            .and_then(|contents| std::fs::write(config_path, contents).map_err(Into::into))
        {
            Ok(()) => self.status = Some("Picker settings saved.".to_string()),
            Err(err) => self.status = Some(format!("Failed to save settings: {err}")),
        }
    }

    fn save_current_size(&mut self, panel_rect: egui::Rect) {
        self.cfg.general.picker_width = (panel_rect.width() + 24.0).round();
        self.cfg.general.picker_height = (panel_rect.height() + 24.0).round();
        self.save_config();
    }

    fn reset_default_size(&mut self, ctx: &egui::Context) {
        self.cfg.general.picker_width = 720.0;
        self.cfg.general.picker_height = 460.0;
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(720.0, 460.0)));
        self.save_config();
    }

    fn options_menu(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, panel_rect: egui::Rect) {
        let pos = panel_rect.right_top() + egui::vec2(-304.0, 76.0);
        let opacity = picker_opacity(&self.cfg);
        egui::Area::new(egui::Id::new("picker-options-menu"))
            .order(egui::Order::Foreground)
            .fixed_pos(pos)
            .show(ui.ctx(), |ui| {
                egui::Frame::none()
                    .fill(alpha_color(
                        egui::Color32::from_rgba_unmultiplied(7, 24, 54, 238),
                        opacity,
                    ))
                    .stroke(egui::Stroke::new(
                        1.0,
                        alpha_color(
                            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 40),
                            opacity,
                        ),
                    ))
                    .rounding(egui::Rounding::same(8.0))
                    .inner_margin(egui::Margin::same(12.0))
                    .show(ui, |ui| {
                        ui.set_width(270.0);
                        ui.label(egui::RichText::new("Picker options").strong());
                        ui.add(
                            egui::Slider::new(&mut self.cfg.general.picker_icon_size, 40.0..=112.0)
                                .text("Icon size"),
                        );
                        ui.add(
                            egui::Slider::new(
                                &mut self.cfg.general.picker_window_opacity,
                                0.0..=1.0,
                            )
                            .text("Window opacity"),
                        );
                        ui.add(
                            egui::Slider::new(&mut self.cfg.general.picker_padding, 0.0..=56.0)
                                .text("Padding"),
                        );
                        ui.checkbox(&mut self.cfg.general.picker_acrylic, "Background blur");
                        ui.horizontal(|ui| {
                            ui.label("Color");
                            if ui
                                .text_edit_singleline(&mut self.options_color_buffer)
                                .changed()
                                && parse_hex_color(&self.options_color_buffer).is_some()
                            {
                                self.cfg.general.picker_background_color =
                                    self.options_color_buffer.clone();
                            }
                        });
                        ui.horizontal_wrapped(|ui| {
                            for (id, label) in [
                                ("bubbles", "Bubbles"),
                                ("solid", "Solid"),
                                ("image", "Image"),
                            ] {
                                if ui
                                    .selectable_label(
                                        self.cfg.general.picker_background == id,
                                        label,
                                    )
                                    .clicked()
                                {
                                    self.cfg.general.picker_background = id.to_string();
                                    self.background_override = Some(id.to_string());
                                }
                            }
                        });
                        ui.horizontal(|ui| {
                            if ui.button("Choose image...").clicked() {
                                if let Some(path) = pick_background_image_file() {
                                    self.cfg.general.picker_background_image = Some(path);
                                    self.cfg.general.picker_background = "image".to_string();
                                    self.background_override = Some("image".to_string());
                                }
                            }
                            if let Some(path) = &self.cfg.general.picker_background_image {
                                ui.label(short_label(path, 24));
                            }
                        });
                        ui.horizontal(|ui| {
                            if ui.button("Default size").clicked() {
                                self.reset_default_size(ctx);
                            }
                            if ui.button("Save size").clicked() {
                                self.save_current_size(panel_rect);
                            }
                            if ui.button("Save").clicked() {
                                self.save_config();
                            }
                        });
                        if ui.button("Open settings").clicked() {
                            self.open_settings();
                        }
                    });
            });
    }

    fn footer(&mut self, ui: &mut egui::Ui, panel_rect: egui::Rect) {
        let opacity = picker_opacity(&self.cfg);
        let pill_width = (panel_rect.width() * 0.54).clamp(300.0, 460.0);
        let pill_rect = egui::Rect::from_center_size(
            egui::pos2(panel_rect.center().x, panel_rect.bottom() - 48.0),
            egui::vec2(pill_width, 48.0),
        );
        let painter = ui.painter_at(pill_rect);
        painter.rect_filled(
            pill_rect,
            egui::Rounding::same(8.0),
            alpha_color(
                egui::Color32::from_rgba_unmultiplied(20, 83, 170, 160),
                opacity,
            ),
        );
        painter.text(
            pill_rect.left_center() + egui::vec2(24.0, 0.0),
            egui::Align2::LEFT_CENTER,
            "URL",
            egui::FontId::proportional(18.0),
            alpha_color(egui::Color32::WHITE, opacity),
        );
        painter.text(
            pill_rect.left_center() + egui::vec2(76.0, 0.0),
            egui::Align2::LEFT_CENTER,
            bottom_label(&self.url),
            egui::FontId::proportional(18.0),
            alpha_color(egui::Color32::WHITE, opacity),
        );

        if self.config_path.is_some() {
            let button_rect = egui::Rect::from_min_size(
                pill_rect.right_center() - egui::vec2(102.0, 15.0),
                egui::vec2(84.0, 30.0),
            );
            let response = ui
                .interact(
                    button_rect,
                    ui.id().with("always-footer"),
                    egui::Sense::click(),
                )
                .on_hover_text("Always open this domain here");
            painter.rect_filled(
                button_rect,
                egui::Rounding::same(6.0),
                alpha_color(
                    egui::Color32::from_rgba_unmultiplied(255, 255, 255, 24),
                    opacity,
                ),
            );
            painter.text(
                button_rect.center(),
                egui::Align2::CENTER_CENTER,
                "Always",
                egui::FontId::proportional(14.0),
                alpha_color(egui::Color32::WHITE, opacity),
            );
            if response.clicked() {
                self.always_open_selected();
            }
        }
    }
}

#[derive(Clone, Copy)]
enum HeaderIcon {
    Menu,
    Settings,
    Close,
}

fn icon_button(ui: &mut egui::Ui, icon: HeaderIcon, tooltip: &str, opacity: f32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(56.0, 40.0), egui::Sense::click());
    let painter = ui.painter_at(rect);
    let fill = if response.hovered() {
        alpha_color(
            egui::Color32::from_rgba_unmultiplied(20, 83, 170, 180),
            opacity,
        )
    } else {
        control_fill(opacity)
    };
    painter.rect_filled(rect, egui::Rounding::same(7.0), fill);
    let stroke = egui::Stroke::new(2.0, alpha_color(egui::Color32::WHITE, opacity));
    let c = rect.center();
    match icon {
        HeaderIcon::Menu => {
            for y in [-6.0, 0.0, 6.0] {
                painter.line_segment([c + egui::vec2(-9.0, y), c + egui::vec2(9.0, y)], stroke);
            }
        }
        HeaderIcon::Settings => {
            painter.circle_stroke(c, 7.0, stroke);
            painter.circle_filled(c, 2.2, alpha_color(egui::Color32::WHITE, opacity));
            for angle in [
                0.0_f32,
                std::f32::consts::FRAC_PI_2,
                std::f32::consts::PI,
                std::f32::consts::PI + std::f32::consts::FRAC_PI_2,
            ] {
                let dir = egui::vec2(angle.cos(), angle.sin());
                painter.line_segment([c + dir * 10.0, c + dir * 13.0], stroke);
            }
        }
        HeaderIcon::Close => {
            painter.line_segment(
                [c + egui::vec2(-7.0, -7.0), c + egui::vec2(7.0, 7.0)],
                stroke,
            );
            painter.line_segment(
                [c + egui::vec2(-7.0, 7.0), c + egui::vec2(7.0, -7.0)],
                stroke,
            );
        }
    }
    response.on_hover_text(tooltip)
}

fn draw_panel_background(
    ui: &egui::Ui,
    rect: egui::Rect,
    cfg: &Config,
    background: &str,
    textures: &mut HashMap<String, egui::TextureHandle>,
) {
    let painter = ui.painter();
    let opacity = picker_opacity(cfg);
    if background == "image" {
        if let Some(path) = &cfg.general.picker_background_image {
            if let Some(texture) = texture_for(ui.ctx(), textures, path, false) {
                let uv = cover_uv(texture.size_vec2(), rect.size());
                painter.image(
                    texture.id(),
                    rect,
                    uv,
                    alpha_color(egui::Color32::WHITE, opacity),
                );
                return;
            }
        }
    }

    if background == "solid" {
        painter.rect_filled(
            rect,
            egui::Rounding::same(10.0),
            alpha_color(
                parse_hex_color(&cfg.general.picker_background_color)
                    .unwrap_or_else(|| egui::Color32::from_rgb(20, 83, 170)),
                opacity,
            ),
        );
        return;
    }

    painter.rect_filled(
        rect,
        egui::Rounding::same(10.0),
        alpha_color(egui::Color32::from_rgb(7, 24, 54), opacity),
    );
    painter.rect_filled(
        rect,
        egui::Rounding::same(10.0),
        alpha_color(
            egui::Color32::from_rgba_unmultiplied(25, 142, 71, 170),
            opacity,
        ),
    );
    painter.circle_filled(
        rect.left_top() + egui::vec2(88.0, 84.0),
        190.0,
        alpha_color(
            egui::Color32::from_rgba_unmultiplied(255, 203, 64, 130),
            opacity,
        ),
    );
    painter.circle_filled(
        rect.right_bottom() - egui::vec2(42.0, 34.0),
        230.0,
        alpha_color(
            egui::Color32::from_rgba_unmultiplied(59, 130, 246, 135),
            opacity,
        ),
    );
}

fn draw_acrylic_overlay(ui: &egui::Ui, rect: egui::Rect, enabled: bool, opacity: f32) {
    if !enabled {
        return;
    }
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        egui::Rounding::same(10.0),
        alpha_color(
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 28),
            opacity,
        ),
    );
}

fn tile_width(cfg: &Config) -> f32 {
    (cfg.general.picker_icon_size.clamp(40.0, 112.0) + 94.0).clamp(138.0, 220.0)
}

fn tile_height(cfg: &Config) -> f32 {
    cfg.general.picker_icon_size.clamp(40.0, 112.0) + 82.0
}

fn columns_for_width(width: f32, tile_width: f32, gap: f32) -> usize {
    ((width + gap) / (tile_width + gap)).floor().max(1.0) as usize
}

fn browser_grid_rect(inner_rect: egui::Rect, panel_rect: egui::Rect, cfg: &Config) -> egui::Rect {
    let padding = cfg.general.picker_padding.clamp(0.0, 56.0);
    let top = inner_rect.top() + 64.0 + padding;
    let bottom = panel_rect.bottom() - 98.0;
    egui::Rect::from_min_max(
        egui::pos2(inner_rect.left(), top),
        egui::pos2(inner_rect.right(), bottom.max(top + 96.0)),
    )
}

fn profile_tile(
    ui: &mut egui::Ui,
    target: &BrowserTarget,
    number: usize,
    selected: bool,
    textures: &mut HashMap<String, egui::TextureHandle>,
    cfg: &Config,
) -> egui::Response {
    let icon_size = cfg.general.picker_icon_size.clamp(40.0, 112.0);
    let size = egui::vec2(tile_width(cfg), tile_height(cfg));
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let painter = ui.painter_at(rect);
    if selected || response.hovered() {
        painter.rect_filled(
            rect,
            egui::Rounding::same(10.0),
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 28),
        );
    }

    let center = rect.center_top() + egui::vec2(0.0, icon_size / 2.0 + 10.0);
    if let Some(texture) = icon_texture(ui.ctx(), textures, target) {
        let icon_rect = egui::Rect::from_center_size(center, egui::vec2(icon_size, icon_size));
        painter.image(
            texture.id(),
            icon_rect,
            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    } else {
        draw_browser_icon(
            &painter,
            center,
            target,
            selected || response.hovered(),
            icon_size / 2.0,
        );
    }

    let badge_center = center + egui::vec2(icon_size * 0.48, -icon_size * 0.38);
    painter.circle_filled(
        badge_center,
        11.0,
        egui::Color32::from_rgba_unmultiplied(20, 35, 70, 132),
    );
    painter.text(
        badge_center,
        egui::Align2::CENTER_CENTER,
        number.to_string(),
        egui::FontId::proportional(12.0),
        egui::Color32::WHITE,
    );

    painter.text(
        rect.center_bottom() - egui::vec2(0.0, 37.0),
        egui::Align2::CENTER_CENTER,
        short_label(&target.name, 18),
        egui::FontId::proportional(17.0),
        egui::Color32::WHITE,
    );
    painter.text(
        rect.center_bottom() - egui::vec2(0.0, 14.0),
        egui::Align2::CENTER_CENTER,
        short_label(&target_subtitle(target), 22),
        egui::FontId::proportional(12.5),
        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 185),
    );

    response
}

fn draw_browser_icon(
    painter: &egui::Painter,
    center: egui::Pos2,
    target: &BrowserTarget,
    active: bool,
    base_radius: f32,
) {
    let radius = if active {
        base_radius + 3.0
    } else {
        base_radius
    };
    let name = target.name.to_ascii_lowercase();
    if name.contains("edge") {
        draw_edge_icon(painter, center, radius);
        return;
    }
    match target.kind.as_str() {
        "chromium" => draw_chromium_icon(painter, center, radius),
        "firefox" => draw_firefox_icon(painter, center, radius),
        _ => draw_generic_icon(painter, center, radius, target),
    }
}

fn draw_edge_icon(painter: &egui::Painter, center: egui::Pos2, r: f32) {
    painter.circle_filled(center, r, egui::Color32::from_rgb(67, 214, 170));
    painter.circle_filled(
        center + egui::vec2(-0.22 * r, 0.14 * r),
        r * 0.78,
        egui::Color32::from_rgb(21, 112, 202),
    );
    painter.circle_filled(
        center + egui::vec2(0.20 * r, 0.18 * r),
        r * 0.60,
        egui::Color32::from_rgb(31, 89, 190),
    );
    painter.circle_filled(
        center + egui::vec2(0.12 * r, -0.02 * r),
        r * 0.42,
        egui::Color32::from_rgb(81, 204, 179),
    );
}

fn draw_chromium_icon(painter: &egui::Painter, center: egui::Pos2, r: f32) {
    painter.circle_filled(center, r, egui::Color32::from_rgb(52, 168, 83));
    painter.circle_filled(
        center + egui::vec2(r * 0.22, -r * 0.23),
        r * 0.76,
        egui::Color32::from_rgb(234, 67, 53),
    );
    painter.circle_filled(
        center + egui::vec2(r * 0.26, r * 0.28),
        r * 0.72,
        egui::Color32::from_rgb(251, 188, 5),
    );
    painter.circle_filled(center, r * 0.48, egui::Color32::WHITE);
    painter.circle_filled(center, r * 0.35, egui::Color32::from_rgb(66, 133, 244));
}

fn draw_firefox_icon(painter: &egui::Painter, center: egui::Pos2, r: f32) {
    painter.circle_filled(center, r, egui::Color32::from_rgb(124, 72, 245));
    painter.circle_filled(
        center + egui::vec2(-r * 0.14, -r * 0.16),
        r * 0.72,
        egui::Color32::from_rgb(51, 211, 238),
    );
    painter.circle_filled(
        center + egui::vec2(0.16 * r, 0.0),
        r * 0.68,
        egui::Color32::from_rgb(97, 58, 230),
    );
    painter.circle_filled(
        center + egui::vec2(-0.25 * r, -0.36 * r),
        r * 0.26,
        egui::Color32::from_rgb(96, 255, 190),
    );
}

fn draw_generic_icon(painter: &egui::Painter, center: egui::Pos2, r: f32, target: &BrowserTarget) {
    painter.circle_filled(center, r, target_color(&target.id));
    painter.text(
        center,
        egui::Align2::CENTER_CENTER,
        initials(&target.name),
        egui::FontId::proportional(24.0),
        egui::Color32::WHITE,
    );
}

fn target_subtitle(target: &BrowserTarget) -> String {
    if let Some(profile) = &target.profile_dir {
        format!("Profile: {profile}")
    } else if let Some(profile) = &target.profile_name {
        format!("Profile: {profile}")
    } else {
        match target.kind.as_str() {
            "chromium" => "Chromium browser".to_string(),
            "firefox" => "Firefox browser".to_string(),
            kind => format!("{kind} target"),
        }
    }
}

fn icon_texture(
    ctx: &egui::Context,
    textures: &mut HashMap<String, egui::TextureHandle>,
    target: &BrowserTarget,
) -> Option<egui::TextureHandle> {
    let source = target
        .icon
        .as_deref()
        .filter(|icon| !icon.eq_ignore_ascii_case("auto"))
        .filter(|icon| !icon.trim().is_empty())
        .unwrap_or(&target.executable);
    if source.eq_ignore_ascii_case("auto") || source.trim().is_empty() {
        return None;
    }
    texture_for(ctx, textures, source, true)
}

fn texture_for(
    ctx: &egui::Context,
    textures: &mut HashMap<String, egui::TextureHandle>,
    source: &str,
    allow_associated_icon: bool,
) -> Option<egui::TextureHandle> {
    let path = normalize_icon_source(source);
    let key = format!("{allow_associated_icon}:{path}");
    if let Some(texture) = textures.get(&key) {
        return Some(texture.clone());
    }

    let image = load_raster_image(&path).or_else(|| {
        if allow_associated_icon {
            load_associated_icon(Path::new(&path))
        } else {
            None
        }
    })?;
    let texture = ctx.load_texture(key.clone(), image, egui::TextureOptions::LINEAR);
    textures.insert(key, texture.clone());
    Some(texture)
}

fn normalize_icon_source(source: &str) -> String {
    let mut value = source.trim().trim_matches('"').to_string();
    if value.starts_with('%') {
        if let Some(end) = value[1..].find('%') {
            let name = &value[1..=end];
            if let Ok(replacement) = std::env::var(name) {
                value = format!("{replacement}{}", &value[end + 2..]);
            }
        }
    }
    if let Some((path, _index)) = value.rsplit_once(',') {
        let trimmed = path.trim().trim_matches('"');
        if !trimmed.is_empty() {
            value = trimmed.to_string();
        }
    }
    value
}

fn load_raster_image(path: &str) -> Option<egui::ColorImage> {
    let ext = Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !matches!(ext.as_str(), "ico" | "jpg" | "jpeg" | "png") {
        return None;
    }

    let bytes = std::fs::read(path).ok()?;
    let image = image::load_from_memory(&bytes).ok()?.to_rgba8();
    let size = [image.width() as usize, image.height() as usize];
    Some(egui::ColorImage::from_rgba_unmultiplied(
        size,
        image.as_raw(),
    ))
}

#[cfg(windows)]
fn load_associated_icon(path: &Path) -> Option<egui::ColorImage> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr::null_mut;
    use windows_sys::Win32::Graphics::Gdi::{
        CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC,
        SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HGDIOBJ, RGBQUAD,
    };
    use windows_sys::Win32::UI::Shell::{SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON};
    use windows_sys::Win32::UI::WindowsAndMessaging::{DestroyIcon, DrawIconEx, DI_NORMAL};

    let mut wide = OsStr::new(path).encode_wide().collect::<Vec<_>>();
    wide.push(0);

    let mut info: SHFILEINFOW = unsafe { std::mem::zeroed() };
    let result = unsafe {
        SHGetFileInfoW(
            wide.as_ptr(),
            0,
            &mut info,
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_LARGEICON,
        )
    };
    if result == 0 || info.hIcon == 0 {
        return None;
    }

    let size = 64i32;
    let screen_dc = unsafe { GetDC(0) };
    if screen_dc == 0 {
        unsafe {
            DestroyIcon(info.hIcon);
        }
        return None;
    }
    let mem_dc = unsafe { CreateCompatibleDC(screen_dc) };
    if mem_dc == 0 {
        unsafe {
            ReleaseDC(0, screen_dc);
            DestroyIcon(info.hIcon);
        }
        return None;
    }

    let mut bits = null_mut();
    let bitmap_info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: size,
            biHeight: -size,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        },
        bmiColors: [RGBQUAD {
            rgbBlue: 0,
            rgbGreen: 0,
            rgbRed: 0,
            rgbReserved: 0,
        }],
    };
    let bitmap = unsafe { CreateDIBSection(mem_dc, &bitmap_info, DIB_RGB_COLORS, &mut bits, 0, 0) };
    if bitmap == 0 || bits.is_null() {
        unsafe {
            DeleteDC(mem_dc);
            ReleaseDC(0, screen_dc);
            DestroyIcon(info.hIcon);
        }
        return None;
    }

    let old = unsafe { SelectObject(mem_dc, bitmap as HGDIOBJ) };
    let drawn = unsafe { DrawIconEx(mem_dc, 0, 0, info.hIcon, size, size, 0, 0, DI_NORMAL) };
    let raw = if drawn != 0 {
        let bytes = unsafe {
            std::slice::from_raw_parts(bits.cast::<u8>(), (size * size * 4) as usize).to_vec()
        };
        let mut rgba = Vec::with_capacity(bytes.len());
        for pixel in bytes.chunks_exact(4) {
            rgba.extend_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
        }
        if rgba.chunks_exact(4).all(|pixel| pixel[3] == 0) {
            for pixel in rgba.chunks_exact_mut(4) {
                if pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0 {
                    pixel[3] = 255;
                }
            }
        }
        Some(rgba)
    } else {
        None
    };

    unsafe {
        if old != 0 {
            SelectObject(mem_dc, old);
        }
        DeleteObject(bitmap as HGDIOBJ);
        DeleteDC(mem_dc);
        ReleaseDC(0, screen_dc);
        DestroyIcon(info.hIcon);
    }

    raw.map(|rgba| egui::ColorImage::from_rgba_unmultiplied([size as usize, size as usize], &rgba))
}

#[cfg(not(windows))]
fn load_associated_icon(_path: &Path) -> Option<egui::ColorImage> {
    None
}

fn cover_uv(image_size: egui::Vec2, target_size: egui::Vec2) -> egui::Rect {
    let image_ratio = image_size.x / image_size.y;
    let target_ratio = target_size.x / target_size.y;
    if image_ratio > target_ratio {
        let visible = target_ratio / image_ratio;
        let inset = (1.0 - visible) / 2.0;
        egui::Rect::from_min_max(egui::pos2(inset, 0.0), egui::pos2(1.0 - inset, 1.0))
    } else {
        let visible = image_ratio / target_ratio;
        let inset = (1.0 - visible) / 2.0;
        egui::Rect::from_min_max(egui::pos2(0.0, inset), egui::pos2(1.0, 1.0 - inset))
    }
}

fn parse_hex_color(value: &str) -> Option<egui::Color32> {
    let hex = value.trim().trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(egui::Color32::from_rgb(r, g, b))
}

fn initials(name: &str) -> String {
    let mut chars = name
        .split(|c: char| c.is_whitespace() || matches!(c, '(' | ')' | '-' | '_'))
        .filter_map(|part| part.chars().next())
        .take(2)
        .collect::<String>();
    if chars.is_empty() {
        chars.push('B');
    }
    chars.to_uppercase()
}

fn header_title(url: &str, fallback: &str) -> String {
    if url.trim().is_empty() {
        fallback.to_string()
    } else {
        "Select a Browser".to_string()
    }
}

fn bottom_label(url: &str) -> String {
    if url.trim().is_empty() {
        "No Url Opened".to_string()
    } else {
        short_label(url, 22)
    }
}

fn short_label(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_string();
    }
    let keep = max.saturating_sub(1);
    format!("{}...", value.chars().take(keep).collect::<String>())
}

fn target_color(id: &str) -> egui::Color32 {
    let mut hash = 0u32;
    for byte in id.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(byte as u32);
    }
    let colors = [
        egui::Color32::from_rgb(46, 144, 250),
        egui::Color32::from_rgb(22, 163, 74),
        egui::Color32::from_rgb(232, 93, 52),
        egui::Color32::from_rgb(217, 119, 6),
        egui::Color32::from_rgb(124, 58, 237),
        egui::Color32::from_rgb(8, 145, 178),
    ];
    colors[(hash as usize) % colors.len()]
}

fn picker_opacity(cfg: &Config) -> f32 {
    cfg.general.picker_window_opacity.clamp(0.0, 1.0)
}

fn alpha_color(color: egui::Color32, opacity: f32) -> egui::Color32 {
    let [r, g, b, a] = color.to_array();
    egui::Color32::from_rgba_unmultiplied(r, g, b, ((a as f32) * opacity).round() as u8)
}

fn control_fill(opacity: f32) -> egui::Color32 {
    alpha_color(
        egui::Color32::from_rgba_unmultiplied(20, 83, 170, 120),
        opacity,
    )
}

fn success_color() -> egui::Color32 {
    egui::Color32::from_rgb(178, 255, 204)
}
