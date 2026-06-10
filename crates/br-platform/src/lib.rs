//! Per-OS integration for `br`: discovering installed browsers/profiles,
//! checking/registering the default `http`/`https` handler, and (best-effort)
//! identifying the foreground application that originated a link click.

use br_core::BrowserTarget;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use windows::WindowsPlatform as CurrentPlatform;

#[cfg(not(target_os = "windows"))]
mod unsupported;
#[cfg(not(target_os = "windows"))]
pub use unsupported::UnsupportedPlatform as CurrentPlatform;

/// Per-OS hooks for browser discovery and default-handler management.
pub trait PlatformIntegration {
    /// Scans well-known locations for installed browsers and their profiles.
    fn discover_browsers(&self) -> anyhow::Result<Vec<BrowserTarget>>;

    /// Returns whether `br` is currently registered as the default `http`/`https` handler.
    fn is_default_handler(&self) -> anyhow::Result<bool>;

    /// Registers `br` as a candidate default handler and, where the OS requires manual
    /// confirmation, opens the relevant system settings page.
    fn register_as_default_handler(&self) -> anyhow::Result<RegisterOutcome>;

    /// Best-effort name of the foreground application (for `source_app` rule matching).
    fn get_foreground_app_name(&self) -> Option<String>;
}

/// Outcome of [`PlatformIntegration::register_as_default_handler`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegisterOutcome {
    /// Registration completed and `br` is fully active as the default handler.
    Registered,
    /// `br` was registered as a candidate; the user must confirm it manually.
    NeedsManualConfirmation { instructions: String },
}

/// Returns the platform integration for the current OS.
pub fn current() -> CurrentPlatform {
    CurrentPlatform::default()
}
