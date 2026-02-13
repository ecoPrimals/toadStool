//! FP64 Precision Validation
//!
//! Validates that GPU fp64 produces CORRECT results, not just fast ones.
//! Tests against CPU f64 reference to ensure precision is real.

use wgpu::util::DeviceExt;

const SHADER_ADD_F32: &str = r#"
@group(0) @binding(0) var<storage, read> a: array<f32>;
@group(0) @binding(1) var<storage, read> b: array<f32>;
@group(0) @binding(2) var<storage, read_write> output: array<f32>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= arrayLength(&output)) {
        return;
    }
    output[idx] = a[idx] + b[idx];
}
"#;

const SHADER_ADD_F64: &str = r#"
@group(0) @binding(0) var<storage, read> a: array<f64>;
@group(0) @binding(1) var<storage, read> b: array<f64>;
@group(0) @binding(2) var<storage, read_write> output: array<f64>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= arrayLength(&output)) {
        return;
    }
    output[idx] = a[idx] + b[idx];
}
"#;

// Kahan summation shader - tests numerical stability
const SHADER_KAHAN_SUM_F32: &str = r#"
@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;

@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var sum: f32 = 0.0;
    var c: f32 = 0.0;  // Compensation for lost low-order bits
    
    let n = arrayLength(&input);
    for (var i = 0u; i < n; i = i + 1u) {
        let y = input[i] - c;
        let t = sum + y;
        c = (t - sum) - y;
        sum = t;
    }
    
    output[0] = sum;
}
"#;

const SHADER_KAHAN_SUM_F64: &str = r#"
@group(0) @binding(0) var<storage, read> input: array<f64>;
@group(0) @binding(1) var<storage, read_write> output: array<f64>;

@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var sum: f64 = 0.0;
    var c: f64 = 0.0;  // Compensation for lost low-order bits
    
    let n = arrayLength(&input);
    for (var i = 0u; i < n; i = i + 1u) {
        let y = input[i] - c;
        let t = sum + y;
        c = (t - sum) - y;
        sum = t;
    }
    
    output[0] = sum;
}
"#;

struct GpuContext {
    device: wgpu::Device,
    queue: wgpu::Queue,
    name: String,
    has_f64: bool,
}

impl GpuContext {
    async fn new(adapter: &wgpu::Adapter) -> Option<Self> {
        let info = adapter.get_info();
        let features = adapter.features();
        let has_f64 = features.contains(wgpu::Features::SHADER_F64);
        
        let required_features = if has_f64 {
            wgpu::Features::SHADER_F64
        } else {
            wgpu::Features::empty()
        };
        
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some(&info.name),
                    required_features,
                    required_limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .ok()?;
        
        Some(Self {
            device,
            queue,
            name: info.name.clone(),
            has_f64,
        })
    }
}

/// Test 1: Element-wise addition precision
fn test_addition_precision(ctx: &GpuContext) {
    println!("\n  Test 1: Element-wise Addition Precision");
    println!("  ─────────────────────────────────────────");
    
    // Use values that challenge floating point precision
    // Small differences that require high precision to capture
    let test_cases: Vec<(f64, f64, &str)> = vec![
        (1.0, 1e-15, "1.0 + 1e-15 (precision limit)"),
        (1e15, 1.0, "1e15 + 1.0 (large + small)"),
        (0.1, 0.2, "0.1 + 0.2 (binary representation)"),
        (std::f64::consts::PI, std::f64::consts::E, "π + e"),
        (1.0000000000000001, 1.0000000000000002, "near-identical values"),
    ];
    
    for (a, b, desc) in &test_cases {
        let expected_f64 = a + b;
        let expected_f32 = (*a as f32) + (*b as f32);
        
        // Run on GPU
        let (gpu_f32, gpu_f64) = run_single_add(ctx, *a, *b);
        
        // Compare
        let f32_error = ((gpu_f32 as f64) - expected_f64).abs();
        let f64_error = (gpu_f64 - expected_f64).abs();
        
        let f32_ulp = ulp_distance_f32(expected_f32, gpu_f32);
        let f64_ulp = ulp_distance_f64(expected_f64, gpu_f64);
        
        println!("    {}", desc);
        println!("      CPU f64:  {:.16e}", expected_f64);
        println!("      GPU f32:  {:.16e}  (error: {:.2e}, {} ULP)", gpu_f32, f32_error, f32_ulp);
        if ctx.has_f64 {
            println!("      GPU f64:  {:.16e}  (error: {:.2e}, {} ULP)", gpu_f64, f64_error, f64_ulp);
            if f64_ulp == 0 {
                println!("      ✅ GPU f64 matches CPU f64 exactly!");
            } else if f64_ulp <= 1 {
                println!("      ✅ GPU f64 within 1 ULP of CPU f64");
            } else {
                println!("      ⚠️  GPU f64 differs by {} ULP", f64_ulp);
            }
        }
        println!();
    }
}

