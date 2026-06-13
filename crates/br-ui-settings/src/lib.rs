//! Settings application for `br` (PRD §9.1 onboarding, §9.3 settings UI).
//!
//! Lets the user manage browsers/profiles, filters, and rules, and walks
//! first-time users through an onboarding flow that discovers installed
//! browsers and registers `br` as the default handler.

use anyhow::{Context, Result};
use br_core::i18n::{tr, Key};
use br_core::model::DefaultActionSerde;
use br_core::{BrowserTarget, Config, DefaultAction, Filter, Rule};
use br_platform::PlatformIntegration;
use eframe::egui;
use std::cmp::Reverse;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

static DARK_THEME: AtomicBool = AtomicBool::new(false);
const LOGO_BYTES: &[u8] = include_bytes!("../../../docs/logo_icon_transparent.png");
const APP_NAME: &str = "BrowserRouter (br)";
const APP_CREATOR: &str = "Juliano Marcon";
const APP_REPOSITORY: &str = "https://github.com/jmarcon/br";

/// Resolves the default config file path (`<config-dir>/br/config.toml`).
pub fn default_config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("br")
        .join("config.toml")
}

/// Loads the config at `path`, falling back to the default (and noting
/// whether onboarding should run because no config file exists yet).
fn load(path: &Path) -> (Config, bool) {
    if !path.exists() {
        return (Config::default(), true);
    }
    let (config, _err) = br_core::config::load_or_default(path);
    (config, false)
}

/// Writes `config` to `path` as TOML, creating parent directories as needed.
fn save(path: &Path, config: &Config) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context("creating config directory")?;
    }
    let contents = toml::to_string_pretty(config).context("serializing config")?;
    std::fs::write(path, contents).context("writing config file")?;
    Ok(())
}

fn pick_image_file() -> Option<String> {
    rfd::FileDialog::new()
        .add_filter("Images", &["png", "jpg", "jpeg", "ico"])
        .pick_file()
        .map(|path| path.display().to_string())
}

fn pick_icon_file() -> Option<String> {
    rfd::FileDialog::new()
        .add_filter(
            "Icons and apps",
            &["ico", "png", "jpg", "jpeg", "exe", "lnk"],
        )
        .pick_file()
        .map(|path| path.display().to_string())
}

fn pick_executable_file() -> Option<String> {
    rfd::FileDialog::new()
        .add_filter("Applications", &["exe", "bat", "cmd", "lnk"])
        .pick_file()
        .map(|path| path.display().to_string())
}

fn pick_directory() -> Option<String> {
    rfd::FileDialog::new()
        .pick_folder()
        .map(|path| path.display().to_string())
}

fn load_logo_rgba() -> Option<(Vec<u8>, u32, u32)> {
    let image = image::load_from_memory(LOGO_BYTES).ok()?.to_rgba8();
    let (width, height) = image.dimensions();
    Some((image.into_raw(), width, height))
}

fn app_icon_data() -> Option<egui::IconData> {
    let (rgba, width, height) = load_logo_rgba()?;
    Some(egui::IconData {
        rgba,
        width,
        height,
    })
}

fn logo_texture(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    let (rgba, width, height) = load_logo_rgba()?;
    let image = egui::ColorImage::from_rgba_unmultiplied([width as usize, height as usize], &rgba);
    Some(ctx.load_texture("br-logo", image, egui::TextureOptions::LINEAR))
}

