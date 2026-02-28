//! Quantum computing platform detection.

use super::probe;
use super::types::PlatformType;
use toadstool::ToadStoolResult;

const QUANTUM_FRAMEWORKS: &[(&str, &str)] = &[
    ("qiskit", "IBM Qiskit"),
    ("cirq", "Google Cirq"),
    ("forest", "Rigetti Forest"),
    ("braket", "Amazon Braket"),
    ("pennylane", "PennyLane"),
];

/// Detect quantum computing platforms.
#[allow(clippy::unused_async)] // Sync probe; async for API consistency with SubstrateDetector
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

    if std::env::var("IBM_QUANTUM_TOKEN").is_ok() {
        platforms.push(PlatformType::Quantum {
            framework: "IBM Quantum Network".to_string(),
            simulator: false,
        });
    }
    if std::env::var("RIGETTI_QCS_TOKEN").is_ok() {
        platforms.push(PlatformType::Quantum {
            framework: "Rigetti QCS".to_string(),
            simulator: false,
        });
    }

    Ok(platforms)
}
