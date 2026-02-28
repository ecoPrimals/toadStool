//! Neuromorphic computing platform detection.

use std::fs;

use super::probe;
use super::types::PlatformType;
use toadstool::ToadStoolResult;

const NEURO_FRAMEWORKS: &[(&str, &str)] = &[
    ("brian2", "Brian2 Spiking Network Simulator"),
    ("nest", "NEST Simulator"),
    ("neuron", "NEURON Simulation Environment"),
    ("nengo", "Nengo Neural Engineering Framework"),
    ("spynnaker", "SpiNNaker Toolkit"),
    ("loihi", "Intel Loihi SDK"),
];

const FPGA_TOOLS: &[(&str, &str)] = &[
    ("vivado", "Xilinx Vivado"),
    ("quartus", "Intel Quartus Prime"),
    ("diamond", "Lattice Diamond"),
];

/// Detect neuromorphic computing platforms.
pub async fn detect() -> ToadStoolResult<Vec<PlatformType>> {
    let mut platforms = Vec::new();

    for (framework, description) in NEURO_FRAMEWORKS {
        if probe::python_package_exists(framework) {
            platforms.push(PlatformType::NeuromorphicComputing {
                platform: (*description).to_string(),
                hardware: false,
            });
        }
    }

    if let Ok(devices) = fs::read_dir("/dev") {
        for entry in devices.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if name.contains("loihi") || name.contains("neuromorphic") {
                    platforms.push(PlatformType::NeuromorphicComputing {
                        platform: "Intel Loihi Hardware".to_string(),
                        hardware: true,
                    });
                    break;
                }
            }
        }
    }

    for (tool, description) in FPGA_TOOLS {
        if probe::command_exists(tool) {
            platforms.push(PlatformType::NeuromorphicComputing {
                platform: format!("{description} FPGA Neuromorphic"),
                hardware: true,
            });
        }
    }

    Ok(platforms)
}
