// SPDX-License-Identifier: AGPL-3.0-or-later
//! Host-level state that determines whether a driver rotation can run safely
//! alongside a live desktop session.

use std::path::Path;

/// Kernel taint bit 7: an oops or BUG has occurred since boot.
const TAINT_BIT_OOPS: u32 = 7;

/// Whether the kernel has recorded an oops or BUG since boot.
///
/// After an oops the kernel is in an undefined state: locks may be held by
/// dead tasks and refcounts leaked. Rotating drivers on a `D`-tainted kernel
/// compounds an existing fault rather than producing a clean experiment.
#[must_use]
pub fn kernel_oops_tainted() -> bool {
    read_taint().is_some_and(|t| t & (1 << TAINT_BIT_OOPS) != 0)
}

fn read_taint() -> Option<u32> {
    std::fs::read_to_string("/proc/sys/kernel/tainted")
        .ok()?
        .trim()
        .parse()
        .ok()
}

/// Whether Xorg is configured to refuse runtime GPU hot-add.
///
/// Both options default to *on* when unset, so an absent configuration is
/// unsafe rather than neutral. A seeder that registers a DRM node on a host
/// with hot-add enabled will have that node claimed by the running server.
#[must_use]
pub fn xorg_gpu_hotadd_disabled() -> bool {
    let mut auto_add_off = false;
    let mut auto_bind_off = false;

    for text in xorg_config_sources() {
        for line in text.lines() {
            let line = line.trim();
            if line.starts_with('#') {
                continue;
            }
            let lower = line.to_ascii_lowercase();
            if lower.contains("\"autoaddgpu\"") && contains_off_value(&lower) {
                auto_add_off = true;
            }
            if lower.contains("\"autobindgpu\"") && contains_off_value(&lower) {
                auto_bind_off = true;
            }
        }
    }

    auto_add_off && auto_bind_off
}

/// Xorg treats these as false; anything else (including absence) is true.
fn contains_off_value(lower_line: &str) -> bool {
    ["\"off\"", "\"false\"", "\"no\"", "\"0\""]
        .iter()
        .any(|v| lower_line.contains(v))
}

fn xorg_config_sources() -> Vec<String> {
    let mut out = Vec::new();

    if let Ok(text) = std::fs::read_to_string("/etc/X11/xorg.conf") {
        out.push(text);
    }

    let dir = Path::new("/etc/X11/xorg.conf.d");
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "conf")
                && let Ok(text) = std::fs::read_to_string(&path)
            {
                out.push(text);
            }
        }
    }

    out
}

/// Whether any display server is currently running.
///
/// Detected via the abstract X socket directory and the Wayland runtime
/// socket, which do not require inspecting other users' processes.
#[must_use]
pub fn display_server_running() -> bool {
    let x_live = std::fs::read_dir("/tmp/.X11-unix")
        .map(|mut d| d.next().is_some())
        .unwrap_or(false);
    if x_live {
        return true;
    }

    let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") else {
        return false;
    };
    std::fs::read_dir(&runtime_dir)
        .map(|entries| {
            entries.flatten().any(|e| {
                e.file_name()
                    .to_str()
                    .is_some_and(|n| n.starts_with("wayland-") && !n.ends_with(".lock"))
            })
        })
        .unwrap_or(false)
}

/// Whether `bdf` is currently driving a display node.
///
/// Used to refuse a rotation whose target is the machine's own display GPU.
#[must_use]
pub fn is_display_gpu(bdf: &str) -> bool {
    super::drm_watch::DrmNodes::for_device(bdf).has_card_node()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn off_values_recognised_case_insensitively() {
        assert!(contains_off_value("option \"autoaddgpu\" \"off\""));
        assert!(contains_off_value("option \"autoaddgpu\" \"false\""));
        assert!(contains_off_value("option \"autoaddgpu\" \"no\""));
        assert!(contains_off_value("option \"autoaddgpu\" \"0\""));
    }

    #[test]
    fn on_values_are_not_treated_as_off() {
        assert!(!contains_off_value("option \"autoaddgpu\" \"on\""));
        assert!(!contains_off_value("option \"autoaddgpu\" \"true\""));
    }

    /// Absent configuration must read as unsafe: Xorg defaults both to on.
    #[test]
    fn absent_config_is_not_mistaken_for_disabled() {
        assert!(!contains_off_value("section \"serverflags\""));
        assert!(!contains_off_value(""));
    }

    #[test]
    fn taint_parsing_detects_oops_bit() {
        // 12929 is the value observed after the nouveau teardown oops.
        let observed: u32 = 12929;
        assert!(observed & (1 << TAINT_BIT_OOPS) != 0);
        // Out-of-tree + unsigned modules alone must not read as an oops.
        let benign: u32 = (1 << 12) | (1 << 13);
        assert!(benign & (1 << TAINT_BIT_OOPS) == 0);
    }

    #[test]
    fn nonexistent_device_is_not_a_display_gpu() {
        assert!(!is_display_gpu("0000:ff:ff.9"));
    }
}
