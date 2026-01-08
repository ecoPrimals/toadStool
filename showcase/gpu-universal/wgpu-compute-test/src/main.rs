use anyhow::{Context, Result};
use wgpu::util::DeviceExt;

const VECTOR_SIZE: usize = 10_000;

// WGSL shader for vector addition (pure Rust, no external compiler!)
const SHADER_SOURCE: &str = r#"
@group(0) @binding(0) var<storage, read> input_a: array<f32>;
@group(0) @binding(1) var<storage, read> input_b: array<f32>;
@group(0) @binding(2) var<storage, read_write> output: array<f32>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index < arrayLength(&input_a)) {
        output[index] = input_a[index] + input_b[index];
    }
}
"#;

fn main() -> Result<()> {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║  wgpu Compute Test - Pure Rust Vector Addition          ║");
    println!("║  Testing WebGPU standard on discovered adapters         ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();

    pollster::block_on(run_test())
}

async fn run_test() -> Result<()> {
    // Create instance
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });

    // Enumerate adapters
    let adapters = instance.enumerate_adapters(wgpu::Backends::all());

    println!("Found {} wgpu adapter(s)", adapters.len());
    println!();

    if adapters.is_empty() {
        anyhow::bail!("No wgpu adapters found");
    }

    // Test each adapter
    for (idx, adapter) in adapters.iter().enumerate() {
        let info = adapter.get_info();
        
        println!("═══════════════════════════════════════════════════════════");
        println!("Adapter {}: {}", idx, info.name);
        println!("  Backend: {:?}", info.backend);
        println!("  Device Type: {:?}", info.device_type);
        println!("  Driver: {}", info.driver);
        println!("═══════════════════════════════════════════════════════════");

        match test_vector_add(adapter).await {
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

    println!("═══════════════════════════════════════════════════════════");
    println!("🎉 wgpu compute test complete!");
    println!("═══════════════════════════════════════════════════════════");
    println!();
    println!("Key Achievement:");
    println!("  ✅ Pure Rust - No external compilers needed");
    println!("  ✅ WGSL - Type-safe shader language");
    println!("  ✅ wgpu - WebGPU standard (cross-platform)");
    println!("  ✅ Safe - Compiler-verified GPU compute");
    println!();
    println!("This is the future of GPU computing in Rust! 🦀");

    Ok(())
}

async fn test_vector_add(adapter: &wgpu::Adapter) -> Result<()> {
    // Request device and queue
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Compute Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
            },
            None,
        )
        .await
        .context("Failed to request device")?;

    println!("  Device created");

    // Prepare input data
    let data_a: Vec<f32> = (0..VECTOR_SIZE).map(|i| i as f32).collect();
    let data_b: Vec<f32> = (0..VECTOR_SIZE).map(|i| (i * 2) as f32).collect();

    // Create buffers
    let buffer_a = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Buffer A"),
        contents: bytemuck::cast_slice(&data_a),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
    });

    let buffer_b = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Buffer B"),
        contents: bytemuck::cast_slice(&data_b),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
    });

    let buffer_c = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Buffer C"),
        size: (VECTOR_SIZE * std::mem::size_of::<f32>()) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    // Create staging buffer for reading results
    let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Staging Buffer"),
        size: (VECTOR_SIZE * std::mem::size_of::<f32>()) as u64,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    println!("  Buffers created and filled");

    // Create shader module
    let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Vector Add Shader"),
        source: wgpu::ShaderSource::Wgsl(SHADER_SOURCE.into()),
    });

    println!("  Shader module created (WGSL compiled at runtime)");

    // Create bind group layout
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Bind Group Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    // Create pipeline layout
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    // Create compute pipeline
    let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Compute Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader_module,
        entry_point: "main",
    });

    println!("  Pipeline created");

    // Create bind group
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Bind Group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer_a.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: buffer_b.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: buffer_c.as_entire_binding(),
            },
        ],
    });

    println!("  Bind group configured");

    // Create command encoder
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Command Encoder"),
    });

    {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Compute Pass"),
            timestamp_writes: None,
        });

        compute_pass.set_pipeline(&compute_pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);

        // Dispatch workgroups
        let workgroup_count = ((VECTOR_SIZE + 255) / 256) as u32;
        compute_pass.dispatch_workgroups(workgroup_count, 1, 1);
    }

    // Copy result to staging buffer
    encoder.copy_buffer_to_buffer(
        &buffer_c,
        0,
        &staging_buffer,
        0,
        (VECTOR_SIZE * std::mem::size_of::<f32>()) as u64,
    );

    // Submit commands
    queue.submit(Some(encoder.finish()));

    println!("  Compute shader executed");

    // Read back results
    let buffer_slice = staging_buffer.slice(..);
    let (sender, receiver) = futures_intrusive::channel::shared::oneshot_channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        sender.send(result).unwrap();
    });

    device.poll(wgpu::Maintain::Wait);

    receiver
        .receive()
        .await
        .context("Failed to receive map result")?
        .context("Failed to map buffer")?;

    let data = buffer_slice.get_mapped_range();
    let result: Vec<f32> = bytemuck::cast_slice(&data).to_vec();
    drop(data);
    staging_buffer.unmap();

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

    Ok(())
}

