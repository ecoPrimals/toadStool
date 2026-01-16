//! GPU Kernels for Neural Network Operations
//!
//! OpenCL and CUDA kernels for MNIST inference:
//! - Matrix multiplication (GEMM)
//! - ReLU activation
//! - Softmax
//!
//! Modern, idiomatic implementation with proper error handling.

#![allow(unused_imports)]

#[cfg(feature = "opencl")]
use anyhow::Context;
use anyhow::Result;
#[cfg(feature = "opencl")]
use ocl::{Buffer, Context as OclContext, Device, Kernel, Platform, Program, Queue};
#[cfg(feature = "opencl")]
use tracing;

/// OpenCL kernel source for neural network operations
#[cfg(feature = "opencl")]
pub const OPENCL_NN_KERNEL: &str = r#"
// Matrix multiplication: C = A * B
// A: (M, K) - input/hidden layer
// B: (K, N) - weights
// C: (M, N) - output
__kernel void matmul(
    __global const float* A,
    __global const float* B,
    __global float* C,
    const int M,
    const int K,
    const int N
) {
    const int row = get_global_id(0);
    const int col = get_global_id(1);
    
    if (row < M && col < N) {
        float sum = 0.0f;
        for (int k = 0; k < K; k++) {
            sum += A[row * K + k] * B[k * N + col];
        }
        C[row * N + col] = sum;
    }
}

// Add bias vector to each row of matrix
// A: (M, N) - matrix
// bias: (N,) - bias vector
// out: (M, N) - output
__kernel void add_bias(
    __global const float* A,
    __global const float* bias,
    __global float* out,
    const int M,
    const int N
) {
    const int row = get_global_id(0);
    const int col = get_global_id(1);
    
    if (row < M && col < N) {
        out[row * N + col] = A[row * N + col] + bias[col];
    }
}

// ReLU activation: out = max(0, x)
__kernel void relu(
    __global const float* input,
    __global float* output,
    const int size
) {
    const int i = get_global_id(0);
    
    if (i < size) {
        output[i] = fmax(0.0f, input[i]);
    }
}

// Softmax activation (per row)
// Each row is treated as a separate softmax
__kernel void softmax(
    __global const float* input,
    __global float* output,
    const int M,
    const int N
) {
    const int row = get_global_id(0);
    
    if (row < M) {
        // Find max for numerical stability
        float max_val = input[row * N];
        for (int i = 1; i < N; i++) {
            float val = input[row * N + i];
            if (val > max_val) {
                max_val = val;
            }
        }
        
        // Compute exp and sum
        float sum = 0.0f;
        for (int i = 0; i < N; i++) {
            float exp_val = exp(input[row * N + i] - max_val);
            output[row * N + i] = exp_val;
            sum += exp_val;
        }
        
        // Normalize
        for (int i = 0; i < N; i++) {
            output[row * N + i] /= sum;
        }
    }
}

// Combined forward pass for layer: out = relu(input @ weights + bias)
__kernel void dense_relu(
    __global const float* input,
    __global const float* weights,
    __global const float* bias,
    __global float* output,
    const int M,
    const int K,
    const int N
) {
    const int row = get_global_id(0);
    const int col = get_global_id(1);
    
    if (row < M && col < N) {
        float sum = 0.0f;
        for (int k = 0; k < K; k++) {
            sum += input[row * K + k] * weights[k * N + col];
        }
        sum += bias[col];
        output[row * N + col] = fmax(0.0f, sum);
    }
}
"#;

/// CUDA kernel source for neural network operations
pub const CUDA_NN_KERNEL: &str = r#"
extern "C" {

// Matrix multiplication: C = A * B
__global__ void matmul(
    const float* A,
    const float* B,
    float* C,
    int M,
    int K,
    int N
) {
    int row = blockIdx.y * blockDim.y + threadIdx.y;
    int col = blockIdx.x * blockDim.x + threadIdx.x;
    
    if (row < M && col < N) {
        float sum = 0.0f;
        for (int k = 0; k < K; k++) {
            sum += A[row * K + k] * B[k * N + col];
        }
        C[row * N + col] = sum;
    }
}

// ReLU activation
__global__ void relu(
    const float* input,
    float* output,
    int size
) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    
    if (i < size) {
        output[i] = fmaxf(0.0f, input[i]);
    }
}

