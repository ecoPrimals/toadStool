// SPDX-License-Identifier: AGPL-3.0-only
//! GPU test safety guards — skip wgpu tests on drivers known to SIGSEGV.
//!
//! The NVIDIA proprietary Vulkan driver has a known issue where `wgpu::Device`
//! drop causes a SIGSEGV during test teardown. This affects all tests that
//! create a wgpu adapter/device pair. These guards detect the driver at test
//! time and skip tests that would crash.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use toadstool_testing::gpu_guards;
//!
//! #[test]
//! fn my_wgpu_test() {
//!     if !gpu_guards::is_wgpu_safe() {
//!         eprintln!("SKIP: wgpu unsafe on this driver");
//!         return;
//!     }
//!     // ... test body
//! }
//! ```
//!
//! ## Absorbed From
//!
//! hotSpring v0.6.25 NVVM poisoning discovery. The same NVIDIA proprietary
//! driver issue that poisons wgpu devices on failed f64 transcendental
//! compilation also causes SIGSEGV during device teardown in tests.

/// Returns `true` if the current system is safe for wgpu device creation/teardown.
///
/// Checks:
/// 1. `TOADSTOOL_WGPU_SAFE=1` env override (for CI with known-safe config)
/// 2. `TOADSTOOL_WGPU_SAFE=0` env override (force skip)
/// 3. Driver detection via `TOADSTOOL_GPU_ADAPTER` or sysfs
///
/// Safe drivers: `nvk`, `nouveau`, `radv`, `amdgpu`, `anv`, `i915`
/// Unsafe drivers: `nvidia` (proprietary)
#[must_use]
pub fn is_wgpu_safe() -> bool {
    if let Ok(val) = std::env::var("TOADSTOOL_WGPU_SAFE") {
        return val == "1" || val.eq_ignore_ascii_case("true");
    }

    if let Ok(adapter) = std::env::var("TOADSTOOL_GPU_ADAPTER") {
        let lower = adapter.to_lowercase();
        if lower.contains("nvidia") && !lower.contains("nvk") && !lower.contains("nouveau") {
            return false;
        }
        return true;
    }

    !detect_nvidia_proprietary()
}

/// Returns a human-readable skip message for wgpu tests.
#[must_use]
pub const fn wgpu_skip_reason() -> &'static str {
    "SKIP: NVIDIA proprietary Vulkan driver detected — wgpu device drop causes SIGSEGV. \
     Set TOADSTOOL_WGPU_SAFE=1 to override."
}

/// Detect if the NVIDIA proprietary driver is loaded (Linux sysfs).
fn detect_nvidia_proprietary() -> bool {
    let drm_dir = std::path::Path::new("/sys/class/drm");
    if !drm_dir.exists() {
        return false;
    }

    let Ok(entries) = std::fs::read_dir(drm_dir) else {
        return false;
    };

    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if !name_str.starts_with("card") || name_str.contains('-') {
            continue;
        }

        let uevent_path = entry.path().join("device").join("uevent");
        if let Ok(content) = std::fs::read_to_string(&uevent_path) {
            for line in content.lines() {
                if let Some(driver) = line.strip_prefix("DRIVER=") {
                    if driver == "nvidia" {
                        return true;
                    }
                }
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_wgpu_safe_respects_env_override() {
        temp_env::with_var("TOADSTOOL_WGPU_SAFE", Some("1"), || {
            assert!(is_wgpu_safe());
        });
        temp_env::with_var("TOADSTOOL_WGPU_SAFE", Some("0"), || {
            assert!(!is_wgpu_safe());
        });
    }

    #[test]
    fn is_wgpu_safe_respects_adapter_env() {
        temp_env::with_vars(
            [
                ("TOADSTOOL_WGPU_SAFE", None::<&str>),
                ("TOADSTOOL_GPU_ADAPTER", Some("AMD RX 6950 XT")),
            ],
            || {
                assert!(is_wgpu_safe());
            },
        );
    }

    #[test]
    fn is_wgpu_safe_nvidia_proprietary_unsafe() {
        temp_env::with_vars(
            [
                ("TOADSTOOL_WGPU_SAFE", None::<&str>),
                ("TOADSTOOL_GPU_ADAPTER", Some("NVIDIA RTX 3090")),
            ],
            || {
                assert!(!is_wgpu_safe());
            },
        );
    }

    #[test]
    fn is_wgpu_safe_nvk_is_safe() {
        temp_env::with_vars(
            [
                ("TOADSTOOL_WGPU_SAFE", None::<&str>),
                ("TOADSTOOL_GPU_ADAPTER", Some("NVK Titan V")),
            ],
            || {
                assert!(is_wgpu_safe());
            },
        );
    }

    #[test]
    fn skip_reason_is_informative() {
        let reason = wgpu_skip_reason();
        assert!(reason.contains("NVIDIA"));
        assert!(reason.contains("SIGSEGV"));
    }
}