fn t(language: &str, text: &'static str) -> &'static str {
    if !(language.eq_ignore_ascii_case("pt-BR") || language.eq_ignore_ascii_case("pt")) {
        return text;
    }
    match text {
        "Actions" => "Ações",
        "About" => "Sobre",
        "Add browser" => "Adicionar navegador",
        "Add filter" => "Adicionar filtro",
        "Add rule" => "Adicionar regra",
        "App" => "Aplicativo",
        "Args" => "Argumentos",
        "Ask" => "Perguntar",
        "Background" => "Fundo",
        "Background blur" => "Desfoque de fundo",
        "Background color" => "Cor de fundo",
        "Background image" => "Imagem de fundo",
        "Block" => "Bloquear",
        "Browse..." => "Procurar...",
        "Browser" => "Navegador",
        "Browser target" => "Destino de navegador",
        "Browsers" => "Navegadores",
        "Bubbles" => "Bolhas",
        "Center" => "Centro",
        "Creator" => "Criador",
        "Cursor" => "Cursor",
        "Dark" => "Escuro",
        "Default action" => "Ação padrão",
        "Default handler" => "Manipulador padrão",
        "Discover installed browsers" => "Detectar navegadores instalados",
        "Enabled" => "Ativado",
        "English" => "Inglês",
        "Exceptions" => "Exceções",
        "Executable" => "Executável",
        "Filter" => "Filtro",
        "Rule" => "Regra",
        "Hidden" => "Oculto",
        "Hide from picker" => "Ocultar no seletor",
        "Icon" => "Ícone",
        "Icon size" => "Tamanho do ícone",
        "Id" => "Id",
        "Image" => "Imagem",
        "Kind" => "Tipo",
        "Language" => "Idioma",
        "Light" => "Claro",
        "Link and browser profile router" => "Roteador de links e perfis de navegador",
        "Log level" => "Nível de log",
        "Mode" => "Modo",
        "Modifier keys" => "Teclas modificadoras",
        "Name" => "Nome",
        "New browser" => "Novo navegador",
        "New rule" => "Nova regra",
        "Open with" => "Abrir com",
        "Open with all" => "Abrir com todos",
        "Move down" => "Mover para baixo",
        "Move up" => "Mover para cima",
        "Padding" => "Espaçamento",
        "Picker" => "Seletor",
        "Picker size" => "Tamanho do seletor",
        "Position" => "Posição",
        "Portuguese (Brazil)" => "Português (Brasil)",
        "Priority" => "Prioridade",
        "Private" => "Privado",
        "Profile directory" => "Diretório do perfil",
        "Profile name" => "Nome do perfil",
        "Remove" => "Remover",
        "Repository" => "Repositório",
        "Routing rule" => "Regra de roteamento",
        "Set br as default browser" => "Definir br como navegador padrão",
        "Solid" => "Sólido",
        "Source apps" => "Apps de origem",
        "Strip query params" => "Remover parâmetros de consulta",
        "System" => "Sistema",
        "Theme" => "Tema",
        "Timeout" => "Tempo limite",
        "Upgrade HTTPS" => "Atualizar para HTTPS",
        "URL cleanup" => "Limpeza de URL",
        "URL patterns" => "Padrões de URL",
        "Version" => "Versão",
        "Window opacity" => "Opacidade da janela",
        "br is not the default browser handler." => "br não é o manipulador de navegador padrão.",
        "br is the default browser handler." => "br é o manipulador de navegador padrão.",
        "px high" => " px de altura",
        "px wide" => " px de largura",
        _ => text,
    }
}

/// Opens the settings window, optionally starting at a specific config path.
pub fn run(config_path: Option<PathBuf>) -> Result<()> {
    let path = config_path.unwrap_or_else(default_config_path);
    let (config, needs_onboarding) = load(&path);

    let options = eframe::NativeOptions {
        viewport: {
            let mut viewport = egui::ViewportBuilder::default()
                .with_inner_size([920.0, 680.0])
                .with_min_inner_size([760.0, 560.0])
                .with_title(tr(Key::SettingsTitle, &config.general.language));
            if let Some(icon) = app_icon_data() {
                viewport = viewport.with_icon(icon);
            }
            viewport
        },
        ..Default::default()
    };

    eframe::run_native(
        "br-settings",
        options,
        Box::new(move |_cc| Ok(Box::new(SettingsApp::new(path, config, needs_onboarding)))),
    )
    .map_err(|e| anyhow::anyhow!("failed to run settings UI: {e}"))
}

fn apply_material_style(ctx: &egui::Context, theme: &str) {
    let theme = theme.to_ascii_lowercase();
    let dark = match theme.as_str() {
        "dark" => true,
        "light" => false,
        _ => matches!(ctx.system_theme(), Some(egui::Theme::Dark)),
    };
    let window_theme = match theme.as_str() {
        "dark" => egui::SystemTheme::Dark,
        "light" => egui::SystemTheme::Light,
        _ => egui::SystemTheme::SystemDefault,
    };
    ctx.send_viewport_cmd(egui::ViewportCommand::SetTheme(window_theme));
    DARK_THEME.store(dark, Ordering::Relaxed);

    let mut visuals = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    visuals.panel_fill = material_bg();
    visuals.window_fill = material_surface();
    visuals.widgets.noninteractive.bg_fill = material_surface();
    visuals.widgets.inactive.bg_fill = material_surface_container();
    visuals.widgets.hovered.bg_fill = material_hover();
    visuals.widgets.active.bg_fill = material_active();
    visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(8);
    visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(8);
    visuals.widgets.active.corner_radius = egui::CornerRadius::same(8);
    visuals.selection.bg_fill = material_primary();
    visuals.selection.stroke.color = egui::Color32::WHITE;
    ctx.set_visuals(visuals);
    ctx.global_style_mut(|style| {
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(12.0, 6.0);
    });
}

fn is_dark_theme() -> bool {
    DARK_THEME.load(Ordering::Relaxed)
}

fn material_bg() -> egui::Color32 {
    if is_dark_theme() {
        egui::Color32::from_rgb(5, 17, 39)
    } else {
        egui::Color32::from_rgb(244, 248, 252)
    }
}

fn material_surface() -> egui::Color32 {
    if is_dark_theme() {
        egui::Color32::from_rgb(7, 24, 54)
    } else {
        egui::Color32::from_rgb(250, 252, 255)
    }
}

fn material_surface_container() -> egui::Color32 {
    if is_dark_theme() {
        egui::Color32::from_rgb(16, 42, 82)
    } else {
        egui::Color32::from_rgb(229, 239, 250)
    }
}

fn material_surface_container_low() -> egui::Color32 {
    if is_dark_theme() {
        egui::Color32::from_rgb(11, 30, 62)
    } else {
        egui::Color32::from_rgb(238, 245, 252)
    }
}

fn material_outline() -> egui::Color32 {
    if is_dark_theme() {
        egui::Color32::from_rgb(57, 91, 135)
    } else {
        egui::Color32::from_rgb(176, 194, 218)
    }
}

fn material_primary() -> egui::Color32 {
    if is_dark_theme() {
        egui::Color32::from_rgb(255, 203, 64)
    } else {
        egui::Color32::from_rgb(20, 83, 170)
    }
}

fn material_on_surface() -> egui::Color32 {
    if is_dark_theme() {
        egui::Color32::from_rgb(238, 244, 250)
    } else {
        egui::Color32::from_rgb(5, 17, 39)
    }
}

fn material_on_surface_variant() -> egui::Color32 {
    if is_dark_theme() {
        egui::Color32::from_rgb(190, 207, 229)
    } else {
        egui::Color32::from_rgb(47, 69, 98)
    }
}

fn material_error() -> egui::Color32 {
    if is_dark_theme() {
        egui::Color32::from_rgb(242, 184, 181)
    } else {
        egui::Color32::from_rgb(179, 38, 30)
    }
}

fn material_hover() -> egui::Color32 {
    if is_dark_theme() {
        egui::Color32::from_rgb(20, 56, 106)
    } else {
        egui::Color32::from_rgb(219, 234, 250)
    }
}

fn material_active() -> egui::Color32 {
    if is_dark_theme() {
        egui::Color32::from_rgb(32, 86, 156)
    } else {
        egui::Color32::from_rgb(194, 221, 250)
    }
}

fn material_tonal() -> egui::Color32 {
    if is_dark_theme() {
        egui::Color32::from_rgb(28, 76, 139)
    } else {
        egui::Color32::from_rgb(219, 234, 250)
    }
}

fn material_on_primary() -> egui::Color32 {
    if is_dark_theme() {
        egui::Color32::from_rgb(5, 17, 39)
    } else {
        egui::Color32::WHITE
    }
}

fn filled_button(label: impl Into<String>) -> egui::Button<'static> {
    egui::Button::new(egui::RichText::new(label.into()).color(material_on_primary()))
        .fill(material_primary())
        .corner_radius(egui::CornerRadius::same(20))
        .min_size(egui::vec2(76.0, 34.0))
}