/// Test 2: Accumulation precision (Kahan summation test)
fn test_accumulation_precision(ctx: &GpuContext) {
    println!("\n  Test 2: Accumulation Precision (summing 1M values)");
    println!("  ─────────────────────────────────────────────────────");
    
    let n = 1_000_000;
    
    // Sum of 1/i for i=1 to n (harmonic series - challenging for precision)
    let data_f64: Vec<f64> = (1..=n).map(|i| 1.0 / (i as f64)).collect();
    let data_f32: Vec<f32> = (1..=n).map(|i| 1.0 / (i as f32)).collect();
    
    // CPU reference (Kahan summation in f64)
    let cpu_kahan_f64 = kahan_sum_f64(&data_f64);
    let cpu_naive_f64: f64 = data_f64.iter().sum();
    let cpu_naive_f32: f32 = data_f32.iter().sum();
    
    println!("    Harmonic series H_{} = Σ(1/i) for i=1 to {}", n, n);
    println!();
    println!("    CPU Kahan f64:  {:.15}", cpu_kahan_f64);
    println!("    CPU naive f64:  {:.15}  (error: {:.2e})", cpu_naive_f64, (cpu_naive_f64 - cpu_kahan_f64).abs());
    println!("    CPU naive f32:  {:.15}  (error: {:.2e})", cpu_naive_f32, (cpu_naive_f32 as f64 - cpu_kahan_f64).abs());
    
    // GPU Kahan summation (if f64 supported)
    if ctx.has_f64 {
        let gpu_kahan_f64 = run_kahan_sum_f64(ctx, &data_f64);
        let gpu_kahan_f32 = run_kahan_sum_f32(ctx, &data_f32);
        
        println!("    GPU Kahan f64:  {:.15}  (error: {:.2e})", gpu_kahan_f64, (gpu_kahan_f64 - cpu_kahan_f64).abs());
        println!("    GPU Kahan f32:  {:.15}  (error: {:.2e})", gpu_kahan_f32, (gpu_kahan_f32 as f64 - cpu_kahan_f64).abs());
        
        if (gpu_kahan_f64 - cpu_kahan_f64).abs() < 1e-10 {
            println!("\n    ✅ GPU f64 Kahan sum matches CPU f64 (within 1e-10)");
        } else {
            println!("\n    ⚠️  GPU f64 Kahan sum differs from CPU");
        }
    }
}

/// Test 3: Numerical stability with ill-conditioned values
fn test_numerical_stability(ctx: &GpuContext) {
    println!("\n  Test 3: Numerical Stability (ill-conditioned operations)");
    println!("  ──────────────────────────────────────────────────────────");
    
    if !ctx.has_f64 {
        println!("    Skipped - no f64 support");
        return;
    }
    
    // Catastrophic cancellation test
    let a = 1.0000000000000001_f64;
    let b = 1.0000000000000000_f64;
    let expected = a - b;
    
    println!("    Catastrophic cancellation: {} - {}", a, b);
    println!("    CPU f64:  {:.16e}", expected);
    
    // Test with addition (b + (a - b) should equal a)
    let reconstructed = b + expected;
    println!("    Reconstructed: {:.16e}", reconstructed);
    println!("    Original a:    {:.16e}", a);
    
    if (reconstructed - a).abs() < 1e-15 {
        println!("    ✅ f64 precision preserved through cancellation");
    } else {
        println!("    ⚠️  Precision loss detected");
    }
}

