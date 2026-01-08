use anyhow::{Context, Result};
use ash::vk;
use std::ffi::CStr;
use std::mem;

const VECTOR_SIZE: usize = 10_000;

fn main() -> Result<()> {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║  Vulkan Compute Test - Vector Addition                  ║");
    println!("║  Testing actual compute execution on discovered GPUs    ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();

    // Initialize Vulkan
    let entry = ash::Entry::linked();
    
    // Create instance
    let app_info = vk::ApplicationInfo::builder()
        .application_name(CStr::from_bytes_with_nul(b"Vulkan Compute Test\0").unwrap())
        .application_version(vk::make_api_version(0, 1, 0, 0))
        .engine_name(CStr::from_bytes_with_nul(b"ToadStool\0").unwrap())
        .engine_version(vk::make_api_version(0, 1, 0, 0))
        .api_version(vk::API_VERSION_1_0);

    let instance_create_info = vk::InstanceCreateInfo::builder()
        .application_info(&app_info);

    let instance = unsafe {
        entry
            .create_instance(&instance_create_info, None)
            .context("Failed to create Vulkan instance")?
    };

    // Enumerate physical devices
    let physical_devices = unsafe {
        instance
            .enumerate_physical_devices()
            .context("Failed to enumerate physical devices")?
    };

    println!("Found {} Vulkan device(s)", physical_devices.len());
    println!();

    // Test each device
    for (idx, &physical_device) in physical_devices.iter().enumerate() {
        let props = unsafe { instance.get_physical_device_properties(physical_device) };
        let device_name = unsafe {
            CStr::from_ptr(props.device_name.as_ptr())
                .to_string_lossy()
                .into_owned()
        };

        println!("═══════════════════════════════════════════════════════════");
        println!("Device {}: {}", idx, device_name);
        println!("═══════════════════════════════════════════════════════════");

        match test_vector_add(&instance, physical_device, idx) {
            Ok(()) => {
                println!("✅ SUCCESS: Vector addition executed correctly!");
                println!();
            }
            Err(e) => {
                println!("❌ FAILED: {}", e);
                println!();
            }
        }
    }

    // Cleanup
    unsafe {
        instance.destroy_instance(None);
    }

    println!("═══════════════════════════════════════════════════════════");
    println!("🎉 Vulkan compute test complete!");
    println!("═══════════════════════════════════════════════════════════");

    Ok(())
}

fn test_vector_add(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    device_index: usize,
) -> Result<()> {
    // Find compute queue family
    let queue_families =
        unsafe { instance.get_physical_device_queue_family_properties(physical_device) };

    let compute_queue_family = queue_families
        .iter()
        .enumerate()
        .find(|(_, props)| props.queue_flags.contains(vk::QueueFlags::COMPUTE))
        .map(|(idx, _)| idx as u32)
        .context("No compute queue family found")?;

    println!("  Queue family: {} (compute)", compute_queue_family);

    // Create logical device
    let queue_priorities = [1.0];
    let queue_create_info = vk::DeviceQueueCreateInfo::builder()
        .queue_family_index(compute_queue_family)
        .queue_priorities(&queue_priorities);

    let device_create_info = vk::DeviceCreateInfo::builder()
        .queue_create_infos(std::slice::from_ref(&queue_create_info));

    let device = unsafe {
        instance
            .create_device(physical_device, &device_create_info, None)
            .context("Failed to create logical device")?
    };

    // Get queue
    let queue = unsafe { device.get_device_queue(compute_queue_family, 0) };

    // Create buffers
    let buffer_size = (VECTOR_SIZE * mem::size_of::<f32>()) as vk::DeviceSize;

    // Helper function to find memory type
    let find_memory_type =
        |type_filter: u32, properties: vk::MemoryPropertyFlags| -> Result<u32> {
            let mem_properties =
                unsafe { instance.get_physical_device_memory_properties(physical_device) };

            for i in 0..mem_properties.memory_type_count {
                if (type_filter & (1 << i)) != 0
                    && mem_properties.memory_types[i as usize]
                        .property_flags
                        .contains(properties)
                {
                    return Ok(i);
                }
            }

            anyhow::bail!("Failed to find suitable memory type")
        };

    // Create input buffer A
    let buffer_create_info = vk::BufferCreateInfo::builder()
        .size(buffer_size)
        .usage(vk::BufferUsageFlags::STORAGE_BUFFER)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);

    let buffer_a = unsafe {
        device
            .create_buffer(&buffer_create_info, None)
            .context("Failed to create buffer A")?
    };

    let mem_requirements = unsafe { device.get_buffer_memory_requirements(buffer_a) };
    let memory_type_index = find_memory_type(
        mem_requirements.memory_type_bits,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    )?;

    let alloc_info = vk::MemoryAllocateInfo::builder()
        .allocation_size(mem_requirements.size)
        .memory_type_index(memory_type_index);

    let buffer_a_memory = unsafe {
        device
            .allocate_memory(&alloc_info, None)
            .context("Failed to allocate buffer A memory")?
    };

    unsafe {
        device
            .bind_buffer_memory(buffer_a, buffer_a_memory, 0)
            .context("Failed to bind buffer A memory")?;
    }

    // Create input buffer B (same process)
    let buffer_b = unsafe {
        device
            .create_buffer(&buffer_create_info, None)
            .context("Failed to create buffer B")?
    };

    let buffer_b_memory = unsafe {
        device
            .allocate_memory(&alloc_info, None)
            .context("Failed to allocate buffer B memory")?
    };

    unsafe {
        device
            .bind_buffer_memory(buffer_b, buffer_b_memory, 0)
            .context("Failed to bind buffer B memory")?;
    }

    // Create output buffer C
    let buffer_c = unsafe {
        device
            .create_buffer(&buffer_create_info, None)
            .context("Failed to create buffer C")?
    };

    let buffer_c_memory = unsafe {
        device
            .allocate_memory(&alloc_info, None)
            .context("Failed to allocate buffer C memory")?
    };

    unsafe {
        device
            .bind_buffer_memory(buffer_c, buffer_c_memory, 0)
            .context("Failed to bind buffer C memory")?;
    }

    // Fill input buffers
    let data_a: Vec<f32> = (0..VECTOR_SIZE).map(|i| i as f32).collect();
    let data_b: Vec<f32> = (0..VECTOR_SIZE).map(|i| (i * 2) as f32).collect();

    unsafe {
        let ptr = device
            .map_memory(
                buffer_a_memory,
                0,
                buffer_size,
                vk::MemoryMapFlags::empty(),
            )
            .context("Failed to map buffer A memory")? as *mut f32;
        std::ptr::copy_nonoverlapping(data_a.as_ptr(), ptr, VECTOR_SIZE);
        device.unmap_memory(buffer_a_memory);

        let ptr = device
            .map_memory(
                buffer_b_memory,
                0,
                buffer_size,
                vk::MemoryMapFlags::empty(),
            )
            .context("Failed to map buffer B memory")? as *mut f32;
        std::ptr::copy_nonoverlapping(data_b.as_ptr(), ptr, VECTOR_SIZE);
        device.unmap_memory(buffer_b_memory);
    }

    println!("  Buffers created and filled");

    // Create compute shader module
    let shader_code = include_bytes!("../shaders/vector_add.spv");
    let shader_module_create_info = vk::ShaderModuleCreateInfo::builder()
        .code(bytemuck::cast_slice(shader_code));

    let shader_module = unsafe {
        device
            .create_shader_module(&shader_module_create_info, None)
            .context("Failed to create shader module")?
    };

    println!("  Shader module created");

    // Create descriptor set layout
    let descriptor_set_layout_bindings = [
        vk::DescriptorSetLayoutBinding::builder()
            .binding(0)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::COMPUTE)
            .build(),
        vk::DescriptorSetLayoutBinding::builder()
            .binding(1)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::COMPUTE)
            .build(),
        vk::DescriptorSetLayoutBinding::builder()
            .binding(2)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::COMPUTE)
            .build(),
    ];

    let descriptor_set_layout_create_info =
        vk::DescriptorSetLayoutCreateInfo::builder().bindings(&descriptor_set_layout_bindings);

    let descriptor_set_layout = unsafe {
        device
            .create_descriptor_set_layout(&descriptor_set_layout_create_info, None)
            .context("Failed to create descriptor set layout")?
    };

    // Create pipeline layout
    let pipeline_layout_create_info = vk::PipelineLayoutCreateInfo::builder()
        .set_layouts(std::slice::from_ref(&descriptor_set_layout));

    let pipeline_layout = unsafe {
        device
            .create_pipeline_layout(&pipeline_layout_create_info, None)
            .context("Failed to create pipeline layout")?
    };

    // Create compute pipeline
    let entry_point = CStr::from_bytes_with_nul(b"main\0").unwrap();
    let shader_stage_create_info = vk::PipelineShaderStageCreateInfo::builder()
        .stage(vk::ShaderStageFlags::COMPUTE)
        .module(shader_module)
        .name(entry_point);

    let compute_pipeline_create_info = vk::ComputePipelineCreateInfo::builder()
        .stage(*shader_stage_create_info)
        .layout(pipeline_layout);

    let compute_pipeline = unsafe {
        device
            .create_compute_pipelines(
                vk::PipelineCache::null(),
                std::slice::from_ref(&compute_pipeline_create_info),
                None,
            )
            .map_err(|(_, e)| e)
            .context("Failed to create compute pipeline")?[0]
    };

    println!("  Pipeline created");

    // Create descriptor pool
    let pool_sizes = [vk::DescriptorPoolSize::builder()
        .ty(vk::DescriptorType::STORAGE_BUFFER)
        .descriptor_count(3)
        .build()];

    let descriptor_pool_create_info = vk::DescriptorPoolCreateInfo::builder()
        .pool_sizes(&pool_sizes)
        .max_sets(1);

    let descriptor_pool = unsafe {
        device
            .create_descriptor_pool(&descriptor_pool_create_info, None)
            .context("Failed to create descriptor pool")?
    };

    // Allocate descriptor set
    let descriptor_set_allocate_info = vk::DescriptorSetAllocateInfo::builder()
        .descriptor_pool(descriptor_pool)
        .set_layouts(std::slice::from_ref(&descriptor_set_layout));

    let descriptor_sets = unsafe {
        device
            .allocate_descriptor_sets(&descriptor_set_allocate_info)
            .context("Failed to allocate descriptor sets")?
    };
    let descriptor_set = descriptor_sets[0];

    // Update descriptor set
    let buffer_info_a = [vk::DescriptorBufferInfo::builder()
        .buffer(buffer_a)
        .offset(0)
        .range(vk::WHOLE_SIZE)
        .build()];

    let buffer_info_b = [vk::DescriptorBufferInfo::builder()
        .buffer(buffer_b)
        .offset(0)
        .range(vk::WHOLE_SIZE)
        .build()];

    let buffer_info_c = [vk::DescriptorBufferInfo::builder()
        .buffer(buffer_c)
        .offset(0)
        .range(vk::WHOLE_SIZE)
        .build()];

    let write_descriptor_sets = [
        vk::WriteDescriptorSet::builder()
            .dst_set(descriptor_set)
            .dst_binding(0)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .buffer_info(&buffer_info_a)
            .build(),
        vk::WriteDescriptorSet::builder()
            .dst_set(descriptor_set)
            .dst_binding(1)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .buffer_info(&buffer_info_b)
            .build(),
        vk::WriteDescriptorSet::builder()
            .dst_set(descriptor_set)
            .dst_binding(2)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .buffer_info(&buffer_info_c)
            .build(),
    ];

    unsafe {
        device.update_descriptor_sets(&write_descriptor_sets, &[]);
    }

    println!("  Descriptor sets configured");

    // Create command pool
    let command_pool_create_info = vk::CommandPoolCreateInfo::builder()
        .queue_family_index(compute_queue_family);

    let command_pool = unsafe {
        device
            .create_command_pool(&command_pool_create_info, None)
            .context("Failed to create command pool")?
    };

    // Allocate command buffer
    let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::builder()
        .command_pool(command_pool)
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_buffer_count(1);

    let command_buffers = unsafe {
        device
            .allocate_command_buffers(&command_buffer_allocate_info)
            .context("Failed to allocate command buffers")?
    };
    let command_buffer = command_buffers[0];

    // Record command buffer
    let begin_info = vk::CommandBufferBeginInfo::builder()
        .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

    unsafe {
        device
            .begin_command_buffer(command_buffer, &begin_info)
            .context("Failed to begin command buffer")?;

        device.cmd_bind_pipeline(
            command_buffer,
            vk::PipelineBindPoint::COMPUTE,
            compute_pipeline,
        );

        device.cmd_bind_descriptor_sets(
            command_buffer,
            vk::PipelineBindPoint::COMPUTE,
            pipeline_layout,
            0,
            &[descriptor_set],
            &[],
        );

        // Dispatch compute work (256 threads per workgroup)
        let workgroup_count = ((VECTOR_SIZE + 255) / 256) as u32;
        device.cmd_dispatch(command_buffer, workgroup_count, 1, 1);

        device
            .end_command_buffer(command_buffer)
            .context("Failed to end command buffer")?;
    }

    println!("  Command buffer recorded");

    // Submit command buffer
    let submit_info = vk::SubmitInfo::builder()
        .command_buffers(&[command_buffer]);

    unsafe {
        device
            .queue_submit(queue, &[*submit_info], vk::Fence::null())
            .context("Failed to submit queue")?;

        device
            .queue_wait_idle(queue)
            .context("Failed to wait for queue")?;
    }

    println!("  Compute shader executed");

    // Read back results
    let result: Vec<f32> = unsafe {
        let ptr = device
            .map_memory(
                buffer_c_memory,
                0,
                buffer_size,
                vk::MemoryMapFlags::empty(),
            )
            .context("Failed to map buffer C memory")? as *const f32;
        let slice = std::slice::from_raw_parts(ptr, VECTOR_SIZE);
        let result = slice.to_vec();
        device.unmap_memory(buffer_c_memory);
        result
    };

    // Verify results
    let mut errors = 0;
    for i in 0..VECTOR_SIZE {
        let expected = data_a[i] + data_b[i];
        let actual = result[i];
        if (expected - actual).abs() > 1e-5 {
            if errors < 10 {
                println!(
                    "  ❌ Mismatch at index {}: expected {}, got {}",
                    i, expected, actual
                );
            }
            errors += 1;
        }
    }

    if errors == 0 {
        println!("  ✓ All {} elements verified correct!", VECTOR_SIZE);
    } else {
        anyhow::bail!("{} elements had incorrect values", errors);
    }

    // Cleanup
    unsafe {
        device.destroy_command_pool(command_pool, None);
        device.destroy_descriptor_pool(descriptor_pool, None);
        device.destroy_pipeline(compute_pipeline, None);
        device.destroy_pipeline_layout(pipeline_layout, None);
        device.destroy_descriptor_set_layout(descriptor_set_layout, None);
        device.destroy_shader_module(shader_module, None);
        device.destroy_buffer(buffer_c, None);
        device.free_memory(buffer_c_memory, None);
        device.destroy_buffer(buffer_b, None);
        device.free_memory(buffer_b_memory, None);
        device.destroy_buffer(buffer_a, None);
        device.free_memory(buffer_a_memory, None);
        device.destroy_device(None);
    }

    Ok(())
}
