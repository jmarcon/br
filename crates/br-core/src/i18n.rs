//! Minimal localization for UI strings (PRD §8.4 `language = "pt-BR"`).
//!
//! Keeps a small fixed set of UI strings translated for `en` and `pt-BR`,
//! selected by `Config::general.language`. Falls back to `en` for unknown
//! languages or keys.

/// A UI string identifier, translated by [`tr`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    SettingsTitle,
    TabGeneral,
    TabBrowsers,
    TabFilters,
    TabRules,
    Save,
    DiscoverBrowsers,
    RegisterDefaultBrowser,
    StartOnLogin,
    OnboardingWelcomeTitle,
    OnboardingWelcomeBody,
    OnboardingFinish,
    PickerOpenWith,
    PickerHelp,
}

/// Returns the translated string for `key` in `language` (e.g. `"pt-BR"`,
/// `"pt"`, `"en"`). Unknown languages fall back to English.
pub fn tr(key: Key, language: &str) -> &'static str {
    if language.eq_ignore_ascii_case("pt-BR") || language.eq_ignore_ascii_case("pt") {
        pt_br(key)
    } else {
        en(key)
    }
}

fn en(key: Key) -> &'static str {
    match key {
        Key::SettingsTitle => "br — Settings",
        Key::TabGeneral => "General",
        Key::TabBrowsers => "Browsers",
        Key::TabFilters => "Filters",
        Key::TabRules => "Rules",
        Key::Save => "Save",
        Key::DiscoverBrowsers => "Discover browsers",
        Key::RegisterDefaultBrowser => "Register as default browser",
        Key::StartOnLogin => "Start on login (runs br-daemon)",
        Key::OnboardingWelcomeTitle => "Welcome to br",
        Key::OnboardingWelcomeBody => {
            "br routes links to the right browser/profile based on rules you define."
        }
        Key::OnboardingFinish => "Finish and open settings",
        Key::PickerOpenWith => "Open:",
        Key::PickerHelp => {
            "Up/Down + Enter, or 1-9 to choose. Hold Shift for private mode. Esc to cancel."
        }
    }
}

fn pt_br(key: Key) -> &'static str {
    match key {
        Key::SettingsTitle => "br — Configurações",
        Key::TabGeneral => "Geral",
        Key::TabBrowsers => "Navegadores",
        Key::TabFilters => "Filtros",
        Key::TabRules => "Regras",
        Key::Save => "Salvar",
        Key::DiscoverBrowsers => "Detectar navegadores",
        Key::RegisterDefaultBrowser => "Definir como navegador padrão",
        Key::StartOnLogin => "Iniciar com o sistema (executa br-daemon)",
        Key::OnboardingWelcomeTitle => "Bem-vindo ao br",
        Key::OnboardingWelcomeBody => {
            "O br direciona links para o navegador/perfil certo com base em regras definidas por você."
        }
        Key::OnboardingFinish => "Concluir e abrir configurações",
        Key::PickerOpenWith => "Abrir:",
        Key::PickerHelp => {
            "Setas + Enter, ou 1-9 para escolher. Segure Shift para modo privado. Esc para cancelar."
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn falls_back_to_english_for_unknown_language() {
        assert_eq!(tr(Key::Save, "fr"), "Save");
    }

    #[test]
    fn translates_pt_br() {
        assert_eq!(tr(Key::Save, "pt-BR"), "Salvar");
        assert_eq!(tr(Key::Save, "pt"), "Salvar");
    }

    #[test]
    fn language_matching_is_case_insensitive() {
        assert_eq!(tr(Key::TabGeneral, "PT-br"), "Geral");
    }
}
