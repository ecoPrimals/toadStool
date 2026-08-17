// SPDX-License-Identifier: AGPL-3.0-or-later
//! Host session safety for driver rotation.
//!
//! The handoff pipeline's existing preflight reasons about the *device*:
//! module state, IOMMU groups, kernel build health. This module reasons about
//! the *host* the device lives in — specifically, whether seeding a driver
//! will take down the operator's desktop session.
//!
//! # Why this exists
//!
//! On 2026-08-16 a Titan V rotation on this gate produced three consecutive
//! session kills that were each misdiagnosed, because nothing in the pipeline
//! could observe the mechanism. The actual sequence:
//!
//! 1. nouveau was loaded with `modeset=2`, believed to be headless.
//! 2. `modeset=2` suppresses display *output* but still registers a DRM
//!    device, so `/dev/dri/card0` appeared for the Titan V.
//! 3. Xorg's udev monitor hot-added it, loaded `modesetting`, and brought up
//!    glamor against a GPU that was mid-rotation.
//! 4. Xorg failed an internal assertion and called `abort()` 466ms later.
//! 5. The dying session's scope teardown SIGTERMed the daemon mid-handoff.
//! 6. The resulting rollback unbound the GPU while its graphics engine was
//!    dead, and nouveau page-faulted in `nouveau_ttm_fini`.
//!
//! Every layer after step 2 reported a plausible local failure, which is why
//! the root cause survived two rounds of fixes. The checks here make step 2
//! observable and refusable.

pub mod drm_watch;
pub mod host_state;

pub use drm_watch::{DrmNodeWatch, DrmNodes};
pub use host_state::{DisplayServer, drm_node_claimable};

/// A single reason a rotation is considered unsafe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SafetyConcern {
    /// Short stable identifier, suitable for matching in tests and logs.
    pub code: &'static str,
    /// Operator-facing explanation of the hazard.
    pub detail: String,
    /// The concrete action that clears it.
    pub remedy: &'static str,
}

/// Verdict on whether a rotation may proceed on this host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSafety {
    /// Hazards found. Empty means safe.
    pub concerns: Vec<SafetyConcern>,
    /// Whether a display server was detected at all.
    pub display_server_running: bool,
}

impl SessionSafety {
    /// Evaluate host safety for rotating the driver on `bdf`.
    #[must_use]
    pub fn evaluate(bdf: &str) -> Self {
        let display_server_running = host_state::display_server_running();
        let mut concerns = Vec::new();

        if host_state::is_display_gpu(bdf) {
            concerns.push(SafetyConcern {
                code: "target_is_display_gpu",
                detail: format!(
                    "{bdf} currently exposes a DRM card node and may be driving the display"
                ),
                remedy: "rotate a secondary GPU, or stop the display server first",
            });
        }

        // Hot-add can only hurt us when something is running to do the adding.
        if display_server_running && !host_state::xorg_gpu_hotadd_disabled() {
            concerns.push(SafetyConcern {
                code: "gpu_hotadd_enabled",
                detail:
                    "a display server is running and Xorg GPU hot-add is not disabled; a seeder \
                     that registers a DRM node will have it claimed mid-rotation"
                        .into(),
                remedy: "set AutoAddGPU and AutoBindGPU to off in /etc/X11/xorg.conf.d/, then restart Xorg",
            });
        }

        if host_state::kernel_oops_tainted() {
            concerns.push(SafetyConcern {
                code: "kernel_oops_tainted",
                detail: "the kernel has recorded an oops or BUG since boot; locks may be held by \
                         dead tasks and module refcounts leaked"
                    .into(),
                remedy: "reboot before rotating drivers again",
            });
        }

        Self {
            concerns,
            display_server_running,
        }
    }

    /// Whether the rotation may proceed.
    #[must_use]
    pub fn is_safe(&self) -> bool {
        self.concerns.is_empty()
    }

    /// One-line summary of all concerns, for a halt detail string.
    #[must_use]
    pub fn summary(&self) -> String {
        if self.concerns.is_empty() {
            return "host safe: no display-server hazard, kernel untainted by oops".into();
        }
        self.concerns
            .iter()
            .map(|c| format!("{}: {} (remedy: {})", c.code, c.detail, c.remedy))
            .collect::<Vec<_>>()
            .join("; ")
    }

    /// Whether a specific concern code is present.
    #[must_use]
    pub fn has(&self, code: &str) -> bool {
        self.concerns.iter().any(|c| c.code == code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_verdict_has_no_concerns_and_says_so() {
        let safety = SessionSafety {
            concerns: Vec::new(),
            display_server_running: false,
        };
        assert!(safety.is_safe());
        assert!(safety.summary().contains("host safe"));
    }

    #[test]
    fn summary_includes_every_concern_with_its_remedy() {
        let safety = SessionSafety {
            concerns: vec![
                SafetyConcern {
                    code: "gpu_hotadd_enabled",
                    detail: "hot-add live".into(),
                    remedy: "disable AutoAddGPU",
                },
                SafetyConcern {
                    code: "kernel_oops_tainted",
                    detail: "oops recorded".into(),
                    remedy: "reboot",
                },
            ],
            display_server_running: true,
        };
        assert!(!safety.is_safe());
        assert!(safety.has("gpu_hotadd_enabled"));
        assert!(safety.has("kernel_oops_tainted"));
        assert!(!safety.has("target_is_display_gpu"));

        let summary = safety.summary();
        assert!(summary.contains("disable AutoAddGPU"));
        assert!(summary.contains("reboot"));
    }

    /// Evaluation must never panic regardless of host configuration; a
    /// rotation refusing to start is recoverable, a crash mid-rotation is not.
    #[test]
    fn evaluate_is_total_on_a_nonexistent_device() {
        let safety = SessionSafety::evaluate("0000:ff:ff.9");
        assert!(!safety.has("target_is_display_gpu"));
        let _ = safety.summary();
    }
}