fn tonal_button(label: impl Into<String>) -> egui::Button<'static> {
    egui::Button::new(egui::RichText::new(label.into()).color(material_on_surface()))
        .fill(material_tonal())
        .corner_radius(egui::CornerRadius::same(20))
        .min_size(egui::vec2(76.0, 34.0))
}

fn danger_button(label: impl Into<String>) -> egui::Button<'static> {
    egui::Button::new(egui::RichText::new(label.into()).color(material_error()))
        .fill(egui::Color32::TRANSPARENT)
        .corner_radius(egui::CornerRadius::same(20))
        .min_size(egui::vec2(64.0, 34.0))
}

fn material_section(ui: &mut egui::Ui, title: &str, add: impl FnOnce(&mut egui::Ui)) {
    let width = ui.available_width().max(240.0);
    egui::Frame::new()
        .fill(material_surface())
        .stroke(egui::Stroke::new(1.0, material_outline()))
        .corner_radius(egui::CornerRadius::same(8))
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.set_width((width - 20.0).max(220.0));
            ui.label(
                egui::RichText::new(title)
                    .size(16.0)
                    .strong()
                    .color(material_on_surface()),
            );
            ui.add_space(6.0);
            add(ui);
        });
    ui.add_space(8.0);
}

fn field_row(ui: &mut egui::Ui, label: &str, add: impl FnOnce(&mut egui::Ui)) {
    let narrow = ui.available_width() < 560.0;
    if narrow {
        ui.vertical(|ui| {
            ui.label(egui::RichText::new(label).color(material_on_surface_variant()));
            add(ui);
        });
        return;
    }

    ui.horizontal(|ui| {
        ui.set_min_height(36.0);
        ui.add_sized(
            [136.0, 22.0],
            egui::Label::new(egui::RichText::new(label).color(material_on_surface_variant())),
        );
        ui.vertical(|ui| {
            ui.set_width(ui.available_width().max(120.0));
            add(ui);
        });
    });
}

fn text_field(ui: &mut egui::Ui, value: &mut String) {
    ui.add_sized(
        [ui.available_width(), 34.0],
        egui::TextEdit::singleline(value).desired_width(f32::INFINITY),
    );
}

fn optional_text_field(ui: &mut egui::Ui, value: &mut Option<String>) {
    let mut buffer = value.clone().unwrap_or_default();
    if ui
        .add_sized(
            [ui.available_width(), 34.0],
            egui::TextEdit::singleline(&mut buffer).desired_width(f32::INFINITY),
        )
        .changed()
    {
        *value = (!buffer.trim().is_empty()).then_some(buffer);
    }
}

fn optional_file_field(
    ui: &mut egui::Ui,
    value: &mut Option<String>,
    browse_label: &'static str,
    pick: impl FnOnce() -> Option<String>,
) {
    let button_width = 96.0;
    let spacing = ui.spacing().item_spacing.x;
    let narrow = ui.available_width() < 340.0;
    if narrow {
        optional_text_field(ui, value);
        if ui.add(tonal_button(browse_label)).clicked() {
            if let Some(path) = pick() {
                *value = Some(path);
            }
        }
        return;
    }

    ui.horizontal(|ui| {
        let text_width = (ui.available_width() - button_width - spacing).max(120.0);
        let mut buffer = value.clone().unwrap_or_default();
        if ui
            .add_sized(
                [text_width, 34.0],
                egui::TextEdit::singleline(&mut buffer).desired_width(text_width),
            )
            .changed()
        {
            *value = (!buffer.trim().is_empty()).then_some(buffer);
        }
        if ui
            .add_sized([button_width, 34.0], tonal_button(browse_label))
            .clicked()
        {
            if let Some(path) = pick() {
                *value = Some(path);
            }
        }
    });
}

fn file_field(
    ui: &mut egui::Ui,
    value: &mut String,
    browse_label: &'static str,
    pick: impl FnOnce() -> Option<String>,
) {
    let button_width = 96.0;
    let spacing = ui.spacing().item_spacing.x;
    let narrow = ui.available_width() < 340.0;
    if narrow {
        text_field(ui, value);
        if ui.add(tonal_button(browse_label)).clicked() {
            if let Some(path) = pick() {
                *value = path;
            }
        }
        return;
    }

    ui.horizontal(|ui| {
        let text_width = (ui.available_width() - button_width - spacing).max(120.0);
        ui.add_sized(
            [text_width, 34.0],
            egui::TextEdit::singleline(value).desired_width(text_width),
        );
        if ui
            .add_sized([button_width, 34.0], tonal_button(browse_label))
            .clicked()
        {
            if let Some(path) = pick() {
                *value = path;
            }
        }
    });
}

