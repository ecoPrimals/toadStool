//! Conv2D GPU Kernels for Convolutional Neural Networks
//!
//! Implements 2D convolution operations for CNNs:
//! - Standard convolution
//! - Strided convolution
//! - Padded convolution
//! - Batch processing
//!
//! Modern, idiomatic implementation with zero technical debt.

#![allow(unused_imports)]

use anyhow::{Context, Result};

#[cfg(feature = "opencl")]
use ocl::{Buffer, Context as OclContext, Device, Kernel, Platform, Program, Queue};

/// OpenCL kernel source for 2D convolution operations
#[cfg(feature = "opencl")]
pub const OPENCL_CONV2D_KERNEL: &str = r#"
// =============================================================================
// 2D Convolution Kernel
// =============================================================================
// Input:  (batch, in_channels, height, width)
// Kernel: (out_channels, in_channels, kernel_h, kernel_w)
// Output: (batch, out_channels, out_height, out_width)

__kernel void conv2d(
    __global const float* input,
    __global const float* weights,
    __global const float* bias,
    __global float* output,
    const int batch_size,
    const int in_channels,
    const int in_height,
    const int in_width,
    const int out_channels,
    const int kernel_h,
    const int kernel_w,
    const int stride_h,
    const int stride_w,
    const int pad_h,
    const int pad_w
) {
    // Output position
    const int b = get_global_id(0);  // batch
    const int oc = get_global_id(1); // output channel
    const int out_pos = get_global_id(2); // output spatial position
    
    const int out_height = (in_height + 2 * pad_h - kernel_h) / stride_h + 1;
    const int out_width = (in_width + 2 * pad_w - kernel_w) / stride_w + 1;
    
    const int oh = out_pos / out_width;
    const int ow = out_pos % out_width;
    
    if (b >= batch_size || oc >= out_channels || oh >= out_height || ow >= out_width) {
        return;
    }
    
    float sum = 0.0f;
    
    // Convolve over all input channels and kernel positions
    for (int ic = 0; ic < in_channels; ic++) {
        for (int kh = 0; kh < kernel_h; kh++) {
            for (int kw = 0; kw < kernel_w; kw++) {
                // Input position (with stride and padding)
                const int ih = oh * stride_h + kh - pad_h;
                const int iw = ow * stride_w + kw - pad_w;
                
                // Check bounds (for padding)
                if (ih >= 0 && ih < in_height && iw >= 0 && iw < in_width) {
                    // Input index: [b, ic, ih, iw]
                    const int input_idx = 
                        b * in_channels * in_height * in_width +
                        ic * in_height * in_width +
                        ih * in_width +
                        iw;
                    
                    // Weight index: [oc, ic, kh, kw]
                    const int weight_idx = 
                        oc * in_channels * kernel_h * kernel_w +
                        ic * kernel_h * kernel_w +
                        kh * kernel_w +
                        kw;
                    
                    sum += input[input_idx] * weights[weight_idx];
                }
            }
        }
    }
    
    // Add bias
    sum += bias[oc];
    
    // Output index: [b, oc, oh, ow]
    const int output_idx = 
        b * out_channels * out_height * out_width +
        oc * out_height * out_width +
        oh * out_width +
        ow;
    
    output[output_idx] = sum;
}

// =============================================================================
// Optimized Conv2D with Local Memory (for small kernels)
// =============================================================================
__kernel void conv2d_optimized(
    __global const float* input,
    __global const float* weights,
    __global const float* bias,
    __global float* output,
    const int batch_size,
    const int in_channels,
    const int in_height,
    const int in_width,
    const int out_channels,
    const int kernel_h,
    const int kernel_w,
    const int stride_h,
    const int stride_w,
    const int pad_h,
    const int pad_w
) {
    // Use local memory for weight tile
    __local float weight_tile[256]; // Adjust size based on kernel
    
    const int local_id = get_local_id(0);
    const int local_size = get_local_size(0);
    
    // Output position
    const int b = get_global_id(0);
    const int oc = get_global_id(1);
    const int oh = get_global_id(2) / get_global_size(3);
    const int ow = get_global_id(2) % get_global_size(3);
    
    if (b >= batch_size || oc >= out_channels) {
        return;
    }
    
    const int out_height = (in_height + 2 * pad_h - kernel_h) / stride_h + 1;
    const int out_width = (in_width + 2 * pad_w - kernel_w) / stride_w + 1;
    
    if (oh >= out_height || ow >= out_width) {
        return;
    }
    
    // Cooperatively load weights into local memory
    const int weights_per_channel = kernel_h * kernel_w;
    const int total_weights = in_channels * weights_per_channel;
    
    for (int i = local_id; i < total_weights && i < 256; i += local_size) {
        const int weight_idx = oc * total_weights + i;
        weight_tile[i] = weights[weight_idx];
    }
    
    barrier(CLK_LOCAL_MEM_FENCE);
    
    float sum = 0.0f;
    
    // Convolve using local memory
    for (int ic = 0; ic < in_channels; ic++) {
        for (int kh = 0; kh < kernel_h; kh++) {
            for (int kw = 0; kw < kernel_w; kw++) {
                const int ih = oh * stride_h + kh - pad_h;
                const int iw = ow * stride_w + kw - pad_w;
                
                if (ih >= 0 && ih < in_height && iw >= 0 && iw < in_width) {
                    const int input_idx = 
                        b * in_channels * in_height * in_width +
                        ic * in_height * in_width +
                        ih * in_width +
                        iw;
                    
                    const int weight_tile_idx = 
                        ic * weights_per_channel +
                        kh * kernel_w +
                        kw;
                    
                    sum += input[input_idx] * weight_tile[weight_tile_idx];
                }
            }
        }
    }
    
    sum += bias[oc];
    
    const int output_idx = 
        b * out_channels * out_height * out_width +
        oc * out_height * out_width +
        oh * out_width +
        ow;
    
    output[output_idx] = sum;
}

