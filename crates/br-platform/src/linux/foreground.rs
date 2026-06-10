use std::fs;

/// Best-effort name of the foreground application's process, used as
/// `source_app` for rule matching (RF-37 / "best effort" per PRD §12).
///
/// Reads the active window's PID via `xdotool` (X11/most desktops) or `swaymsg`
/// (Sway/wlroots), then resolves the process name from `/proc/<pid>/comm`.
pub fn get_foreground_app_name() -> Option<String> {
    let pid = active_window_pid()?;
    let comm = fs::read_to_string(format!("/proc/{pid}/comm")).ok()?;
    Some(comm.trim().to_string())
}

fn active_window_pid() -> Option<u32> {
    if let Some(pid) = pid_via_xdotool() {
        return Some(pid);
    }
    pid_via_swaymsg()
}

fn pid_via_xdotool() -> Option<u32> {
    let output = std::process::Command::new("xdotool")
        .args(["getactivewindow", "getwindowpid"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout).trim().parse().ok()
}

fn pid_via_swaymsg() -> Option<u32> {
    let output = std::process::Command::new("swaymsg")
        .args(["-t", "get_tree"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
    find_focused_pid(&json)
}

fn find_focused_pid(node: &serde_json::Value) -> Option<u32> {
    if node.get("focused").and_then(|v| v.as_bool()) == Some(true) {
        if let Some(pid) = node.get("pid").and_then(|v| v.as_u64()) {
            return Some(pid as u32);
        }
    }
    for child_key in ["nodes", "floating_nodes"] {
        if let Some(children) = node.get(child_key).and_then(|v| v.as_array()) {
            for child in children {
                if let Some(pid) = find_focused_pid(child) {
                    return Some(pid);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_focused_pid_in_sway_tree() {
        let tree = serde_json::json!({
            "nodes": [
                {"focused": false, "pid": 100},
                {"nodes": [
                    {"focused": true, "pid": 4242}
                ]}
            ]
        });
        assert_eq!(find_focused_pid(&tree), Some(4242));
    }

    #[test]
    fn returns_none_when_nothing_focused() {
        let tree = serde_json::json!({"nodes": [{"focused": false, "pid": 1}]});
        assert_eq!(find_focused_pid(&tree), None);
    }
}
