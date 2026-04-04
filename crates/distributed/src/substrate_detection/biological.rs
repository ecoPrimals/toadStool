// SPDX-License-Identifier: AGPL-3.0-only
//! Biological computing platform detection.

use super::probe;
use super::types::PlatformType;
use toadstool::ToadStoolResult;

const BIO_TOOLS: &[(&str, &str)] = &[
    ("blast", "BLAST Sequence Analysis"),
    ("clustalw", "ClustalW Multiple Sequence Alignment"),
    ("biopython", "BioPython Framework"),
    ("openmm", "OpenMM Molecular Dynamics"),
    ("gromacs", "GROMACS Molecular Simulation"),
    ("amber", "AMBER MD Suite"),
];

/// Detect biological computing platforms.
#[expect(
    clippy::unused_async,
    reason = "async signature required by trait/interface"
)] // Sync probe; async for API consistency with SubstrateDetector
pub async fn detect() -> ToadStoolResult<Vec<PlatformType>> {
    let mut platforms = Vec::new();

    for (tool, description) in BIO_TOOLS {
        if probe::command_exists(tool) || probe::python_package_exists(tool) {
            platforms.push(PlatformType::BiologicalComputing {
                platform: (*description).to_string(),
                simulation: true,
            });
        }
    }

    if probe::command_exists("opentrons") {
        platforms.push(PlatformType::BiologicalComputing {
            platform: "Opentrons Lab Automation".to_string(),
            simulation: false,
        });
    }

    if std::env::var("TWIST_BIOSCIENCE_API_KEY").is_ok() {
        platforms.push(PlatformType::BiologicalComputing {
            platform: "Twist Bioscience DNA Synthesis".to_string(),
            simulation: false,
        });
    }

    Ok(platforms)
}