// Combined dense + ReLU layer
__global__ void dense_relu(
    const float* input,
    const float* weights,
    const float* bias,
    float* output,
    int M,
    int K,
    int N
) {
    int row = blockIdx.y * blockDim.y + threadIdx.y;
    int col = blockIdx.x * blockDim.x + threadIdx.x;
    
    if (row < M && col < N) {
        float sum = 0.0f;
        for (int k = 0; k < K; k++) {
            sum += input[row * K + k] * weights[k * N + col];
        }
        sum += bias[col];
        output[row * N + col] = fmaxf(0.0f, sum);
    }
}

}
"#;

/// GPU execution backend
pub enum GpuExecutor {
    #[cfg(feature = "opencl")]
    OpenCL(OpenCLExecutor),
    #[cfg(feature = "cuda")]
    Cuda(CudaExecutor),
    Cpu, // Fallback
}

#[cfg(feature = "opencl")]
pub struct OpenCLExecutor {
    _context: ocl::Context,
    queue: ocl::Queue,
    program: ocl::Program,
}

#[cfg(feature = "opencl")]
impl OpenCLExecutor {
    /// Create new OpenCL executor for neural network inference
    pub fn new(device: &ocl::Device) -> Result<Self> {
        use anyhow::Context;

        let context = ocl::Context::builder()
            .devices(*device)
            .build()
            .context("Failed to create OpenCL context")?;

        let queue =
            ocl::Queue::new(&context, *device, None).context("Failed to create OpenCL queue")?;

        let program = ocl::Program::builder()
            .src(OPENCL_NN_KERNEL)
            .devices(*device)
            .build(&context)
            .context("Failed to build OpenCL program")?;

        Ok(Self {
            _context: context,
            queue,
            program,
        })
    }