// =============================================================================
// MaxPool2D Kernel
// =============================================================================
__kernel void maxpool2d(
    __global const float* input,
    __global float* output,
    const int batch_size,
    const int channels,
    const int in_height,
    const int in_width,
    const int kernel_h,
    const int kernel_w,
    const int stride_h,
    const int stride_w,
    const int pad_h,
    const int pad_w
) {
    const int b = get_global_id(0);
    const int c = get_global_id(1);
    const int oh = get_global_id(2) / get_global_size(3);
    const int ow = get_global_id(2) % get_global_size(3);
    
    if (b >= batch_size || c >= channels) {
        return;
    }
    
    const int out_height = (in_height + 2 * pad_h - kernel_h) / stride_h + 1;
    const int out_width = (in_width + 2 * pad_w - kernel_w) / stride_w + 1;
    
    if (oh >= out_height || ow >= out_width) {
        return;
    }
    
    float max_val = -FLT_MAX;
    
    for (int kh = 0; kh < kernel_h; kh++) {
        for (int kw = 0; kw < kernel_w; kw++) {
            const int ih = oh * stride_h + kh - pad_h;
            const int iw = ow * stride_w + kw - pad_w;
            
            if (ih >= 0 && ih < in_height && iw >= 0 && iw < in_width) {
                const int input_idx = 
                    b * channels * in_height * in_width +
                    c * in_height * in_width +
                    ih * in_width +
                    iw;
                
                max_val = fmax(max_val, input[input_idx]);
            }
        }
    }
    
    const int output_idx = 
        b * channels * out_height * out_width +
        c * out_height * out_width +
        oh * out_width +
        ow;
    
    output[output_idx] = max_val;
}
"#;

/// Conv2D operation parameters
#[derive(Debug, Clone)]
pub struct Conv2DParams {
    pub batch_size: usize,
    pub in_channels: usize,
    pub in_height: usize,
    pub in_width: usize,
    pub out_channels: usize,
    pub kernel_h: usize,
    pub kernel_w: usize,
    pub stride_h: usize,
    pub stride_w: usize,
    pub pad_h: usize,
    pub pad_w: usize,
}

impl Conv2DParams {
    /// Calculate output dimensions
    pub fn output_height(&self) -> usize {
        (self.in_height + 2 * self.pad_h - self.kernel_h) / self.stride_h + 1
    }
    
    pub fn output_width(&self) -> usize {
        (self.in_width + 2 * self.pad_w - self.kernel_w) / self.stride_w + 1
    }
    
    /// Calculate total input size
    pub fn input_size(&self) -> usize {
        self.batch_size * self.in_channels * self.in_height * self.in_width
    }
    
    /// Calculate total weight size
    pub fn weight_size(&self) -> usize {
        self.out_channels * self.in_channels * self.kernel_h * self.kernel_w
    }
    
    /// Calculate total output size
    pub fn output_size(&self) -> usize {
        self.batch_size * self.out_channels * self.output_height() * self.output_width()
    }
}

/// OpenCL Conv2D executor
#[cfg(feature = "opencl")]
pub struct Conv2DExecutor {
    _context: OclContext,
    queue: Queue,
    program: Program,
}