fn csv_field(ui: &mut egui::Ui, value: &mut Vec<String>) {
    let mut buffer = value.join(", ");
    if ui
        .add_sized(
            [ui.available_width(), 34.0],
            egui::TextEdit::singleline(&mut buffer).desired_width(f32::INFINITY),
        )
        .changed()
    {
        *value = split_csv(&buffer);
    }
}

fn browser_target_label(id: &str, browsers: &[BrowserTarget]) -> String {
    browsers
        .iter()
        .find(|browser| browser.id == id)
        .map(|browser| format!("{} ({})", browser.name, browser.id))
        .unwrap_or_else(|| id.to_string())
}

fn browser_target_combo(
    ui: &mut egui::Ui,
    id_salt: &str,
    value: &mut Option<String>,
    browsers: &[BrowserTarget],
    blank_label: &str,
) {
    let selected = value
        .as_deref()
        .map(|id| browser_target_label(id, browsers))
        .unwrap_or_else(|| blank_label.to_string());
    egui::ComboBox::from_id_salt(id_salt)
        .selected_text(selected)
        .width(ui.available_width())
        .show_ui(ui, |ui| {
            ui.selectable_value(value, None, blank_label);
            for browser in browsers {
                ui.selectable_value(
                    value,
                    Some(browser.id.clone()),
                    format!("{} ({})", browser.name, browser.id),
                );
            }
        });
}

fn nav_button(ui: &mut egui::Ui, selected: bool, label: &str) -> egui::Response {
    let fill = if selected {
        material_tonal()
    } else {
        egui::Color32::TRANSPARENT
    };
    ui.add_sized(
        [ui.available_width(), 40.0],
        egui::Button::new(egui::RichText::new(label).color(material_on_surface()))
            .fill(fill)
            .corner_radius(egui::CornerRadius::same(24)),
    )
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum Tab {
    General,
    Browsers,
    Filters,
    Rules,
    About,
}

struct SettingsApp {
    path: PathBuf,
    config: Config,
    tab: Tab,
    onboarding: bool,
    status: Option<String>,
}

impl SettingsApp {
    fn new(path: PathBuf, mut config: Config, onboarding: bool) -> Self {
        if let Ok(enabled) = br_platform::current().is_autostart_enabled() {
            config.general.start_on_login = enabled;
        }
        Self {
            path,
            config,
            tab: Tab::General,
            onboarding,
            status: None,
        }
    }

    fn save(&mut self) {
        match save(&self.path, &self.config) {
            Ok(()) => {
                let lang = &self.config.general.language;
                self.status = Some(
                    if lang.eq_ignore_ascii_case("pt-BR") || lang.eq_ignore_ascii_case("pt") {
                        format!("Salvo em {}", self.path.display())
                    } else {
                        format!("Saved to {}", self.path.display())
                    },
                );
            }
            Err(err) => {
                let lang = &self.config.general.language;
                self.status = Some(
                    if lang.eq_ignore_ascii_case("pt-BR") || lang.eq_ignore_ascii_case("pt") {
                        format!("Falha ao salvar: {err}")
                    } else {
                        format!("Failed to save: {err}")
                    },
                );
            }
        }
    }

    fn discover_browsers(&mut self) {
        let platform = br_platform::current();
        match platform.discover_browsers() {
            Ok(found) => {
                let existing: std::collections::HashSet<String> =
                    self.config.browsers.iter().map(|b| b.id.clone()).collect();
                let mut added = 0;
                for target in found {
                    if !existing.contains(&target.id) {
                        self.config.browsers.push(target);
                        added += 1;
                    }
                }
                let lang = &self.config.general.language;
                self.status = Some(
                    if lang.eq_ignore_ascii_case("pt-BR") || lang.eq_ignore_ascii_case("pt") {
                        format!("{added} novo(s) navegador(es) detectado(s).")
                    } else {
                        format!("Discovered {added} new browser(s).")
                    },
                );
            }
            Err(err) => {
                let lang = &self.config.general.language;
                self.status = Some(
                    if lang.eq_ignore_ascii_case("pt-BR") || lang.eq_ignore_ascii_case("pt") {
                        format!("Falha na detecção: {err}")
                    } else {
                        format!("Discovery failed: {err}")
                    },
                );
            }
        }
    }

    fn register_default_handler(&mut self) {
        let platform = br_platform::current();
        match platform.register_as_default_handler() {
            Ok(br_platform::RegisterOutcome::Registered) => {
                let lang = &self.config.general.language;
                self.status = Some(
                    if lang.eq_ignore_ascii_case("pt-BR") || lang.eq_ignore_ascii_case("pt") {
                        "br agora é o navegador padrão.".to_string()
                    } else {
                        "br is now the default browser.".to_string()
                    },
                );
            }
            Ok(br_platform::RegisterOutcome::NeedsManualConfirmation { instructions }) => {
                self.status = Some(instructions);
            }
            Err(err) => {
                let lang = &self.config.general.language;
                self.status = Some(
                    if lang.eq_ignore_ascii_case("pt-BR") || lang.eq_ignore_ascii_case("pt") {
                        format!("Falha no registro: {err}")
                    } else {
                        format!("Registration failed: {err}")
                    },
                );
            }
        }
    }

    fn ensure_fallback_rule(&mut self) {
        if self.config.rules.is_empty() {
            self.config.rules.push(Rule {
                id: "fallback".to_string(),
                name: "Default: ask".to_string(),
                enabled: true,
                priority: 0,
                match_: br_core::MatchCondition {
                    url_pattern: vec!["*".to_string()],
                    ..Default::default()
                },
                action: br_core::Action {
                    ask: true,
                    ..Default::default()
                },
            });
        }
        if self.config.config_version == 0 {
            self.config.config_version = br_core::config::CURRENT_CONFIG_VERSION;
        }
    }

    fn show_onboarding(&mut self, ui: &mut egui::Ui) {
        let lang = self.config.general.language.clone();
        apply_material_style(ui.ctx(), &self.config.general.theme);
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.add_space(28.0);
            ui.vertical_centered(|ui| {
                ui.set_max_width(560.0);
                material_section(ui, tr(Key::OnboardingWelcomeTitle, &lang), |ui| {
                    ui.label(tr(Key::OnboardingWelcomeBody, &lang));
                    ui.add_space(16.0);

                    ui.label(t(&lang, "Discover installed browsers"));
                    ui.horizontal(|ui| {
                        if ui
                            .add(tonal_button(tr(Key::DiscoverBrowsers, &lang)))
                            .clicked()
                        {
                            self.discover_browsers();
                        }
                        ui.label(format!("{} configured", self.config.browsers.len()));
                    });
                    ui.add_space(8.0);

                    ui.label(t(&lang, "Set br as default browser"));
                    if ui
                        .add(tonal_button(tr(Key::RegisterDefaultBrowser, &lang)))
                        .clicked()
                    {
                        self.register_default_handler();
                    }
                    ui.add_space(16.0);

                    if ui
                        .add(filled_button(tr(Key::OnboardingFinish, &lang)))
                        .clicked()
                    {
                        self.ensure_fallback_rule();
                        self.save();
                        self.onboarding = false;
                    }
                });

                if let Some(status) = &self.status {
                    ui.colored_label(material_primary(), status);
                }
            });
        });
    }
}

