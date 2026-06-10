use crate::{PlatformIntegration, RegisterOutcome};
use br_core::BrowserTarget;

/// Fallback platform integration for OSes without a dedicated implementation yet
/// (macOS, Linux). All operations are no-ops / report "not implemented".
#[derive(Debug, Default)]
pub struct UnsupportedPlatform;

impl PlatformIntegration for UnsupportedPlatform {
    fn discover_browsers(&self) -> anyhow::Result<Vec<BrowserTarget>> {
        Ok(Vec::new())
    }

    fn is_default_handler(&self) -> anyhow::Result<bool> {
        Ok(false)
    }

    fn register_as_default_handler(&self) -> anyhow::Result<RegisterOutcome> {
        anyhow::bail!(
            "default handler registration is not yet implemented for {}",
            std::env::consts::OS
        )
    }

    fn get_foreground_app_name(&self) -> Option<String> {
        None
    }

    fn set_autostart(&self, _enabled: bool) -> anyhow::Result<()> {
        anyhow::bail!(
            "autostart is not yet implemented for {}",
            std::env::consts::OS
        )
    }

    fn is_autostart_enabled(&self) -> anyhow::Result<bool> {
        Ok(false)
    }
}
