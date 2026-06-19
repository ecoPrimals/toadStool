// SPDX-License-Identifier: AGPL-3.0-or-later
//! Quantum computing platform detection.

use super::probe;
use super::types::PlatformType;
use toadstool::ToadStoolResult;
use toadstool_common::interned_strings::socket_env;

const QUANTUM_FRAMEWORKS: &[(&str, &str)] = &[
    ("qiskit", "IBM Qiskit"),
    ("cirq", "Google Cirq"),
    ("forest", "Rigetti Forest"),
    ("braket", "Amazon Braket"),
    ("pennylane", "PennyLane"),
];

/// Detect quantum computing platforms.
#[expect(
    clippy::unused_async,
    reason = "async signature required by trait/interface"
)] // Sync probe; async for API consistency with SubstrateDetector
pub async fn detect() -> ToadStoolResult<Vec<PlatformType>> {
    let mut platforms = Vec::new();

    for (command, name) in QUANTUM_FRAMEWORKS {
        if probe::command_exists(command) || probe::python_package_exists(command) {
            platforms.push(PlatformType::Quantum {
                framework: (*name).to_string(),
                simulator: true,
            });
        }
    }

    if std::env::var(socket_env::IBM_QUANTUM_TOKEN).is_ok() {
        platforms.push(PlatformType::Quantum {
            framework: "IBM Quantum Network".to_string(),
            simulator: false,
        });
    }
    if std::env::var(socket_env::RIGETTI_QCS_TOKEN).is_ok() {
        platforms.push(PlatformType::Quantum {
            framework: "Rigetti QCS".to_string(),
            simulator: false,
        });
    }

    Ok(platforms)
}