impl eframe::App for SettingsApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        apply_material_style(ui.ctx(), &self.config.general.theme);
        if self.onboarding {
            self.show_onboarding(ui);
            return;
        }

        let lang = self.config.general.language.clone();
        egui::Panel::top("app-bar")
            .exact_size(52.0)
            .frame(
                egui::Frame::new()
                    .fill(material_surface())
                    .inner_margin(egui::Margin::symmetric(16, 6)),
            )
            .show_inside(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.label(
                        egui::RichText::new(tr(Key::SettingsTitle, &lang))
                            .size(22.0)
                            .strong()
                            .color(material_on_surface()),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(filled_button(tr(Key::Save, &lang))).clicked() {
                            self.save();
                        }
                    });
                });
            });

        egui::Panel::left("settings-nav")
            .exact_size(152.0)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(material_surface_container_low())
                    .inner_margin(egui::Margin::same(8)),
            )
            .show_inside(ui, |ui| {
                if nav_button(ui, self.tab == Tab::General, tr(Key::TabGeneral, &lang)).clicked() {
                    self.tab = Tab::General;
                }
                if nav_button(ui, self.tab == Tab::Browsers, tr(Key::TabBrowsers, &lang)).clicked()
                {
                    self.tab = Tab::Browsers;
                }
                if nav_button(ui, self.tab == Tab::Filters, tr(Key::TabFilters, &lang)).clicked() {
                    self.tab = Tab::Filters;
                }
                if nav_button(ui, self.tab == Tab::Rules, tr(Key::TabRules, &lang)).clicked() {
                    self.tab = Tab::Rules;
                }
                if nav_button(ui, self.tab == Tab::About, t(&lang, "About")).clicked() {
                    self.tab = Tab::About;
                }
            });

        if let Some(status) = self.status.as_deref() {
            egui::Panel::bottom("status")
                .frame(
                    egui::Frame::new()
                        .fill(material_surface_container())
                        .inner_margin(egui::Margin::symmetric(16, 6)),
                )
                .show_inside(ui, |ui| {
                    ui.label(status);
                });
        }

        egui::CentralPanel::default().show_inside(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        ui.set_width((ui.available_width() - 8.0).max(280.0));
                        match self.tab {
                            Tab::General => self.show_general(ui),
                            Tab::Browsers => self.show_browsers(ui),
                            Tab::Filters => self.show_filters(ui),
                            Tab::Rules => self.show_rules(ui),
                            Tab::About => self.show_about(ui),
                        }
                    });
                });
            });
        });
    }
}

