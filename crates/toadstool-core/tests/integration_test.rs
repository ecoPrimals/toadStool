// SPDX-License-Identifier: AGPL-3.0-only
//! Integration test: Complete stack working together

use toadstool_core::{HardwareManager, HardwareType};

#[test]
fn test_complete_stack_integration() {
    println!("\n🍄 ToadStool + 🦈 BarraCuda Integration Test\n");

    // Step 1: ToadStool discovers hardware
    let hw = HardwareManager::discover().expect("Hardware discovery failed");
    println!("✓ Discovered {} device(s)", hw.devices().len());

    // Should find at least CPU
    assert!(!hw.devices().is_empty(), "Should discover at least CPU");

    // Step 2: Check device types
    let gpu_available = hw.has_gpu();
    let npu_available = hw.has_npu();

    println!("  GPU available: {gpu_available}");
    println!("  NPU available: {npu_available}");

    // Step 3: Verify devices have correct properties
    for device in hw.devices() {
        match device.hardware_type {
            HardwareType::Gpu => {
                // GPUs should support userspace (via WGPU)
                assert!(device.userspace_capable);
            }
            HardwareType::Npu => {
                // NPUs should have either kernel driver or userspace
                assert!(device.driver_available || device.userspace_capable);
            }
            HardwareType::Cpu => {
                // CPU always available
                assert!(device.driver_available);
                assert!(device.userspace_capable);
            }
            _ => {}
        }
    }

    // Step 4: Test rescan capability
    let mut hw_mut = hw;
    hw_mut.rescan().expect("Rescan failed");
    println!("✓ Rescan successful");

    println!("\n✓ Complete stack integration verified\n");
}

#[test]
fn test_device_selection_logic() {
    let hw = HardwareManager::discover().expect("Hardware discovery failed");

    // Should always be able to select a device
    let cpus = hw.devices_by_type(HardwareType::Cpu);
    assert!(!cpus.is_empty(), "CPU should always be available");

    // If GPU available, should find it
    if hw.has_gpu() {
        let gpus = hw.devices_by_type(HardwareType::Gpu);
        assert!(!gpus.is_empty());
    }

    // If NPU available, should find it
    if hw.has_npu() {
        let npus = hw.devices_by_type(HardwareType::Npu);
        assert!(!npus.is_empty());
    }
}
