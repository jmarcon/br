use anyhow::{Context, Result};
use br_core::{BrowserTarget, Config};
use std::process::Command;

use crate::PlatformIntegration;

/// Launches the browser/profile referenced by `target_id` with `url`.
///
/// If the target's `executable` is `"auto"`, it is resolved against the
/// browsers discovered by [`PlatformIntegration::discover_browsers`].
pub fn launch(platform: &impl PlatformIntegration, cfg: &Config, target_id: &str, url: &str, private: bool) -> Result<()> {
    let target = cfg
        .browsers
        .iter()
        .find(|b| b.id == target_id)
        .with_context(|| format!("unknown browser target '{target_id}'"))?;

    let resolved_executable;
    let executable = if target.executable == "auto" {
        let discovered = platform.discover_browsers().unwrap_or_default();
        let found = resolve_auto(target, &discovered).with_context(|| {
            format!(
                "browser '{}' has executable = \"auto\" but could not be auto-detected; \
set an explicit executable path",
                target.id
            )
        })?;
        resolved_executable = found.executable.clone();
        &resolved_executable
    } else {
        &target.executable
    };

    let mut command = Command::new(executable);
    command.args(&target.args);

    if let Some(profile_dir) = &target.profile_dir {
        command.arg(format!("--profile-directory={profile_dir}"));
    }
    if let Some(profile_name) = &target.profile_name {
        command.arg("-P").arg(profile_name);
    }
    if private {
        match target.kind.as_str() {
            "chromium" => {
                command.arg("--incognito");
            }
            "firefox" => {
                command.arg("--private-window");
            }
            _ => {}
        }
    }
    command.arg(url);

    command
        .spawn()
        .with_context(|| format!("failed to launch '{executable}'"))?;
    Ok(())
}

fn resolve_auto<'a>(target: &BrowserTarget, discovered: &'a [BrowserTarget]) -> Option<&'a BrowserTarget> {
    discovered
        .iter()
        .find(|b| b.id == target.id || (b.kind == target.kind && b.name.contains(&target.name)))
}
