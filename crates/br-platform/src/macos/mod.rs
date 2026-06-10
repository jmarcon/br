mod browsers;
mod foreground;
mod register;

use crate::{PlatformIntegration, RegisterOutcome};
use br_core::BrowserTarget;

/// macOS implementation of [`PlatformIntegration`].
///
/// Browser discovery scans `/Applications` and `~/Applications` for `.app`
/// bundles whose `Info.plist` declares `CFBundleURLTypes` for `http`/`https`
/// (RF-39). Default-handler registration opens System Settings for manual
/// confirmation (RF-37), since it requires Cocoa `LSSetDefaultHandlerForURLScheme`
/// bindings not used here. Foreground app detection shells out to `osascript`.
#[derive(Debug, Default)]
pub struct MacosPlatform;

impl PlatformIntegration for MacosPlatform {
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