#[cfg(feature = "opencl")]
impl Conv2DExecutor {
    /// Create new Conv2D executor
    pub fn new() -> Result<Self> {
        use anyhow::Context;
        
        // Find a platform with GPU devices
        let platforms = Platform::list();
        let mut selected_device = None;
        let mut selected_platform = None;
        
        for platform in platforms {
            if let Ok(devices) = Device::list_all(platform) {
                for device in devices {
                    // Check if it's a GPU device
                    if let Ok(device_type) = device.info(ocl::core::DeviceInfo::Type) {
                        use ocl::core::{DeviceInfoResult, DeviceType};
                        if let DeviceInfoResult::Type(DeviceType::GPU) = device_type {
                            selected_device = Some(device);
                            selected_platform = Some(platform);
                            break;
                        }
                    }
                }
                if selected_device.is_some() {
                    break;
                }
            }
        }
        
        let device = selected_device.context("No OpenCL GPU device found")?;
        let platform = selected_platform.context("No OpenCL platform found")?;
        
        let context = OclContext::builder()
            .platform(platform)
            .devices(device)
            .build()
            .context("Failed to create OpenCL context")?;
        
        let queue = Queue::new(&context, device, None)
            .context("Failed to create command queue")?;
        
        let program = Program::builder()
            .src(OPENCL_CONV2D_KERNEL)
            .devices(device)
            .build(&context)
            .context("Failed to build OpenCL program")?;
        
        Ok(Self {
            _context: context,
            queue,
            program,
        })
    }
    
    /// Execute Conv2D operation on GPU
    pub fn conv2d(
        &self,
        input: &[f32],
        weights: &[f32],
        bias: &[f32],
        params: &Conv2DParams,
    ) -> Result<Vec<f32>> {
        use anyhow::Context;
        
        // Validate input sizes
        assert_eq!(input.len(), params.input_size());
        assert_eq!(weights.len(), params.weight_size());
        assert_eq!(bias.len(), params.out_channels);
        
        let output_size = params.output_size();
        
        // Create GPU buffers
        let input_buf = Buffer::builder()
            .queue(self.queue.clone())
            .len(input.len())
            .copy_host_slice(input)
            .build()
            .context("Failed to create input buffer")?;
        
        let weights_buf = Buffer::builder()
            .queue(self.queue.clone())
            .len(weights.len())
            .copy_host_slice(weights)
            .build()
            .context("Failed to create weights buffer")?;
        
        let bias_buf = Buffer::builder()
            .queue(self.queue.clone())
            .len(bias.len())
            .copy_host_slice(bias)
            .build()
            .context("Failed to create bias buffer")?;
        
        let output_buf: Buffer<f32> = Buffer::builder()
            .queue(self.queue.clone())
            .len(output_size)
            .build()
            .context("Failed to create output buffer")?;
        
        // Build kernel
        let out_height = params.output_height();
        let out_width = params.output_width();
        
        let kernel = Kernel::builder()
            .program(&self.program)
            .name("conv2d")
            .queue(self.queue.clone())
            .global_work_size((
                params.batch_size,
                params.out_channels,
                out_height * out_width,
            ))
            .arg(&input_buf)
            .arg(&weights_buf)
            .arg(&bias_buf)
            .arg(&output_buf)
            .arg(params.batch_size as i32)
            .arg(params.in_channels as i32)
            .arg(params.in_height as i32)
            .arg(params.in_width as i32)
            .arg(params.out_channels as i32)
            .arg(params.kernel_h as i32)
            .arg(params.kernel_w as i32)
            .arg(params.stride_h as i32)
            .arg(params.stride_w as i32)
            .arg(params.pad_h as i32)
            .arg(params.pad_w as i32)
            .build()
            .context("Failed to build Conv2D kernel")?;
        
        // Execute
        unsafe {
            kernel.enq().context("Failed to execute Conv2D")?;
        }
        
        // Read results
        let mut output = vec![0.0f32; output_size];
        output_buf.read(&mut output).enq()
            .context("Failed to read output from GPU")?;
        
        Ok(output)
    }
    
    /// Execute MaxPool2D operation on GPU
    pub fn maxpool2d(
        &self,
        input: &[f32],
        batch_size: usize,
        channels: usize,
        in_height: usize,
        in_width: usize,
        kernel_h: usize,
        kernel_w: usize,
        stride_h: usize,
        stride_w: usize,
    ) -> Result<Vec<f32>> {
        use anyhow::Context;
        
        let out_height = (in_height - kernel_h) / stride_h + 1;
        let out_width = (in_width - kernel_w) / stride_w + 1;
        let output_size = batch_size * channels * out_height * out_width;
        
        // Create GPU buffers
        let input_buf = Buffer::builder()
            .queue(self.queue.clone())
            .len(input.len())
            .copy_host_slice(input)
            .build()
            .context("Failed to create input buffer")?;
        
        let output_buf: Buffer<f32> = Buffer::builder()
            .queue(self.queue.clone())
            .len(output_size)
            .build()
            .context("Failed to create output buffer")?;
        
        // Build kernel
        let kernel = Kernel::builder()
            .program(&self.program)
            .name("maxpool2d")
            .queue(self.queue.clone())
            .global_work_size([batch_size, channels, out_height * out_width])
            .arg(&input_buf)
            .arg(&output_buf)
            .arg(batch_size as i32)
            .arg(channels as i32)
            .arg(in_height as i32)
            .arg(in_width as i32)
            .arg(kernel_h as i32)
            .arg(kernel_w as i32)
            .arg(stride_h as i32)
            .arg(stride_w as i32)
            .arg(0i32) // pad_h
            .arg(0i32) // pad_w
            .build()
            .context("Failed to build MaxPool2D kernel")?;
        
        // Execute
        unsafe {
            kernel.enq().context("Failed to execute MaxPool2D")?;
        }
        
        // Read results
        let mut output = vec![0.0f32; output_size];
        output_buf.read(&mut output).enq()
            .context("Failed to read output from GPU")?;
        
        Ok(output)
    }
}

