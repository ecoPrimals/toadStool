// SPDX-License-Identifier: AGPL-3.0-only
//! Probing utilities for substrate detection.
//!
//! Provides command existence checks, Python package detection, and OS introspection.

use std::process::Command;

use toadstool::ToadStoolResult;

/// Check if a command exists in PATH.
#[must_use]
pub fn command_exists(command: &str) -> bool {
    Command::new("which")
        .arg(command)
        .output()
        .map(|output| output.status.success())
        .unwrap_or_default()
}

/// Check if a Python package is importable.
#[must_use]
pub fn python_package_exists(package: &str) -> bool {
    if std::process::Command::new("python3")
        .args(["-c", &format!("import {package}")])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return true;
    }
    std::process::Command::new("python")
        .args(["-c", &format!("import {package}")])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Detect Linux distribution from /etc/os-release.
pub fn detect_linux_distribution() -> ToadStoolResult<String> {
    if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
        for line in content.lines() {
            if line.starts_with("ID=") {
                return Ok(line.trim_start_matches("ID=").trim_matches('"').to_string());
            }
        }
    }
    Ok("linux".to_string())
}
