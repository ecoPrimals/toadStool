//! 2D Convolution (Conv2D) Operation Demo
//!
//! Demonstrates:
//! - Conv2D: The backbone of Convolutional Neural Networks (CNNs)
//! - Stride and padding effects
//! - Multi-channel convolutions
//! - Edge detection filters (practical example)
//!
//! Conv2D is THE operation for computer vision and image processing!

use anyhow::Result;
use std::collections::HashMap;
use toadstool_runtime_universal::runtime::UniversalRuntime;
use toadstool_runtime_universal::types::*;

#[tokio::main]
async fn main() -> Result<()> {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║  Universal Runtime: 2D Convolution Demo                 ║");
    println!("║  barraCuda Phase 1 - THE Computer Vision Operation      ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();

    // Discover compute units
    println!("🔍 Discovering compute units...");
    let runtime = UniversalRuntime::discover().await?;
    println!("✅ Found {} compute unit(s)", runtime.num_units());
    println!();

    // Demo 1: Simple 3x3 Convolution (Identity Filter)
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Demo 1: Simple 3x3 Identity Convolution");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    println!("Conv2D: The backbone of CNNs");
    println!("Used in: Image classification, object detection, segmentation");
    println!();

    // Input: 1 batch, 1 channel, 5x5 image
    #[rustfmt::skip]
    let input = vec![
        1.0, 2.0, 3.0, 4.0, 5.0,
        6.0, 7.0, 8.0, 9.0, 10.0,
        11.0, 12.0, 13.0, 14.0, 15.0,
        16.0, 17.0, 18.0, 19.0, 20.0,
        21.0, 22.0, 23.0, 24.0, 25.0,
    ];

    // 3x3 Identity kernel (center=1, rest=0)
    #[rustfmt::skip]
    let kernel = vec![
        0.0, 0.0, 0.0,
        0.0, 1.0, 0.0,
        0.0, 0.0, 0.0,
    ];

    println!("Input (1×1×5×5):");
    for row in 0..5 {
        print!("  [");
        for col in 0..5 {
            print!("{:>4.0}", input[row * 5 + col]);
            if col < 4 {
                print!(", ");
            }
        }
        println!("]");
    }
    println!();

    println!("Kernel (1×1×3×3) - Identity:");
    println!("  [0, 0, 0]");
    println!("  [0, 1, 0]  ← Center picks middle value");
    println!("  [0, 0, 0]");
    println!();

    let conv_workload = Workload {
        operation: OperationType::Conv,
        data_type: DataType::F32,
        num_operations: 3 * 3 * 3 * 3, // batch * out_ch * out_h * out_w * kernel_h * kernel_w * in_ch
        required_memory: (input.len() + kernel.len() + 9) * std::mem::size_of::<f32>(),
        input: WorkloadData::F32Conv2D {
            input: input.clone(),
            kernel: kernel.clone(),
            bias: None,
            batch_size: 1,
            in_channels: 1,
            height: 5,
            width: 5,
            out_channels: 1,
            kernel_h: 3,
            kernel_w: 3,
            stride: 1,
            padding: 0,
        },
        params: WorkloadParams {
            params: HashMap::new(),
        },
    };

    let result = runtime.execute_optimal(conv_workload).await?;

    if let WorkloadData::F32Matrix(output, batch, rest) = &result.data {
        let out_size = rest / batch;
        let out_dim = (out_size as f32).sqrt() as usize; // Assuming square output

        println!("Output (1×1×3×3) - stride=1, padding=0:");
        println!("  Input 5x5 → Output 3x3 (no padding)");
        for row in 0..out_dim {
            print!("  [");
            for col in 0..out_dim {
                print!("{:>4.0}", output[row * out_dim + col]);
                if col < out_dim - 1 {
                    print!(", ");
                }
            }
            println!("]");
        }
        println!();

        println!("Identity kernel preserves center values! ✅");
        println!("Output[1,1] = Input[2,2] = 13.0");
    }

    println!();
    println!("Executed on: {}", result.metadata.unit_name);
    println!("Duration:    {:?}", result.metadata.duration);
    println!();

    // Demo 2: Edge Detection (Sobel Filter)
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Demo 2: Edge Detection with Sobel Filter");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    println!("Sobel filter: Detects vertical edges");
    println!("Used in: Feature extraction, edge detection, image preprocessing");
    println!();

    // Simple pattern with vertical edge
    #[rustfmt::skip]
    let edge_input = vec![
        0.0, 0.0, 0.0, 255.0, 255.0,
        0.0, 0.0, 0.0, 255.0, 255.0,
        0.0, 0.0, 0.0, 255.0, 255.0,
        0.0, 0.0, 0.0, 255.0, 255.0,
        0.0, 0.0, 0.0, 255.0, 255.0,
    ];

    // Sobel vertical edge detector
    #[rustfmt::skip]
    let sobel_kernel = vec![
        -1.0, 0.0, 1.0,
        -2.0, 0.0, 2.0,
        -1.0, 0.0, 1.0,
    ];

    println!("Input (black→white edge):");
    for row in 0..5 {
        print!("  [");
        for col in 0..5 {
            let val = edge_input[row * 5 + col];
            print!("{:>5.0}", val);
            if col < 4 {
                print!(", ");
            }
        }
        println!("]");
    }
    println!();

    println!("Sobel Kernel (vertical edge):");
    println!("  [-1,  0,  1]");
    println!("  [-2,  0,  2]");
    println!("  [-1,  0,  1]");
    println!();

    let sobel_workload = Workload {
        operation: OperationType::Conv,
        data_type: DataType::F32,
        num_operations: 3 * 3 * 3 * 3,
        required_memory: (edge_input.len() + sobel_kernel.len() + 9) * std::mem::size_of::<f32>(),
        input: WorkloadData::F32Conv2D {
            input: edge_input.clone(),
            kernel: sobel_kernel.clone(),
            bias: None,
            batch_size: 1,
            in_channels: 1,
            height: 5,
            width: 5,
            out_channels: 1,
            kernel_h: 3,
            kernel_w: 3,
            stride: 1,
            padding: 0,
        },
        params: WorkloadParams {
            params: HashMap::new(),
        },
    };

    let sobel_result = runtime.execute_optimal(sobel_workload).await?;

    if let WorkloadData::F32Matrix(output, _, rest) = &sobel_result.data {
        let out_dim = (*rest as f32).sqrt() as usize;

        println!("Edge Detection Output (3x3):");
        for row in 0..out_dim {
            print!("  [");
            for col in 0..out_dim {
                print!("{:>6.0}", output[row * out_dim + col]);
                if col < out_dim - 1 {
                    print!(", ");
                }
            }
            println!("]");
        }
        println!();

        println!("High values = strong vertical edge detected! 🎯");
        println!("Column 2 (at edge) has highest response!");
    }

    println!();
    println!("Executed on: {}", sobel_result.metadata.unit_name);
    println!("Duration:    {:?}", sobel_result.metadata.duration);
    println!();

    // Demo 3: Stride and Padding
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Demo 3: Stride and Padding Effects");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    println!("Stride: Controls output size (stride=2 → half size)");
    println!("Padding: Preserves spatial dimensions");
    println!();

    #[rustfmt::skip]
    let stride_input = vec![
        1.0, 2.0, 3.0, 4.0,
        5.0, 6.0, 7.0, 8.0,
        9.0, 10.0, 11.0, 12.0,
        13.0, 14.0, 15.0, 16.0,
    ];

    #[rustfmt::skip]
    let avg_kernel = vec![
        0.25, 0.25,
        0.25, 0.25,
    ];

    println!("Input (4x4):");
    for row in 0..4 {
        print!("  [");
        for col in 0..4 {
            print!("{:>4.0}", stride_input[row * 4 + col]);
            if col < 3 {
                print!(", ");
            }
        }
        println!("]");
    }
    println!();

    println!("Kernel (2x2 average filter):");
    println!("  [0.25, 0.25]");
    println!("  [0.25, 0.25]  ← Averages 2x2 region");
    println!();

    println!("Testing stride=2 (downsampling):");

    let stride_workload = Workload {
        operation: OperationType::Conv,
        data_type: DataType::F32,
        num_operations: 2 * 2 * 2 * 2,
        required_memory: (stride_input.len() + avg_kernel.len() + 4) * std::mem::size_of::<f32>(),
        input: WorkloadData::F32Conv2D {
            input: stride_input.clone(),
            kernel: avg_kernel.clone(),
            bias: None,
            batch_size: 1,
            in_channels: 1,
            height: 4,
            width: 4,
            out_channels: 1,
            kernel_h: 2,
            kernel_w: 2,
            stride: 2, // Stride=2 → output is half size
            padding: 0,
        },
        params: WorkloadParams {
            params: HashMap::new(),
        },
    };

    let stride_result = runtime.execute_optimal(stride_workload).await?;

    if let WorkloadData::F32Matrix(output, _, rest) = &stride_result.data {
        let out_dim = (*rest as f32).sqrt() as usize;

        println!("Output (2x2) - Input 4x4 → Output 2x2:");
        for row in 0..out_dim {
            print!("  [");
            for col in 0..out_dim {
                print!("{:>6.2}", output[row * out_dim + col]);
                if col < out_dim - 1 {
                    print!(", ");
                }
            }
            println!("]");
        }
        println!();

        println!("Stride=2 → Downsampled to 2x2 (quarter size)! ✅");
        println!("Each output is average of 2x2 input region");
    }

    println!();
    println!("Executed on: {}", stride_result.metadata.unit_name);
    println!("Duration:    {:?}", stride_result.metadata.duration);
    println!();

    // Demo 4: Multi-Channel Convolution (RGB → Features)
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Demo 4: Multi-Channel RGB Convolution");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    println!("Typical CNN first layer:");
    println!("  • Input: RGB image (3 channels)");
    println!("  • Kernel: Multiple feature detectors");
    println!("  • Output: Feature maps (e.g., 8 channels)");
    println!();

    // Small RGB image: 3 channels, 3x3
    #[rustfmt::skip]
    let rgb_input = vec![
        // Red channel
        255.0, 128.0, 0.0,
        200.0, 100.0, 50.0,
        150.0, 75.0, 25.0,
        // Green channel
        0.0, 128.0, 255.0,
        50.0, 100.0, 200.0,
        25.0, 75.0, 150.0,
        // Blue channel
        128.0, 128.0, 128.0,
        100.0, 100.0, 100.0,
        75.0, 75.0, 75.0,
    ];

    // 2 output channels, each with 3 input channels, 2x2 kernel
    // Channel 0: Red detector, Channel 1: Green detector
    #[rustfmt::skip]
    let rgb_kernel = vec![
        // Output channel 0 (red detector)
        // Red input
        1.0, 0.0,
        0.0, 0.0,
        // Green input
        0.0, 0.0,
        0.0, 0.0,
        // Blue input
        0.0, 0.0,
        0.0, 0.0,
        // Output channel 1 (green detector)
        // Red input
        0.0, 0.0,
        0.0, 0.0,
        // Green input
        0.0, 1.0,
        0.0, 0.0,
        // Blue input
        0.0, 0.0,
        0.0, 0.0,
    ];

    println!("Input: 3 channels (RGB), 3x3 each");
    println!("Kernel: 2 output channels, 3 input channels, 2x2 each");
    println!();

    let rgb_workload = Workload {
        operation: OperationType::Conv,
        data_type: DataType::F32,
        num_operations: 2 * 2 * 2 * 3 * 2 * 2,
        required_memory: (rgb_input.len() + rgb_kernel.len() + 8) * std::mem::size_of::<f32>(),
        input: WorkloadData::F32Conv2D {
            input: rgb_input.clone(),
            kernel: rgb_kernel.clone(),
            bias: None,
            batch_size: 1,
            in_channels: 3, // RGB
            height: 3,
            width: 3,
            out_channels: 2, // 2 feature detectors
            kernel_h: 2,
            kernel_w: 2,
            stride: 1,
            padding: 0,
        },
        params: WorkloadParams {
            params: HashMap::new(),
        },
    };

    let rgb_result = runtime.execute_optimal(rgb_workload).await?;

    if let WorkloadData::F32Matrix(output, _, _rest) = &rgb_result.data {
        println!("✅ Multi-channel convolution complete!");
        println!("  Input: 1×3×3×3 (batch, channels, H, W)");
        println!("  Output: 1×2×2×2 (batch, out_channels, out_H, out_W)");
        println!();

        println!("Output Channel 0 (red detector):");
        println!("  [{:>6.0}, {:>6.0}]", output[0], output[1]);
        println!("  [{:>6.0}, {:>6.0}]", output[2], output[3]);
        println!();

        println!("Output Channel 1 (green detector):");
        println!("  [{:>6.0}, {:>6.0}]", output[4], output[5]);
        println!("  [{:>6.0}, {:>6.0}]", output[6], output[7]);
        println!();

        println!("Each output channel responds to different input features! ✅");
    }

    println!();
    println!("Executed on: {}", rgb_result.metadata.unit_name);
    println!("Duration:    {:?}", rgb_result.metadata.duration);
    println!();

    // Pattern Observations
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🎓 Pattern Observations (barraCuda Learning)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    println!("Conv2D:");
    println!("  • Parallelism: Batch + Output channel parallel (excellent!)");
    println!("  • Pattern: 7 nested loops (batch, out_ch, out_y, out_x, in_ch, ky, kx)");
    println!("  • Compute: O(B * C_out * H_out * W_out * C_in * K_h * K_w)");
    println!("  • Memory: Strided access (can be optimized with im2col)");
    println!("  • CPU: Good with batch/channel parallelism");
    println!("  • GPU: EXCELLENT (shared memory for kernel reuse)");
    println!();

    println!("Stride and Padding:");
    println!("  • Stride=1: Preserves resolution");
    println!("  • Stride=2: Downsamples (2x smaller)");
    println!("  • Padding=0: Output shrinks by (kernel-1)");
    println!("  • Padding=(kernel-1)/2: Preserves size (\"same\" padding)");
    println!();

    println!("Use Cases (Conv2D is EVERYWHERE in CV):");
    println!("  1. Image classification: ResNet, VGG, Inception");
    println!("  2. Object detection: YOLO, Faster R-CNN, SSD");
    println!("  3. Semantic segmentation: U-Net, FCN, DeepLab");
    println!("  4. Image generation: StyleGAN, Pix2Pix");
    println!("  5. Feature extraction: Edge detection, texture analysis");
    println!();

    println!("CNN Architecture Pattern:");
    println!("  Conv2D → ReLU → Conv2D → ReLU → MaxPool → ...");
    println!("  All operations now in barraCuda! 🎯");
    println!();

    println!("Key Insights:");
    println!("  1. Conv2D is THE operation for computer vision");
    println!("     • 70-90% of compute time in CNNs");
    println!("     • Optimization = model optimization");
    println!();
    println!("  2. Spatial locality is critical");
    println!("     • Input region reused K_h × K_w times");
    println!("     • Kernel reused H_out × W_out times");
    println!("     • Cache optimization potential!");
    println!();
    println!("  3. Multi-channel convolution = feature learning");
    println!("     • Each output channel detects different features");
    println!("     • Deep networks learn hierarchical representations");
    println!();
    println!("  4. Hyperparameters matter");
    println!("     • Kernel size: 3×3 (modern), 5×5, 7×7 (older)");
    println!("     • Stride: 1 (preserve), 2 (downsample)");
    println!("     • Padding: Same (preserve size), valid (shrink)");
    println!();

    println!("barraCuda Opportunities:");
    println!("  • Im2col optimization: Transform to MatMul (reuse MatMul tiling!)");
    println!("  • Winograd: Fast convolution for 3×3 kernels (2.25x speedup)");
    println!("  • Shared memory: Cache kernel/input for GPU");
    println!("  • Fusion: Conv2D + ReLU + BatchNorm → 1 kernel");
    println!("  • Depthwise separable: Factorize into depthwise + pointwise");
    println!();

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ Demo complete!");
    println!();
    println!("Conv2D is now implemented! THE operation for computer vision.");
    println!();
    println!("With Conv2D complete, barraCuda can now:");
    println!("  ✅ Process image classification (ResNet, VGG, etc.)");
    println!("  ✅ Detect objects (YOLO, Faster R-CNN)");
    println!("  ✅ Segment images (U-Net, DeepLab)");
    println!("  ✅ Extract features (edges, textures)");
    println!("  ✅ Handle multi-channel inputs (RGB, depth, etc.)");
    println!();
    println!("Universal Runtime makes Conv2D hardware-agnostic! 🚀");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    Ok(())
}
