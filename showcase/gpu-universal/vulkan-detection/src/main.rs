// SPDX-License-Identifier: AGPL-3.0-or-later
use anyhow::Result;
use ash::{vk, Entry};
use std::ffi::CStr;

fn main() -> Result<()> {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  Vulkan Vendor-Agnostic Detection Test (Rust)               ║");
    println!("║  Verifying Both NVIDIA + AMD via Vulkan                     ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    // Create Vulkan entry point
    let entry = unsafe { Entry::load()? };

    // Create Vulkan instance
    let app_info = vk::ApplicationInfo::builder()
        .application_name(CStr::from_bytes_with_nul(b"VulkanDetection\0")?)
        .application_version(vk::make_api_version(0, 1, 0, 0))
        .engine_name(CStr::from_bytes_with_nul(b"ToadStool\0")?)
        .engine_version(vk::make_api_version(0, 1, 0, 0))
        .api_version(vk::make_api_version(0, 1, 0, 0));

    let create_info = vk::InstanceCreateInfo::builder()
        .application_info(&app_info);

    let instance = unsafe { entry.create_instance(&create_info, None)? };

    // Enumerate physical devices
    let physical_devices = unsafe { instance.enumerate_physical_devices()? };

    println!("🔍 Discovered {} Vulkan device(s):", physical_devices.len());
    println!();

    let mut nvidia_found = false;
    let mut amd_found = false;

    for (idx, &physical_device) in physical_devices.iter().enumerate() {
        // Get device properties
        let props = unsafe { instance.get_physical_device_properties(physical_device) };
        
        // Extract device name
        let device_name = unsafe {
            CStr::from_ptr(props.device_name.as_ptr())
                .to_string_lossy()
                .to_string()
        };

        // Get device type
        let device_type = match props.device_type {
            vk::PhysicalDeviceType::DISCRETE_GPU => "Discrete GPU",
            vk::PhysicalDeviceType::INTEGRATED_GPU => "Integrated GPU",
            vk::PhysicalDeviceType::VIRTUAL_GPU => "Virtual GPU",
            vk::PhysicalDeviceType::CPU => "CPU",
            _ => "Other",
        };

        // Get vendor name
        let vendor = match props.vendor_id {
            0x1002 => "AMD",
            0x10DE => "NVIDIA",
            0x8086 => "Intel",
            _ => "Unknown",
        };

        // Get memory heaps
        let mem_props = unsafe { instance.get_physical_device_memory_properties(physical_device) };
        let mut total_device_memory = 0u64;
        for i in 0..mem_props.memory_heap_count {
            let heap = mem_props.memory_heaps[i as usize];
            if heap.flags.contains(vk::MemoryHeapFlags::DEVICE_LOCAL) {
                total_device_memory += heap.size;
            }
        }

        // Get queue families
        let queue_families = unsafe {
            instance.get_physical_device_queue_family_properties(physical_device)
        };
        
        let mut compute_queue_count = 0;
        for queue_family in &queue_families {
            if queue_family.queue_flags.contains(vk::QueueFlags::COMPUTE) {
                compute_queue_count += queue_family.queue_count;
            }
        }

        println!("──────────────────────────────────────────────────────────────");
        println!("Device {}: {}", idx, device_name);
        println!("──────────────────────────────────────────────────────────────");
        println!("  Type:            {}", device_type);
        println!("  Vendor:          {} ({:#x})", vendor, props.vendor_id);
        println!("  Device ID:       {:#x}", props.device_id);
        println!("  API Version:     {}.{}.{}",
            vk::api_version_major(props.api_version),
            vk::api_version_minor(props.api_version),
            vk::api_version_patch(props.api_version));
        println!("  Driver Version:  {}.{}.{}",
            vk::api_version_major(props.driver_version),
            vk::api_version_minor(props.driver_version),
            vk::api_version_patch(props.driver_version));
        println!("  Device Memory:   {:.1} GB", total_device_memory as f64 / 1e9);
        println!("  Compute Queues:  {}", compute_queue_count);
        println!();

        // Track vendor detection
        if vendor == "NVIDIA" || device_name.to_lowercase().contains("nvidia") {
            nvidia_found = true;
        }
        if vendor == "AMD" || device_name.to_lowercase().contains("amd") || device_name.contains("gfx") {
            amd_found = true;
        }
    }

    // Cleanup
    unsafe {
        instance.destroy_instance(None);
    }

    println!("══════════════════════════════════════════════════════════════");
    println!("📊 SUMMARY");
    println!("══════════════════════════════════════════════════════════════");
    println!();
    println!("  Total Devices:   {}", physical_devices.len());
    println!();
    println!("  NVIDIA GPU:      {}", if nvidia_found { "✅ DETECTED" } else { "❌ NOT FOUND" });
    println!("  AMD GPU:         {}", if amd_found { "✅ DETECTED" } else { "❌ NOT FOUND" });
    println!();

    if nvidia_found && amd_found {
        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║  🎉 SUCCESS: BOTH GPUS DETECTED VIA VULKAN                  ║");
        println!("║  Vulkan Compute: READY FOR BOTH VENDORS ✅                  ║");
        println!("╚══════════════════════════════════════════════════════════════╝");
    } else if nvidia_found || amd_found {
        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║  ⚠️  PARTIAL SUCCESS: One GPU detected via Vulkan           ║");
        println!("║  Check hardware and Vulkan drivers                          ║");
        println!("╚══════════════════════════════════════════════════════════════╝");
    } else {
        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║  ❌ NO GPUS DETECTED VIA VULKAN                             ║");
        println!("║  Check Vulkan installation and drivers                      ║");
        println!("╚══════════════════════════════════════════════════════════════╝");
    }

    println!();
    println!("💡 Key Insight:");
    println!("   Vulkan provides vendor-agnostic compute across NVIDIA, AMD,");
    println!("   and Intel GPUs with a single API and shader language (SPIR-V).");
    println!();

    Ok(())
}
