//! Integration with ToadStool's UniversalSubstrate

use anyhow::Result;
use toadstool_distributed::universal::types::neuromorphic::NeuromorphicPlatform;

use crate::{AkidaBoard, AkidaMesh};

/// Convert Akida board to NeuromorphicPlatform
pub fn to_neuromorphic_platform(board: &AkidaBoard) -> NeuromorphicPlatform {
    NeuromorphicPlatform::NeuromorphicChip {
        chip_name: board.chip_name.clone(),
        manufacturer: "BrainChip".to_string(),
        core_count: board.npu_count,
        neuron_count_per_core: 1024, // AKD1000 spec: ~1K neurons per NPU
        synapse_count_per_core: 10_000, // AKD1000 spec: ~10K synapses per NPU
        power_consumption_mw: board.power_watts * 1000.0,
    }
}

/// Register Akida mesh with UniversalSubstrate
pub async fn register_with_substrate(mesh: &AkidaMesh) -> Result<()> {
    tracing::info!(
        "Registering {} Akida board(s) with UniversalSubstrate",
        mesh.boards.len()
    );
    
    // Convert each board to neuromorphic platform
    let platforms: Vec<_> = mesh
        .boards
        .iter()
        .map(to_neuromorphic_platform)
        .collect();
    
    // In production, this would call the actual UniversalSubstrate API
    // For now, we'll just log the registration
    for (i, platform) in platforms.iter().enumerate() {
        tracing::info!("Registered board {}: {:?}", i, platform);
    }
    
    Ok(())
}

/// Query workload compatibility for Akida
pub fn is_compatible_workload(workload_type: &str) -> bool {
    matches!(
        workload_type,
        "classification"
            | "pattern_matching"
            | "intent_recognition"
            | "kmer_filtering"
            | "motion_detection"
            | "anomaly_detection"
            | "event_processing"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AkidaBoard, BoardHealth};
    use std::path::PathBuf;
    
    #[test]
    fn test_platform_conversion() {
        let board = AkidaBoard {
            index: 0,
            pcie_address: "0000:01:00.0".to_string(),
            device_path: PathBuf::from("/dev/akida0"),
            chip_name: "Akida AKD1000".to_string(),
            npu_count: 80,
            memory_bytes: 10 * 1024 * 1024,
            power_watts: 1.2,
            temperature_celsius: 42.0,
            pcie_generation: 2,
            pcie_lanes: 4,
            health: BoardHealth::Healthy,
            node_name: None,
        };
        
        let platform = to_neuromorphic_platform(&board);
        
        match platform {
            NeuromorphicPlatform::NeuromorphicChip {
                chip_name,
                manufacturer,
                core_count,
                power_consumption_mw,
                ..
            } => {
                assert_eq!(chip_name, "Akida AKD1000");
                assert_eq!(manufacturer, "BrainChip");
                assert_eq!(core_count, 80);
                assert_eq!(power_consumption_mw, 1200.0);
            }
            _ => panic!("Wrong platform type"),
        }
    }
    
    #[test]
    fn test_workload_compatibility() {
        assert!(is_compatible_workload("classification"));
        assert!(is_compatible_workload("kmer_filtering"));
        assert!(is_compatible_workload("intent_recognition"));
        assert!(!is_compatible_workload("matrix_multiplication"));
        assert!(!is_compatible_workload("ray_tracing"));
    }
}

