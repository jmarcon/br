//! Settings application for `br` (PRD §9.1 onboarding, §9.3 settings UI).
//!
//! Lets the user manage browsers/profiles, filters, and rules, and walks
//! first-time users through an onboarding flow that discovers installed
//! browsers and registers `br` as the default handler.

use anyhow::{Context, Result};
use br_core::i18n::{tr, Key};
use br_core::{BrowserTarget, Config, Filter, Rule};
use br_platform::PlatformIntegration;
use eframe::egui;
use std::path::{Path, PathBuf};

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

/// Opens the settings window, optionally starting at a specific config path.
pub fn run(config_path: Option<PathBuf>) -> Result<()> {
    let path = config_path.unwrap_or_else(default_config_path);
    let (config, needs_onboarding) = load(&path);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([720.0, 520.0])
            .with_title(tr(Key::SettingsTitle, &config.general.language)),
        ..Default::default()
    };

    eframe::run_native(
        "br-settings",
        options,
        Box::new(move |_cc| {
            Ok(Box::new(SettingsApp::new(path, config, needs_onboarding)))
        }),
    )
    .map_err(|e| anyhow::anyhow!("failed to run settings UI: {e}"))
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum Tab {
    General,
    Browsers,
    Filters,
    Rules,
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
            Ok(()) => self.status = Some(format!("Saved to {}", self.path.display())),
            Err(err) => self.status = Some(format!("Failed to save: {err}")),
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
                self.status = Some(format!("Discovered {added} new browser(s)."));
            }
            Err(err) => self.status = Some(format!("Discovery failed: {err}")),
        }
    }

    fn register_default_handler(&mut self) {
        let platform = br_platform::current();
        match platform.register_as_default_handler() {
            Ok(br_platform::RegisterOutcome::Registered) => {
                self.status = Some("br is now the default browser.".to_string());
            }
            Ok(br_platform::RegisterOutcome::NeedsManualConfirmation { instructions }) => {
                self.status = Some(instructions);
            }
            Err(err) => self.status = Some(format!("Registration failed: {err}")),
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

    fn show_onboarding(&mut self, ctx: &egui::Context) {
        let lang = self.config.general.language.clone();
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(tr(Key::OnboardingWelcomeTitle, &lang));
            ui.label(tr(Key::OnboardingWelcomeBody, &lang));
            ui.add_space(12.0);

            ui.label("1. Discover installed browsers:");
            if ui.button(tr(Key::DiscoverBrowsers, &lang)).clicked() {
                self.discover_browsers();
            }
            ui.label(format!("   {} browser(s) configured.", self.config.browsers.len()));
            ui.add_space(8.0);

            ui.label("2. Set br as your default browser:");
            if ui.button(tr(Key::RegisterDefaultBrowser, &lang)).clicked() {
                self.register_default_handler();
            }
            ui.add_space(8.0);

            ui.label("3. Finish setup:");
            if ui.button(tr(Key::OnboardingFinish, &lang)).clicked() {
                self.ensure_fallback_rule();
                self.save();
                self.onboarding = false;
            }

            if let Some(status) = &self.status {
                ui.add_space(12.0);
                ui.colored_label(egui::Color32::LIGHT_BLUE, status);
            }
        });
    }
}

impl eframe::App for SettingsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.onboarding {
            self.show_onboarding(ctx);
            return;
        }

        let lang = self.config.general.language.clone();
        egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.tab, Tab::General, tr(Key::TabGeneral, &lang));
                ui.selectable_value(&mut self.tab, Tab::Browsers, tr(Key::TabBrowsers, &lang));
                ui.selectable_value(&mut self.tab, Tab::Filters, tr(Key::TabFilters, &lang));
                ui.selectable_value(&mut self.tab, Tab::Rules, tr(Key::TabRules, &lang));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(tr(Key::Save, &lang)).clicked() {
                        self.save();
                    }
                });
            });
        });

        if let Some(status) = self.status.clone() {
            egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
                ui.label(status);
            });
        }

        egui::CentralPanel::default().show(ctx, |ui| match self.tab {
            Tab::General => self.show_general(ui),
            Tab::Browsers => self.show_browsers(ui),
            Tab::Filters => self.show_filters(ui),
            Tab::Rules => self.show_rules(ui),
        });
    }
}

impl SettingsApp {
    fn show_general(&mut self, ui: &mut egui::Ui) {
        let lang = self.config.general.language.clone();
        let general = &mut self.config.general;
        ui.horizontal(|ui| {
            ui.label("Theme:");
            ui.text_edit_singleline(&mut general.theme);
        });
        ui.horizontal(|ui| {
            ui.label("Language:");
            ui.text_edit_singleline(&mut general.language);
        });
        ui.horizontal(|ui| {
            ui.label("Picker position:");
            ui.text_edit_singleline(&mut general.picker_position);
        });
        ui.horizontal(|ui| {
            ui.label("Picker timeout (ms, 0 = none):");
            let mut value = general.picker_timeout_ms.to_string();
            if ui.text_edit_singleline(&mut value).changed() {
                if let Ok(parsed) = value.parse() {
                    general.picker_timeout_ms = parsed;
                }
            }
        });
        let was_start_on_login = general.start_on_login;
        ui.checkbox(&mut general.start_on_login, tr(Key::StartOnLogin, &lang));
        if general.start_on_login != was_start_on_login {
            let enabled = general.start_on_login;
            match br_platform::current().set_autostart(enabled) {
                Ok(()) => {
                    self.status = Some(if enabled {
                        "br-daemon will now start on login.".to_string()
                    } else {
                        "br-daemon autostart disabled.".to_string()
                    })
                }
                Err(err) => self.status = Some(format!("Failed to update autostart: {err}")),
            }
        }
        ui.add_space(8.0);
        ui.separator();
        if ui.button(tr(Key::DiscoverBrowsers, &lang)).clicked() {
            self.discover_browsers();
        }
        ui.add_space(4.0);
        let is_default = br_platform::current().is_default_handler().unwrap_or(false);
        ui.label(if is_default {
            "br is the default browser handler."
        } else {
            "br is NOT the default browser handler."
        });
        if ui.button(tr(Key::RegisterDefaultBrowser, &lang)).clicked() {
            self.register_default_handler();
        }
    }

