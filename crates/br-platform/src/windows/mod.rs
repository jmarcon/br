mod browsers;
mod foreground;
mod register;

use crate::{PlatformIntegration, RegisterOutcome};
use br_core::BrowserTarget;

/// Windows implementation of [`PlatformIntegration`].
///
/// Browser discovery reads `HKEY_LOCAL_MACHINE\SOFTWARE\Clients\StartMenuInternet`
/// for installed browsers, plus each browser's `Local State` (Chromium) or
/// `profiles.ini` (Firefox) for profiles. Default-handler registration writes
/// to `HKEY_CURRENT_USER\Software\Classes` and `RegisteredApplications`, then
/// opens the Windows 11 "Default apps" settings page (RF-37).
#[derive(Debug, Default)]
pub struct WindowsPlatform;

/// `ProgId` registered under `HKEY_CURRENT_USER\Software\Classes` for `br`.
pub(crate) const PROG_ID: &str = "BrowserRouter.HTTP";

impl PlatformIntegration for WindowsPlatform {
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
}