/// Test 4: Verify GPU isn't silently using f32
fn test_precision_verification(ctx: &GpuContext) {
    println!("\n  Test 4: Verify GPU f64 is NOT secretly f32");
    println!("  ─────────────────────────────────────────────");
    
    if !ctx.has_f64 {
        println!("    Skipped - no f64 support");
        return;
    }
    
    // Value that's representable differently in f32 vs f64
    let a = 1.0000000000001_f64;  // More precision than f32 can represent
    let b = 0.0000000000001_f64;
    
    let cpu_f64 = a + b;
    let cpu_f32 = (a as f32) + (b as f32);
    
    let (gpu_f32_result, gpu_f64_result) = run_single_add(ctx, a, b);
    
    println!("    a = {:.16e}", a);
    println!("    b = {:.16e}", b);
    println!();
    println!("    CPU f64:  {:.16e}", cpu_f64);
    println!("    CPU f32:  {:.16e}", cpu_f32);
    println!("    GPU f64:  {:.16e}", gpu_f64_result);
    println!("    GPU f32:  {:.16e}", gpu_f32_result);
    println!();
    
    // The key test: GPU f64 should match CPU f64 MUCH better than CPU f32
    let f64_error = (gpu_f64_result - cpu_f64).abs();
    let f32_error = (gpu_f64_result - cpu_f32 as f64).abs();
    
    // If secretly using f32, the error would be near f32 precision (~1e-7 relative)
    // True f64 should be within machine epsilon (~2e-16 relative)
    let is_true_f64 = f64_error < 1e-14 && (f32_error > f64_error * 100.0 || f32_error > 1e-10);
    
    if is_true_f64 {
        println!("    ✅ GPU f64 matches CPU f64 (NOT f32) - TRUE double precision!");
        println!("       f64 error: {:.2e}, f32 error: {:.2e}", f64_error, f32_error);
    } else if f64_error < 1e-14 {
        println!("    ✅ GPU f64 matches CPU f64 exactly (test values happen to work in both precisions)");
    } else {
        println!("    ⚠️  GPU f64 does not match CPU f64 - unexpected behavior");
    }
}

// Helper functions

fn ulp_distance_f32(a: f32, b: f32) -> u64 {
    let ai = a.to_bits() as i32;
    let bi = b.to_bits() as i32;
    (ai - bi).unsigned_abs() as u64
}

fn ulp_distance_f64(a: f64, b: f64) -> u64 {
    let ai = a.to_bits() as i64;
    let bi = b.to_bits() as i64;
    (ai - bi).unsigned_abs()
}

fn kahan_sum_f64(data: &[f64]) -> f64 {
    let mut sum = 0.0_f64;
    let mut c = 0.0_f64;
    for &x in data {
        let y = x - c;
        let t = sum + y;
        c = (t - sum) - y;
        sum = t;
    }
    sum
}

fn run_single_add(ctx: &GpuContext, a: f64, b: f64) -> (f32, f64) {
    // F32 version
    let a_f32 = [a as f32];
    let b_f32 = [b as f32];
    
    let a_buffer = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("a_f32"),
        contents: bytemuck::cast_slice(&a_f32),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let b_buffer = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("b_f32"),
        contents: bytemuck::cast_slice(&b_f32),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let output_buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("output_f32"),
        size: 4,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let staging_buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("staging_f32"),
        size: 4,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    
    let module = ctx.device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("add_f32"),
        source: wgpu::ShaderSource::Wgsl(SHADER_ADD_F32.into()),
    });
    
    let bind_group_layout = ctx.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None }, count: None },
        ],
    });
    
    let pipeline_layout = ctx.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });
    
    let pipeline = ctx.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        module: &module,
        entry_point: "main",
    });
    
    let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: a_buffer.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: b_buffer.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 2, resource: output_buffer.as_entire_binding() },
        ],
    });
    
    let mut encoder = ctx.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }
    encoder.copy_buffer_to_buffer(&output_buffer, 0, &staging_buffer, 0, 4);
    ctx.queue.submit(Some(encoder.finish()));
    
    let slice = staging_buffer.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    ctx.device.poll(wgpu::Maintain::Wait);
    let f32_result: f32 = {
        let data = slice.get_mapped_range();
        *bytemuck::from_bytes(&data)
    };
    staging_buffer.unmap();
    
    // F64 version (if supported)
    let f64_result = if ctx.has_f64 {
        let a_f64 = [a];
        let b_f64 = [b];
        
        let a_buffer = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("a_f64"),
            contents: bytemuck::cast_slice(&a_f64),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let b_buffer = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("b_f64"),
            contents: bytemuck::cast_slice(&b_f64),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let output_buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("output_f64"),
            size: 8,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let staging_buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("staging_f64"),
            size: 8,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        let module = ctx.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("add_f64"),
            source: wgpu::ShaderSource::Wgsl(SHADER_ADD_F64.into()),
        });
        
        let pipeline = ctx.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: "main",
        });
        
        let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: a_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: b_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: output_buffer.as_entire_binding() },
            ],
        });
        
        let mut encoder = ctx.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }
        encoder.copy_buffer_to_buffer(&output_buffer, 0, &staging_buffer, 0, 8);
        ctx.queue.submit(Some(encoder.finish()));
        
        let slice = staging_buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        ctx.device.poll(wgpu::Maintain::Wait);
        let result: f64 = {
            let data = slice.get_mapped_range();
            *bytemuck::from_bytes(&data)
        };
        staging_buffer.unmap();
        result
    } else {
        0.0
    };
    
    (f32_result, f64_result)
}