    fn show_browsers(&mut self, ui: &mut egui::Ui) {
        let lang = self.config.general.language.clone();
        let mut remove_idx = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (i, browser) in self.config.browsers.iter_mut().enumerate() {
                ui.push_id(i, |ui| {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label("Id:");
                            ui.text_edit_singleline(&mut browser.id);
                            ui.label("Name:");
                            ui.text_edit_singleline(&mut browser.name);
                            if ui.button("Remove").clicked() {
                                remove_idx = Some(i);
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("Kind:");
                            ui.text_edit_singleline(&mut browser.kind);
                            ui.label("Executable:");
                            ui.text_edit_singleline(&mut browser.executable);
                            ui.checkbox(&mut browser.hidden, "Hidden");
                        });
                    });
                });
            }
        });
        if let Some(i) = remove_idx {
            self.config.browsers.remove(i);
        }
        ui.add_space(8.0);
        if ui.button("Add browser").clicked() {
            self.config.browsers.push(BrowserTarget {
                id: format!("browser-{}", self.config.browsers.len() + 1),
                name: "New browser".to_string(),
                kind: "generic".to_string(),
                executable: "auto".to_string(),
                args: vec![],
                profile_dir: None,
                profile_name: None,
                icon: None,
                hidden: false,
            });
        }
        if ui.button(tr(Key::DiscoverBrowsers, &lang)).clicked() {
            self.discover_browsers();
        }
    }

    fn show_filters(&mut self, ui: &mut egui::Ui) {
        let mut remove_idx = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (i, filter) in self.config.filters.iter_mut().enumerate() {
                ui.push_id(i, |ui| {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label("Id:");
                            ui.text_edit_singleline(&mut filter.id);
                            ui.checkbox(&mut filter.enabled, "Enabled");
                            if ui.button("Remove").clicked() {
                                remove_idx = Some(i);
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("Strip query params (comma-separated):");
                            let mut value = filter.strip_query_params.join(", ");
                            if ui.text_edit_singleline(&mut value).changed() {
                                filter.strip_query_params = split_csv(&value);
                            }
                        });
                        ui.checkbox(&mut filter.upgrade_http_to_https, "Upgrade HTTP to HTTPS");
                        ui.horizontal(|ui| {
                            ui.label("Exceptions (comma-separated):");
                            let mut value = filter.exceptions.join(", ");
                            if ui.text_edit_singleline(&mut value).changed() {
                                filter.exceptions = split_csv(&value);
                            }
                        });
                    });
                });
            }
        });
        if let Some(i) = remove_idx {
            self.config.filters.remove(i);
        }
        ui.add_space(8.0);
        if ui.button("Add filter").clicked() {
            self.config.filters.push(Filter {
                id: format!("filter-{}", self.config.filters.len() + 1),
                enabled: true,
                ..Default::default()
            });
        }
    }

    fn show_rules(&mut self, ui: &mut egui::Ui) {
        let mut remove_idx = None;
        self.config.rules.sort_by(|a, b| b.priority.cmp(&a.priority));
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (i, rule) in self.config.rules.iter_mut().enumerate() {
                ui.push_id(i, |ui| {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label("Id:");
                            ui.text_edit_singleline(&mut rule.id);
                            ui.label("Name:");
                            ui.text_edit_singleline(&mut rule.name);
                            ui.checkbox(&mut rule.enabled, "Enabled");
                            if ui.button("Remove").clicked() {
                                remove_idx = Some(i);
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("Priority:");
                            let mut value = rule.priority.to_string();
                            if ui.text_edit_singleline(&mut value).changed() {
                                if let Ok(parsed) = value.parse() {
                                    rule.priority = parsed;
                                }
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("URL patterns (comma-separated):");
                            let mut value = rule.match_.url_pattern.join(", ");
                            if ui.text_edit_singleline(&mut value).changed() {
                                rule.match_.url_pattern = split_csv(&value);
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("Source apps (comma-separated):");
                            let mut value = rule.match_.source_app.join(", ");
                            if ui.text_edit_singleline(&mut value).changed() {
                                rule.match_.source_app = split_csv(&value);
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("Open with (browser id, blank = ask):");
                            let mut value = rule.action.open_with.clone().unwrap_or_default();
                            if ui.text_edit_singleline(&mut value).changed() {
                                rule.action.ask = value.is_empty();
                                rule.action.open_with =
                                    if value.is_empty() { None } else { Some(value) };
                            }
                            ui.checkbox(&mut rule.action.private, "Private");
                            ui.checkbox(&mut rule.action.block, "Block");
                        });
                    });
                });
            }
        });
        if let Some(i) = remove_idx {
            self.config.rules.remove(i);
        }
        ui.add_space(8.0);
        if ui.button("Add rule").clicked() {
            let next_priority = self.config.rules.iter().map(|r| r.priority).max().unwrap_or(0) + 10;
            self.config.rules.push(Rule {
                id: format!("rule-{}", self.config.rules.len() + 1),
                name: "New rule".to_string(),
                enabled: true,
                priority: next_priority,
                match_: br_core::MatchCondition::default(),
                action: br_core::Action {
                    ask: true,
                    ..Default::default()
                },
            });
        }
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
