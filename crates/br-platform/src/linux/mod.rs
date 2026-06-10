mod browsers;
mod foreground;
mod register;

use crate::{PlatformIntegration, RegisterOutcome};
use br_core::BrowserTarget;

/// Linux implementation of [`PlatformIntegration`].
///
/// Browser discovery scans `.desktop` files under the standard freedesktop
/// application directories for `x-scheme-handler/http(s)` handlers (RF-39).
/// Default-handler registration installs a `.desktop` file and uses
/// `xdg-mime`/`xdg-settings` (RF-37). Foreground app detection uses
/// `xdotool`/`swaymsg` plus `/proc/<pid>/comm` (best effort).
#[derive(Debug, Default)]
pub struct LinuxPlatform;

impl PlatformIntegration for LinuxPlatform {
    fn discover_browsers(&self) -> anyhow::Result<Vec<BrowserTarget>> {
        Ok(browsers::discover())
    }

    fn is_default_handler(&self) -> anyhow::Result<bool> {
        register::is_default_handler()
    }

    fn register_as_default_handler(&self) -> anyhow::Result<RegisterOutcome> {
        register::register_as_default_handler()
    }

    fn get_foreground_app_name(&self) -> Option<String> {
        foreground::get_foreground_app_name()
    }

    fn set_autostart(&self, enabled: bool) -> anyhow::Result<()> {
        register::set_autostart(enabled)
    }

    fn is_autostart_enabled(&self) -> anyhow::Result<bool> {
        register::is_autostart_enabled()
    }
}
