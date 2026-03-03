// SPDX-License-Identifier: AGPL-3.0-or-later
//! Language runtime detection (Python, Node, Java, Go, Rust).

use super::probe;
use super::types::PlatformType;
use toadstool::ToadStoolResult;

const RUNTIMES: &[(&str, &str)] = &[
    ("python", "Python"),
    ("python3", "Python3"),
    ("node", "NodeJS"),
    ("java", "Java"),
    ("go", "Go"),
    ("rustc", "Rust"),
];

/// Detect language runtimes.
#[allow(clippy::unused_async)] // Sync probe; async for API consistency with SubstrateDetector
pub async fn detect() -> ToadStoolResult<Vec<PlatformType>> {
    let mut platforms = Vec::new();

    for (command, name) in RUNTIMES {
        if probe::command_exists(command) {
            platforms.push(PlatformType::Language {
                name: (*name).to_string(),
                command: (*command).to_string(),
            });
        }
    }

    Ok(platforms)
}