impl SettingsApp {
    fn show_general(&mut self, ui: &mut egui::Ui) {
        let lang = self.config.general.language.clone();
        let browsers = self.config.browsers.clone();
        material_section(ui, t(&lang, "App"), |ui| {
            let general = &mut self.config.general;
            field_row(ui, t(&lang, "Theme"), |ui| {
                egui::ComboBox::from_id_salt("theme")
                    .selected_text(&general.theme)
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut general.theme,
                            "system".to_string(),
                            t(&lang, "System"),
                        );
                        ui.selectable_value(
                            &mut general.theme,
                            "light".to_string(),
                            t(&lang, "Light"),
                        );
                        ui.selectable_value(
                            &mut general.theme,
                            "dark".to_string(),
                            t(&lang, "Dark"),
                        );
                    });
            });
            field_row(ui, t(&lang, "Language"), |ui| {
                egui::ComboBox::from_id_salt("language")
                    .selected_text(&general.language)
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut general.language,
                            "en".to_string(),
                            t(&lang, "English"),
                        );
                        ui.selectable_value(
                            &mut general.language,
                            "pt-BR".to_string(),
                            t(&lang, "Portuguese (Brazil)"),
                        );
                    });
            });
            field_row(ui, t(&lang, "Log level"), |ui| {
                egui::ComboBox::from_id_salt("log-level")
                    .selected_text(&general.log_level)
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        for level in ["error", "warn", "info", "debug", "trace"] {
                            ui.selectable_value(&mut general.log_level, level.to_string(), level);
                        }
                    });
            });
            field_row(ui, t(&lang, "Default action"), |ui| {
                let mut selected = match &general.default_action.0 {
                    DefaultAction::Ask => None,
                    DefaultAction::OpenWith(id) => Some(id.clone()),
                };
                browser_target_combo(
                    ui,
                    "default-action",
                    &mut selected,
                    &browsers,
                    t(&lang, "Ask"),
                );
                general.default_action = DefaultActionSerde(match selected {
                    Some(id) => DefaultAction::OpenWith(id),
                    None => DefaultAction::Ask,
                });
            });
        });

        material_section(ui, t(&lang, "Picker"), |ui| {
            let general = &mut self.config.general;
            field_row(ui, t(&lang, "Position"), |ui| {
                egui::ComboBox::from_id_salt("picker-position")
                    .selected_text(&general.picker_position)
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut general.picker_position,
                            "cursor".to_string(),
                            t(&lang, "Cursor"),
                        );
                        ui.selectable_value(
                            &mut general.picker_position,
                            "center".to_string(),
                            t(&lang, "Center"),
                        );
                    });
            });
            field_row(ui, t(&lang, "Background"), |ui| {
                egui::ComboBox::from_id_salt("picker-background")
                    .selected_text(&general.picker_background)
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut general.picker_background,
                            "bubbles".to_string(),
                            t(&lang, "Bubbles"),
                        );
                        ui.selectable_value(
                            &mut general.picker_background,
                            "solid".to_string(),
                            t(&lang, "Solid"),
                        );
                        ui.selectable_value(
                            &mut general.picker_background,
                            "image".to_string(),
                            t(&lang, "Image"),
                        );
                    });
            });
            field_row(ui, t(&lang, "Background color"), |ui| {
                text_field(ui, &mut general.picker_background_color);
            });
            field_row(ui, t(&lang, "Background image"), |ui| {
                optional_file_field(
                    ui,
                    &mut general.picker_background_image,
                    t(&lang, "Browse..."),
                    pick_image_file,
                );
            });
            field_row(ui, t(&lang, "Icon size"), |ui| {
                ui.add(egui::Slider::new(
                    &mut general.picker_icon_size,
                    40.0..=112.0,
                ));
            });
            field_row(ui, t(&lang, "Padding"), |ui| {
                ui.add(egui::Slider::new(&mut general.picker_padding, 0.0..=56.0));
            });
            field_row(ui, t(&lang, "Window opacity"), |ui| {
                ui.add(egui::Slider::new(
                    &mut general.picker_window_opacity,
                    0.0..=1.0,
                ));
            });
            field_row(ui, t(&lang, "Picker size"), |ui| {
                ui.horizontal(|ui| {
                    ui.add(
                        egui::DragValue::new(&mut general.picker_width)
                            .speed(8.0)
                            .suffix(t(&lang, "px wide")),
                    );
                    ui.add(
                        egui::DragValue::new(&mut general.picker_height)
                            .speed(8.0)
                            .suffix(t(&lang, "px high")),
                    );
                });
            });
            field_row(ui, t(&lang, "Timeout"), |ui| {
                ui.add(
                    egui::DragValue::new(&mut general.picker_timeout_ms)
                        .speed(50.0)
                        .suffix(" ms"),
                );
            });
            field_row(ui, t(&lang, "Background blur"), |ui| {
                ui.checkbox(&mut general.picker_acrylic, t(&lang, "Enabled"));
            });
        });

        material_section(ui, t(&lang, "System"), |ui| {
            let was_start_on_login = self.config.general.start_on_login;
            field_row(ui, tr(Key::StartOnLogin, &lang), |ui| {
                ui.checkbox(&mut self.config.general.start_on_login, t(&lang, "Enabled"));
            });
            if self.config.general.start_on_login != was_start_on_login {
                let enabled = self.config.general.start_on_login;
                match br_platform::current().set_autostart(enabled) {
                    Ok(()) => {
                        self.status = Some(if enabled {
                            if lang.eq_ignore_ascii_case("pt-BR") || lang.eq_ignore_ascii_case("pt")
                            {
                                "br-daemon agora iniciará com o sistema.".to_string()
                            } else {
                                "br-daemon will now start on login.".to_string()
                            }
                        } else {
                            if lang.eq_ignore_ascii_case("pt-BR") || lang.eq_ignore_ascii_case("pt")
                            {
                                "Inicialização automática do br-daemon desativada.".to_string()
                            } else {
                                "br-daemon autostart disabled.".to_string()
                            }
                        })
                    }
                    Err(err) => {
                        self.status = Some(
                            if lang.eq_ignore_ascii_case("pt-BR") || lang.eq_ignore_ascii_case("pt")
                            {
                                format!("Falha ao atualizar inicialização automática: {err}")
                            } else {
                                format!("Failed to update autostart: {err}")
                            },
                        );
                    }
                }
            }

            let is_default = br_platform::current().is_default_handler().unwrap_or(false);
            field_row(ui, t(&lang, "Default handler"), |ui| {
                ui.horizontal(|ui| {
                    ui.label(if is_default {
                        t(&lang, "br is the default browser handler.")
                    } else {
                        t(&lang, "br is not the default browser handler.")
                    });
                    if ui
                        .add(tonal_button(tr(Key::RegisterDefaultBrowser, &lang)))
                        .clicked()
                    {
                        self.register_default_handler();
                    }
                });
            });
            field_row(ui, t(&lang, "Browsers"), |ui| {
                if ui
                    .add(tonal_button(tr(Key::DiscoverBrowsers, &lang)))
                    .clicked()
                {
                    self.discover_browsers();
                }
            });
        });
    }

    fn show_browsers(&mut self, ui: &mut egui::Ui) {
        let lang = self.config.general.language.clone();
        let mut remove_idx = None;
        let mut move_up_idx = None;
        let mut move_down_idx = None;
        let browser_count = self.config.browsers.len();
        for (i, browser) in self.config.browsers.iter_mut().enumerate() {
            let title = if browser.name.is_empty() {
                format!("{} {}", t(&lang, "Browser"), i + 1)
            } else {
                browser.name.clone()
            };
            ui.push_id(i, |ui| {
                material_section(ui, &title, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(t(&lang, "Browser target"))
                                .color(material_on_surface_variant()),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.add(danger_button(t(&lang, "Remove"))).clicked() {
                                remove_idx = Some(i);
                            }
                            if ui
                                .add_enabled(
                                    i + 1 < browser_count,
                                    tonal_button(t(&lang, "Move down")),
                                )
                                .clicked()
                            {
                                move_down_idx = Some(i);
                            }
                            if ui
                                .add_enabled(i > 0, tonal_button(t(&lang, "Move up")))
                                .clicked()
                            {
                                move_up_idx = Some(i);
                            }
                        });
                    });
                    ui.add_space(4.0);
                    field_row(ui, t(&lang, "Id"), |ui| text_field(ui, &mut browser.id));
                    field_row(ui, t(&lang, "Name"), |ui| text_field(ui, &mut browser.name));
                    field_row(ui, t(&lang, "Kind"), |ui| {
                        egui::ComboBox::from_id_salt("browser-kind")
                            .selected_text(&browser.kind)
                            .width(ui.available_width())
                            .show_ui(ui, |ui| {
                                for kind in ["chromium", "firefox", "edge", "generic"] {
                                    ui.selectable_value(&mut browser.kind, kind.to_string(), kind);
                                }
                            });
                    });
                    field_row(ui, t(&lang, "Executable"), |ui| {
                        file_field(
                            ui,
                            &mut browser.executable,
                            t(&lang, "Browse..."),
                            pick_executable_file,
                        );
                    });
                    field_row(ui, t(&lang, "Args"), |ui| {
                        csv_field(ui, &mut browser.args);
                    });
                    field_row(ui, t(&lang, "Profile directory"), |ui| {
                        optional_file_field(
                            ui,
                            &mut browser.profile_dir,
                            t(&lang, "Browse..."),
                            pick_directory,
                        );
                    });
                    field_row(ui, t(&lang, "Profile name"), |ui| {
                        optional_text_field(ui, &mut browser.profile_name);
                    });
                    field_row(ui, t(&lang, "Icon"), |ui| {
                        optional_file_field(
                            ui,
                            &mut browser.icon,
                            t(&lang, "Browse..."),
                            pick_icon_file,
                        );
                    });
                    field_row(ui, t(&lang, "Hidden"), |ui| {
                        ui.checkbox(&mut browser.hidden, t(&lang, "Hide from picker"));
                    });
                });
            });
        }
        if let Some(i) = remove_idx {
            self.config.browsers.remove(i);
        } else if let Some(i) = move_up_idx {
            self.config.browsers.swap(i, i - 1);
        } else if let Some(i) = move_down_idx {
            self.config.browsers.swap(i, i + 1);
        }
        material_section(ui, t(&lang, "Actions"), |ui| {
            ui.horizontal(|ui| {
                if ui.add(filled_button(t(&lang, "Add browser"))).clicked() {
                    self.config.browsers.push(BrowserTarget {
                        id: format!("browser-{}", self.config.browsers.len() + 1),
                        name: t(&lang, "New browser").to_string(),
                        kind: "generic".to_string(),
                        executable: "auto".to_string(),
                        args: vec![],
                        profile_dir: None,
                        profile_name: None,
                        icon: None,
                        hidden: false,
                    });
                }
                if ui
                    .add(tonal_button(tr(Key::DiscoverBrowsers, &lang)))
                    .clicked()
                {
                    self.discover_browsers();
                }
            });
        });
    }

    fn show_filters(&mut self, ui: &mut egui::Ui) {
        let lang = self.config.general.language.clone();
        let mut remove_idx = None;
        for (i, filter) in self.config.filters.iter_mut().enumerate() {
            let title = if filter.id.is_empty() {
                format!("{} {}", t(&lang, "Filter"), i + 1)
            } else {
                filter.id.clone()
            };
            ui.push_id(i, |ui| {
                material_section(ui, &title, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(t(&lang, "URL cleanup"))
                                .color(material_on_surface_variant()),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.add(danger_button(t(&lang, "Remove"))).clicked() {
                                remove_idx = Some(i);
                            }
                        });
                    });
                    ui.add_space(4.0);
                    field_row(ui, t(&lang, "Id"), |ui| text_field(ui, &mut filter.id));
                    field_row(ui, t(&lang, "Enabled"), |ui| {
                        ui.checkbox(&mut filter.enabled, t(&lang, "Enabled"));
                    });
                    field_row(ui, t(&lang, "Strip query params"), |ui| {
                        csv_field(ui, &mut filter.strip_query_params);
                    });
                    field_row(ui, t(&lang, "Upgrade HTTPS"), |ui| {
                        ui.checkbox(&mut filter.upgrade_http_to_https, t(&lang, "Enabled"));
                    });
                    field_row(ui, t(&lang, "Exceptions"), |ui| {
                        csv_field(ui, &mut filter.exceptions);
                    });
                });
            });
        }
        if let Some(i) = remove_idx {
            self.config.filters.remove(i);
        }
        material_section(ui, t(&lang, "Actions"), |ui| {
            if ui.add(filled_button(t(&lang, "Add filter"))).clicked() {
                self.config.filters.push(Filter {
                    id: format!("filter-{}", self.config.filters.len() + 1),
                    enabled: true,
                    ..Default::default()
                });
            }
        });
    }

    fn show_rules(&mut self, ui: &mut egui::Ui) {
        let lang = self.config.general.language.clone();
        let mut remove_idx = None;
        let browsers = self.config.browsers.clone();
        self.config.rules.sort_by_key(|rule| Reverse(rule.priority));
        for (i, rule) in self.config.rules.iter_mut().enumerate() {
            let title = if rule.name.is_empty() {
                format!("{} {}", t(&lang, "Rule"), i + 1)
            } else {
                rule.name.clone()
            };
            ui.push_id(i, |ui| {
                material_section(ui, &title, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(t(&lang, "Routing rule"))
                                .color(material_on_surface_variant()),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.add(danger_button(t(&lang, "Remove"))).clicked() {
                                remove_idx = Some(i);
                            }
                        });
                    });
                    ui.add_space(4.0);
                    field_row(ui, t(&lang, "Id"), |ui| text_field(ui, &mut rule.id));
                    field_row(ui, t(&lang, "Name"), |ui| text_field(ui, &mut rule.name));
                    field_row(ui, t(&lang, "Enabled"), |ui| {
                        ui.checkbox(&mut rule.enabled, t(&lang, "Enabled"));
                    });
                    field_row(ui, t(&lang, "Priority"), |ui| {
                        ui.add(egui::DragValue::new(&mut rule.priority).speed(1.0));
                    });
                    field_row(ui, t(&lang, "URL patterns"), |ui| {
                        csv_field(ui, &mut rule.match_.url_pattern);
                    });
                    field_row(ui, t(&lang, "Source apps"), |ui| {
                        csv_field(ui, &mut rule.match_.source_app);
                    });
                    field_row(ui, t(&lang, "Open with"), |ui| {
                        browser_target_combo(
                            ui,
                            "rule-open-with",
                            &mut rule.action.open_with,
                            &browsers,
                            t(&lang, "Ask"),
                        );
                        rule.action.ask = rule.action.open_with.is_none();
                    });
                    field_row(ui, t(&lang, "Open with all"), |ui| {
                        ui.horizontal_wrapped(|ui| {
                            for browser in &browsers {
                                let mut selected = rule.action.open_with_all.contains(&browser.id);
                                if ui
                                    .checkbox(
                                        &mut selected,
                                        format!("{} ({})", browser.name, browser.id),
                                    )
                                    .changed()
                                {
                                    if selected {
                                        rule.action.open_with_all.push(browser.id.clone());
                                    } else {
                                        rule.action.open_with_all.retain(|id| id != &browser.id);
                                    }
                                }
                            }
                        });
                    });
                    field_row(ui, t(&lang, "Modifier keys"), |ui| {
                        ui.horizontal_wrapped(|ui| {
                            for key in ["shift", "ctrl", "alt", "meta"] {
                                let key_string = key.to_string();
                                let mut selected = rule.match_.modifier_keys.contains(&key_string);
                                if ui.checkbox(&mut selected, key).changed() {
                                    if selected {
                                        rule.match_.modifier_keys.push(key_string);
                                    } else {
                                        rule.match_.modifier_keys.retain(|value| value != key);
                                    }
                                }
                            }
                        });
                    });
                    field_row(ui, t(&lang, "Mode"), |ui| {
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut rule.action.private, t(&lang, "Private"));
                            ui.checkbox(&mut rule.action.block, t(&lang, "Block"));
                        });
                    });
                });
            });
        }
        if let Some(i) = remove_idx {
            self.config.rules.remove(i);
        }
        material_section(ui, t(&lang, "Actions"), |ui| {
            if ui.add(filled_button(t(&lang, "Add rule"))).clicked() {
                let next_priority = self
                    .config
                    .rules
                    .iter()
                    .map(|r| r.priority)
                    .max()
                    .unwrap_or(0)
                    + 10;
                self.config.rules.push(Rule {
                    id: format!("rule-{}", self.config.rules.len() + 1),
                    name: t(&lang, "New rule").to_string(),
                    enabled: true,
                    priority: next_priority,
                    match_: br_core::MatchCondition::default(),
                    action: br_core::Action {
                        ask: true,
                        ..Default::default()
                    },
                });
            }
        });
    }

    fn show_about(&mut self, ui: &mut egui::Ui) {
        let lang = self.config.general.language.clone();
        material_section(ui, t(&lang, "About"), |ui| {
            ui.horizontal_wrapped(|ui| {
                if let Some(texture) = logo_texture(ui.ctx()) {
                    ui.image((texture.id(), egui::vec2(96.0, 96.0)));
                }
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new(APP_NAME)
                            .size(24.0)
                            .strong()
                            .color(material_on_surface()),
                    );
                    ui.label(
                        egui::RichText::new(t(&lang, "Link and browser profile router"))
                            .color(material_on_surface_variant()),
                    );
                    ui.add_space(8.0);
                    field_row(ui, t(&lang, "Name"), |ui| {
                        ui.label(APP_NAME);
                    });
                    field_row(ui, t(&lang, "Creator"), |ui| {
                        ui.label(APP_CREATOR);
                    });
                    field_row(ui, t(&lang, "Version"), |ui| {
                        ui.label(env!("CARGO_PKG_VERSION"));
                    });
                    field_row(ui, t(&lang, "Repository"), |ui| {
                        ui.hyperlink_to(APP_REPOSITORY, APP_REPOSITORY);
                    });
                });
            });
        });
    }
}

fn split_csv(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}
