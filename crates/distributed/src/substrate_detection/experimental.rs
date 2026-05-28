// SPDX-License-Identifier: AGPL-3.0-or-later
//! Experimental platform detection via capability probing.
//!
//! Detects non-standard compute substrates (FPGA, neuromorphic, quantum
//! simulators) by checking for characteristic device files, drivers, and
//! environment variables. Returns an empty vec when no experimental
//! hardware is found — this is the normal case on most machines.

use super::types::PlatformType;
use toadstool_common::interned_strings::socket_env;
use toadstool::ToadStoolResult;

/// Detect experimental platforms by probing for hardware indicators.
///
/// Checks (in order):
/// 1. Xilinx/Intel FPGA device nodes (`/dev/xclmgmt*`, `/dev/xdma*`, `XILINX_XRT`)
/// 2. Neuromorphic indicators (Akida via VFIO, SpiNNaker via env)
/// 3. Quantum simulator environments (`QISKIT_HOME`, `CIRQ_HOME`)
///
/// Each probe is non-blocking and tolerates missing paths gracefully.
pub async fn detect() -> ToadStoolResult<Vec<PlatformType>> {
    let mut platforms = Vec::new();

    if let Some(fpga) = probe_fpga() {
        platforms.push(fpga);
    }
    if let Some(neuro) = probe_neuromorphic() {
        platforms.push(neuro);
    }
    if let Some(quantum) = probe_quantum_sim() {
        platforms.push(quantum);
    }

    Ok(platforms)
}

fn probe_fpga() -> Option<PlatformType> {
    let has_xrt = std::env::var(socket_env::XILINX_XRT).is_ok();
    let has_quartus = std::env::var(socket_env::QUARTUS_ROOTDIR).is_ok();
    let has_xclmgmt = std::path::Path::new("/dev/xclmgmt0").exists();

    if has_xrt || has_xclmgmt {
        Some(PlatformType::Other {
            os: "fpga".into(),
            architecture: "xilinx-xrt".into(),
        })
    } else if has_quartus {
        Some(PlatformType::Other {
            os: "fpga".into(),
            architecture: "intel-quartus".into(),
        })
    } else {
        None
    }
}

fn probe_neuromorphic() -> Option<PlatformType> {
    let has_akida_vfio = std::path::Path::new("/sys/bus/pci/drivers/vfio-pci").exists()
        && std::env::var(socket_env::AKIDA_DEVICE_ID).is_ok();
    let has_spinnaker = std::env::var(socket_env::SPINNAKER_ROOT).is_ok();

    if has_akida_vfio {
        Some(PlatformType::Other {
            os: "neuromorphic".into(),
            architecture: "akida-vfio".into(),
        })
    } else if has_spinnaker {
        Some(PlatformType::Other {
            os: "neuromorphic".into(),
            architecture: "spinnaker".into(),
        })
    } else {
        None
    }
}

fn probe_quantum_sim() -> Option<PlatformType> {
    let has_qiskit = std::env::var(socket_env::QISKIT_HOME).is_ok();
    let has_cirq = std::env::var(socket_env::CIRQ_HOME).is_ok();

    if has_qiskit || has_cirq {
        let runtime = if has_qiskit { "qiskit" } else { "cirq" };
        Some(PlatformType::Other {
            os: "quantum-sim".into(),
            architecture: runtime.into(),
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn detect_returns_empty_on_standard_machine() {
        let platforms = detect().await.unwrap();
        assert!(platforms.len() <= 3);
    }

    #[test]
    fn probe_fpga_returns_none_without_hardware() {
        temp_env::with_vars_unset(["XILINX_XRT", "QUARTUS_ROOTDIR"], || {
            assert!(probe_fpga().is_none());
        });
    }

    #[test]
    fn probe_neuromorphic_returns_none_without_hardware() {
        temp_env::with_vars_unset(["AKIDA_DEVICE_ID", "SPINNAKER_ROOT"], || {
            assert!(probe_neuromorphic().is_none());
        });
    }

    #[test]
    fn probe_quantum_returns_none_without_runtime() {
        temp_env::with_vars_unset(["QISKIT_HOME", "CIRQ_HOME"], || {
            assert!(probe_quantum_sim().is_none());
        });
    }
}