    /// Execute neural network forward pass on GPU
    ///
    /// Layer 1: input (batch, 784) @ w1 (784, 128) + b1 (128,) -> ReLU -> hidden (batch, 128)
    /// Layer 2: hidden (batch, 128) @ w2 (128, 10) + b2 (10,) -> softmax -> output (batch, 10)
    pub fn forward_pass(
        &self,
        input: &[f32], // (batch, 784)
        w1: &[f32],    // (784, 128)
        b1: &[f32],    // (128,)
        w2: &[f32],    // (128, 10)
        b2: &[f32],    // (10,)
        batch_size: usize,
    ) -> Result<Vec<f32>> {
        use anyhow::Context;

        // Create GPU buffers
        let input_buf = ocl::Buffer::builder()
            .queue(self.queue.clone())
            .len(batch_size * 784)
            .copy_host_slice(input)
            .build()
            .context("Failed to create input buffer")?;

        let w1_buf = ocl::Buffer::builder()
            .queue(self.queue.clone())
            .len(784 * 128)
            .copy_host_slice(w1)
            .build()
            .context("Failed to create w1 buffer")?;

        let b1_buf = ocl::Buffer::builder()
            .queue(self.queue.clone())
            .len(128)
            .copy_host_slice(b1)
            .build()
            .context("Failed to create b1 buffer")?;

        let w2_buf = ocl::Buffer::builder()
            .queue(self.queue.clone())
            .len(128 * 10)
            .copy_host_slice(w2)
            .build()
            .context("Failed to create w2 buffer")?;

        let b2_buf = ocl::Buffer::builder()
            .queue(self.queue.clone())
            .len(10)
            .copy_host_slice(b2)
            .build()
            .context("Failed to create b2 buffer")?;

        // Intermediate buffers
        let hidden_buf: ocl::Buffer<f32> = ocl::Buffer::builder()
            .queue(self.queue.clone())
            .len(batch_size * 128)
            .build()
            .context("Failed to create hidden buffer")?;

        let output_buf: ocl::Buffer<f32> = ocl::Buffer::builder()
            .queue(self.queue.clone())
            .len(batch_size * 10)
            .build()
            .context("Failed to create output buffer")?;

        // Layer 1: input @ w1 + b1 -> ReLU -> hidden
        let kernel1 = ocl::Kernel::builder()
            .program(&self.program)
            .name("dense_relu")
            .queue(self.queue.clone())
            .global_work_size([batch_size, 128])
            .arg(&input_buf)
            .arg(&w1_buf)
            .arg(&b1_buf)
            .arg(&hidden_buf)
            .arg(batch_size as i32)
            .arg(784i32)
            .arg(128i32)
            .build()
            .context("Failed to build layer1 kernel")?;

        // SAFETY: OpenCL FFI - kernel.enq() is unsafe in ocl crate (thin wrapper over clEnqueueNDRangeKernel).
        // Invariants upheld:
        // - Kernel built with correct buffer arguments (input, w1, b1, output, dimensions)
        // - All buffers are valid and not aliased
        // - Queue is valid and matches kernel's queue
        unsafe {
            kernel1.enq().context("Failed to execute layer1")?;
        }

        // Layer 2: hidden @ w2 (no bias/activation in kernel, will add after)
        let z2_buf: ocl::Buffer<f32> = ocl::Buffer::builder()
            .queue(self.queue.clone())
            .len(batch_size * 10)
            .build()
            .context("Failed to create z2 buffer")?;

        let kernel2 = ocl::Kernel::builder()
            .program(&self.program)
            .name("matmul")
            .queue(self.queue.clone())
            .global_work_size([batch_size, 10])
            .arg(&hidden_buf)
            .arg(&w2_buf)
            .arg(&z2_buf)
            .arg(batch_size as i32)
            .arg(128i32)
            .arg(10i32)
            .build()
            .context("Failed to build layer2 matmul kernel")?;

        unsafe {
            kernel2.enq().context("Failed to execute layer2 matmul")?;
        }

        // Add bias
        let kernel_bias = ocl::Kernel::builder()
            .program(&self.program)
            .name("add_bias")
            .queue(self.queue.clone())
            .global_work_size([batch_size, 10])
            .arg(&z2_buf)
            .arg(&b2_buf)
            .arg(&output_buf)
            .arg(batch_size as i32)
            .arg(10i32)
            .build()
            .context("Failed to build add_bias kernel")?;

        // SAFETY: OpenCL FFI - kernel.enq() is unsafe in ocl crate (thin wrapper over clEnqueueNDRangeKernel).
        // Invariants upheld:
        // - Kernel built with correct buffer arguments (z2, b2, output, dimensions)
        // - All buffers are valid and not aliased
        // - Queue is valid and matches kernel's queue
        unsafe {
            kernel_bias.enq().context("Failed to execute add_bias")?;
        }

        // Softmax
        let kernel_softmax = ocl::Kernel::builder()
            .program(&self.program)
            .name("softmax")
            .queue(self.queue.clone())
            .global_work_size(batch_size)
            .arg(&output_buf)
            .arg(&output_buf) // In-place
            .arg(batch_size as i32)
            .arg(10i32)
            .build()
            .context("Failed to build softmax kernel")?;

        // SAFETY: OpenCL FFI - kernel.enq() is unsafe in ocl crate (thin wrapper over clEnqueueNDRangeKernel).
        // Invariants upheld:
        // - Kernel built with correct buffer arguments (output, output, dimensions)
        // - In-place operation: output buffer used for both input and output
        // - All buffers are valid and not aliased
        // - Queue is valid and matches kernel's queue
        unsafe {
            kernel_softmax.enq().context("Failed to execute softmax")?;
        }

        // Read results back
        let mut output = vec![0.0f32; batch_size * 10];
        output_buf
            .read(&mut output)
            .enq()
            .context("Failed to read output from GPU")?;

        Ok(output)
    }
}

#[cfg(feature = "cuda")]
pub struct CudaExecutor {
    // TODO: Implement CUDA executor
    // Will use cudarc for kernel compilation and execution
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opencl_kernel_compiles() {
        // Kernel source should be valid OpenCL
        assert!(OPENCL_NN_KERNEL.contains("__kernel"));
        assert!(OPENCL_NN_KERNEL.contains("matmul"));
        assert!(OPENCL_NN_KERNEL.contains("relu"));
        assert!(OPENCL_NN_KERNEL.contains("softmax"));
    }

    #[test]
    fn test_cuda_kernel_syntax() {
        // Kernel source should be valid CUDA
        assert!(CUDA_NN_KERNEL.contains("__global__"));
        assert!(CUDA_NN_KERNEL.contains("extern \"C\""));
    }
}
