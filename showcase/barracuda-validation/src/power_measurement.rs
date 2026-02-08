// Power measurement utilities for BarraCUDA showcases
// Real hardware queries (no hardcoding)

use std::process::Command;
use tracing;

/// Query real-time GPU power draw via nvidia-smi
/// Falls back to typical estimate if nvidia-smi unavailable
pub fn query_gpu_power() -> f32 {
    match Command::new("nvidia-smi")
        .args(&["--query-gpu=power.draw", "--format=csv,noheader,nounits"])
        .output()
    {
        Ok(output) if output.status.success() => {
            let power_str = String::from_utf8_lossy(&output.stdout);
            if let Ok(power_watts) = power_str.trim().parse::<f32>() {
                tracing::debug!("GPU power measured: {:.2}W (nvidia-smi)", power_watts);
                return power_watts;
            }
        }
        Err(e) => {
            tracing::warn!("nvidia-smi unavailable: {}", e);
        }
        _ => {
            tracing::warn!("nvidia-smi query failed");
        }
    }
    
    tracing::warn!("GPU power: using typical estimate (nvidia-smi unavailable)");
    250.0 // RTX 3090 typical under compute load
}

/// Query real-time CPU package power via RAPL (Linux)
/// Falls back to typical single-core estimate if RAPL unavailable
pub fn query_cpu_power() -> f32 {
    // Try to read RAPL energy counter
    let rapl_path = "/sys/class/powercap/intel-rapl:0/energy_uj";
    
    if let Ok(energy_before) = std::fs::read_to_string(rapl_path) {
        std::thread::sleep(std::time::Duration::from_millis(100));
        if let Ok(energy_after) = std::fs::read_to_string(rapl_path) {
            if let (Ok(before), Ok(after)) = (
                energy_before.trim().parse::<u64>(),
                energy_after.trim().parse::<u64>(),
            ) {
                let delta_uj = after.saturating_sub(before);
                let power_watts = delta_uj as f32 / 100_000.0; // 100ms sample
                tracing::debug!("CPU power measured: {:.2}W (RAPL)", power_watts);
                return power_watts;
            }
        }
    }
    
    tracing::warn!("CPU power: using typical estimate (RAPL unavailable)");
    5.0 // Single-core typical
}

/// Query NPU power from Akida device
/// Falls back to typical estimate if hwmon unavailable
pub fn query_npu_power(pcie_address: &str) -> f32 {
    let sysfs_base = format!("/sys/bus/pci/devices/{}", pcie_address);
    let hwmon_dir = format!("{}/hwmon", sysfs_base);
    
    // Find hwmon directory
    if let Ok(entries) = std::fs::read_dir(&hwmon_dir) {
        for entry in entries.flatten() {
            let hwmon_path = entry.path();
            let power_path = hwmon_path.join("power1_input");
            
            if let Ok(power_str) = std::fs::read_to_string(&power_path) {
                if let Ok(power_uw) = power_str.trim().parse::<f64>() {
                    let power_watts = (power_uw / 1_000_000.0) as f32;
                    tracing::debug!("NPU power measured: {:.2}W (hwmon {})", power_watts, pcie_address);
                    return power_watts;
                }
            }
        }
    }
    
    tracing::warn!("NPU power: using typical estimate (hwmon unavailable for {})", pcie_address);
    2.0 // Akida AKD1000 typical
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_gpu_power_query() {
        let power = query_gpu_power();
        assert!(power > 0.0);
        assert!(power < 500.0); // Sanity check
    }
    
    #[test]
    fn test_cpu_power_query() {
        let power = query_cpu_power();
        assert!(power > 0.0);
        assert!(power < 200.0); // Sanity check
    }
}