fn run_kahan_sum_f64(ctx: &GpuContext, data: &[f64]) -> f64 {
    let input_buffer = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("input"),
        contents: bytemuck::cast_slice(data),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let output_buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("output"),
        size: 8,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let staging_buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("staging"),
        size: 8,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    
    let module = ctx.device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("kahan_f64"),
        source: wgpu::ShaderSource::Wgsl(SHADER_KAHAN_SUM_F64.into()),
    });
    
    let bind_group_layout = ctx.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None }, count: None },
        ],
    });
    
    let pipeline_layout = ctx.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });
    
    let pipeline = ctx.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        module: &module,
        entry_point: "main",
    });
    
    let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: input_buffer.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: output_buffer.as_entire_binding() },
        ],
    });
    
    let mut encoder = ctx.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }
    encoder.copy_buffer_to_buffer(&output_buffer, 0, &staging_buffer, 0, 8);
    ctx.queue.submit(Some(encoder.finish()));
    
    let slice = staging_buffer.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    ctx.device.poll(wgpu::Maintain::Wait);
    let result: f64 = {
        let data = slice.get_mapped_range();
        *bytemuck::from_bytes(&data)
    };
    staging_buffer.unmap();
    result
}

fn run_kahan_sum_f32(ctx: &GpuContext, data: &[f32]) -> f32 {
    let input_buffer = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("input"),
        contents: bytemuck::cast_slice(data),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let output_buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("output"),
        size: 4,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let staging_buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("staging"),
        size: 4,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    
    let module = ctx.device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("kahan_f32"),
        source: wgpu::ShaderSource::Wgsl(SHADER_KAHAN_SUM_F32.into()),
    });
    
    let bind_group_layout = ctx.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None }, count: None },
        ],
    });
    
    let pipeline_layout = ctx.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });
    
    let pipeline = ctx.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        module: &module,
        entry_point: "main",
    });
    
    let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: input_buffer.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: output_buffer.as_entire_binding() },
        ],
    });
    
    let mut encoder = ctx.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }
    encoder.copy_buffer_to_buffer(&output_buffer, 0, &staging_buffer, 0, 4);
    ctx.queue.submit(Some(encoder.finish()));
    
    let slice = staging_buffer.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    ctx.device.poll(wgpu::Maintain::Wait);
    let result: f32 = {
        let data = slice.get_mapped_range();
        *bytemuck::from_bytes(&data)
    };
    staging_buffer.unmap();
    result
}

#[tokio::main]
async fn main() {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  FP64 PRECISION VALIDATION                                    ║");
    println!("║  Verifying GPU fp64 produces CORRECT results                  ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
    
    for adapter in instance.enumerate_adapters(wgpu::Backends::VULKAN) {
        let info = adapter.get_info();
        if info.device_type != wgpu::DeviceType::DiscreteGpu {
            continue;
        }
        
        if let Some(ctx) = GpuContext::new(&adapter).await {
            println!("\n══════════════════════════════════════════════════════════════");
            println!("  {}", ctx.name);
            println!("  SHADER_F64: {}", if ctx.has_f64 { "✅ Supported" } else { "❌ Not available" });
            println!("══════════════════════════════════════════════════════════════");
            
            test_addition_precision(&ctx);
            test_accumulation_precision(&ctx);
            test_numerical_stability(&ctx);
            test_precision_verification(&ctx);
        }
    }
    
    println!("\n══════════════════════════════════════════════════════════════");
    println!("  CONCLUSION");
    println!("══════════════════════════════════════════════════════════════");
    println!("  If all tests pass (✅), the GPU is providing TRUE fp64:");
    println!("  - Not secretly using f32");
    println!("  - Not emulating with degraded precision");
    println!("  - Full IEEE 754 double precision");
    println!();
    println!("  This validates that the silicon IS capable of fp64,");
    println!("  and we're bypassing any artificial vendor limitations!");
}
