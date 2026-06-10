use crate::info_plist::{self, BundleInfo};
use crate::util::{classify, slug};
use br_core::BrowserTarget;
use std::path::{Path, PathBuf};

/// Discovers installed browsers by scanning `/Applications` and
/// `~/Applications` for `.app` bundles whose `Info.plist` declares
/// `CFBundleURLTypes` for `http`/`https` (RF-39).
pub fn discover() -> Vec<BrowserTarget> {
    let mut seen_ids = std::collections::HashSet::new();
    let mut targets = Vec::new();

    for dir in application_dirs() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("app") {
                continue;
            }
            let Some(bundle) = read_bundle_info(&path) else {
                continue;
            };
            if !bundle.is_browser() {
                continue;
            }

            let id = slug(&bundle.name);
            if !seen_ids.insert(id.clone()) {
                continue;
            }

            targets.push(BrowserTarget {
                id,
                kind: classify(&bundle.name).to_string(),
                executable: path.to_string_lossy().to_string(),
                name: bundle.name,
                args: vec![],
                profile_dir: None,
                profile_name: None,
                icon: None,
                hidden: false,
            });
        }
    }

    targets
}

#[cfg(target_os = "macos")]
fn read_bundle_info(app_path: &Path) -> Option<BundleInfo> {
    let plist_path = app_path.join("Contents/Info.plist");
    let value = plist::Value::from_file(&plist_path).ok()?;
    let fallback_name = app_path.file_stem()?.to_str()?;
    info_plist::from_plist(&value, fallback_name)
}

#[cfg(not(target_os = "macos"))]
fn read_bundle_info(_app_path: &Path) -> Option<BundleInfo> {
    None
}

fn application_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![PathBuf::from("/Applications")];
    if let Some(home) = dirs::home_dir() {
        dirs.push(home.join("Applications"));
    }
    dirs.into_iter().filter(|d| Path::new(d).is_dir()).collect()
}