/// CPU reference implementation for Conv2D (for testing)
pub fn conv2d_cpu(
    input: &[f32],
    weights: &[f32],
    bias: &[f32],
    params: &Conv2DParams,
) -> Vec<f32> {
    let out_height = params.output_height();
    let out_width = params.output_width();
    let output_size = params.output_size();
    let mut output = vec![0.0f32; output_size];
    
    for b in 0..params.batch_size {
        for oc in 0..params.out_channels {
            for oh in 0..out_height {
                for ow in 0..out_width {
                    let mut sum = 0.0f32;
                    
                    for ic in 0..params.in_channels {
                        for kh in 0..params.kernel_h {
                            for kw in 0..params.kernel_w {
                                let ih = oh * params.stride_h + kh;
                                let iw = ow * params.stride_w + kw;
                                
                                if ih >= params.pad_h
                                    && ih < params.in_height + params.pad_h
                                    && iw >= params.pad_w
                                    && iw < params.in_width + params.pad_w
                                {
                                    let ih = ih - params.pad_h;
                                    let iw = iw - params.pad_w;
                                    
                                    let input_idx = b * params.in_channels * params.in_height * params.in_width
                                        + ic * params.in_height * params.in_width
                                        + ih * params.in_width
                                        + iw;
                                    
                                    let weight_idx = oc * params.in_channels * params.kernel_h * params.kernel_w
                                        + ic * params.kernel_h * params.kernel_w
                                        + kh * params.kernel_w
                                        + kw;
                                    
                                    sum += input[input_idx] * weights[weight_idx];
                                }
                            }
                        }
                    }
                    
                    sum += bias[oc];
                    
                    let output_idx = b * params.out_channels * out_height * out_width
                        + oc * out_height * out_width
                        + oh * out_width
                        + ow;
                    
                    output[output_idx] = sum;
                }
            }
        }
    }
    
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conv2d_params() {
        let params = Conv2DParams {
            batch_size: 1,
            in_channels: 3,
            in_height: 28,
            in_width: 28,
            out_channels: 32,
            kernel_h: 3,
            kernel_w: 3,
            stride_h: 1,
            stride_w: 1,
            pad_h: 0,
            pad_w: 0,
        };
        
        assert_eq!(params.output_height(), 26);
        assert_eq!(params.output_width(), 26);
        assert_eq!(params.input_size(), 1 * 3 * 28 * 28);
        assert_eq!(params.weight_size(), 32 * 3 * 3 * 3);
        assert_eq!(params.output_size(), 1 * 32 * 26 * 26);
    }
    
    #[test]
    fn test_conv2d_cpu_simple() {
        // Simple 1x1x3x3 input, 1 filter of 1x1x2x2
        let params = Conv2DParams {
            batch_size: 1,
            in_channels: 1,
            in_height: 3,
            in_width: 3,
            out_channels: 1,
            kernel_h: 2,
            kernel_w: 2,
            stride_h: 1,
            stride_w: 1,
            pad_h: 0,
            pad_w: 0,
        };
        
        let input = vec![
            1.0, 2.0, 3.0,
            4.0, 5.0, 6.0,
            7.0, 8.0, 9.0,
        ];
        
        let weights = vec![
            1.0, 1.0,
            1.0, 1.0,
        ];
        
        let bias = vec![0.0];
        
        let output = conv2d_cpu(&input, &weights, &bias, &params);
        
        // Should be 2x2 output
        assert_eq!(output.len(), 4);
        
        // Verify values
        assert_eq!(output[0], 12.0); // 1+2+4+5
        assert_eq!(output[1], 16.0); // 2+3+5+6
        assert_eq!(output[2], 24.0); // 4+5+7+8
        assert_eq!(output[3], 28.0); // 5+6+8+9
    }
}

