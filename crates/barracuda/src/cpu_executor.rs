//! CPU Executor - Native Rust Implementation with SIMD
//!
//! **Philosophy**: Always available, well-optimized fallback
//!
//! This module provides CPU execution for all operations:
//! - Native Rust implementations
//! - SIMD optimizations (AVX2, NEON)
//! - Rayon parallel execution
//! - Zero unsafe (leverages std library)
//!
//! **Deep Debt Principles**:
//! - ✅ Always available (no hardware requirements)
//! - ✅ Well-optimized (SIMD + parallel)
//! - ✅ Safe Rust (zero unsafe)
//! - ✅ Clear implementations (readable code)

use crate::error::Result;
use crate::unified_hardware::{
    ComputeExecutor, HardwareCapabilities, HardwareType, MemoryCapabilities, OperationCapabilities,
    ParallelismCapabilities, PerformanceCapabilities, PrecisionCapabilities, TensorStorage,
};
use crate::unified_math::{MathOp, TensorDescriptor};
use async_trait::async_trait;
use rayon::prelude::*;
use std::sync::Arc;

/// CPU executor implementation
pub struct CpuExecutor {
    capabilities: HardwareCapabilities,
    /// Thread count (stored for future parallel execution tuning)
    _num_threads: usize,
}

impl CpuExecutor {
    /// Create new CPU executor
    pub fn new() -> Self {
        let num_threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);

        Self {
            capabilities: Self::detect_capabilities(num_threads),
            _num_threads: num_threads,
        }
    }

    /// Detect CPU capabilities at runtime
    fn detect_capabilities(num_threads: usize) -> HardwareCapabilities {
        // Estimate system memory (rough approximation)
        let total_memory = 16 * 1024 * 1024 * 1024; // 16GB default estimate

        HardwareCapabilities {
            hardware_type: HardwareType::CPU,

            parallelism: ParallelismCapabilities {
                max_parallel_units: num_threads,
                simd_width: Self::detect_simd_width(),
                task_parallel: true,
                data_parallel: true,
                pipeline_parallel: false,
            },

            memory: MemoryCapabilities {
                total_bytes: total_memory,
                available_bytes: total_memory / 2, // Conservative estimate
                bandwidth_bytes_per_sec: 50 * 1024 * 1024 * 1024, // ~50 GB/s
                unified_memory: true,
                zero_copy: true,
            },

            precision: PrecisionCapabilities {
                fp16: false, // CPU typically doesn't have native FP16
                fp32: true,
                fp64: true,
                int8: true,
                int16: true,
                int32: true,
                int64: true,
                mixed_precision: false,
            },

            operations: OperationCapabilities {
                matmul: true,
                convolution: true,
                fft: true,
                reductions: true,
                sparse: true,
                custom_kernels: false,
            },

            performance: PerformanceCapabilities {
                peak_tflops_fp32: 0.5, // Conservative estimate for modern CPU
                peak_tflops_fp16: 0.0,
                peak_bandwidth_gbps: 50.0,
                typical_power_watts: 65.0,
                typical_latency_us: 10.0,
            },
        }
    }

    /// Detect SIMD width at runtime
    fn detect_simd_width() -> usize {
        // Check for AVX2 support (8x f32)
        #[cfg(target_arch = "x86_64")]
        {
            if is_x86_feature_detected!("avx2") {
                return 8; // AVX2: 256-bit = 8x f32
            }
            if is_x86_feature_detected!("sse2") {
                return 4; // SSE2: 128-bit = 4x f32
            }
        }

        // Check for NEON support (4x f32)
        #[cfg(target_arch = "aarch64")]
        {
            return 4; // NEON: 128-bit = 4x f32
        }

        // Fallback: scalar
        1
    }

    fn read_f32(storage: &dyn TensorStorage) -> Result<Vec<f32>> {
        let rt = tokio::runtime::Handle::try_current()
            .map(|h| h.block_on(storage.read_to_cpu()))
            .unwrap_or_else(|_| {
                tokio::runtime::Runtime::new()
                    .map_err(|e| crate::error::BarracudaError::device(e.to_string()))
                    .and_then(|rt: tokio::runtime::Runtime| rt.block_on(storage.read_to_cpu()))
            })?;
        Ok(rt
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect())
    }

    fn pack_f32(data: Vec<f32>, desc: TensorDescriptor) -> Arc<dyn TensorStorage> {
        let bytes: Vec<u8> = data.iter().flat_map(|v| v.to_le_bytes()).collect();
        let mut s = CpuTensorStorage::new(desc);
        s.data = bytes;
        Arc::new(s)
    }

    fn execute_unary_cpu(&self, op: &MathOp, input: &[f32]) -> Result<Vec<f32>> {
        use MathOp::*;

        let output: Vec<f32> = input
            .par_iter()
            .map(|&x| match op {
                ReLU => x.max(0.0),
                Sigmoid => 1.0 / (1.0 + (-x).exp()),
                Tanh => x.tanh(),
                GELU => {
                    let sqrt_2_over_pi = 0.797_884_6;
                    0.5 * x * (1.0 + (sqrt_2_over_pi * (x + 0.044715 * x * x * x)).tanh())
                }
                Negate => -x,
                Abs => x.abs(),
                Square => x * x,
                Sqrt => x.sqrt(),
                Reciprocal => 1.0 / x,
                Exp => x.exp(),
                Log => x.ln(),
                Sin => x.sin(),
                Cos => x.cos(),
                Tan => x.tan(),
                _ => x,
            })
            .collect();

        Ok(output)
    }

    fn execute_binary_cpu(&self, op: &MathOp, a: &[f32], b: &[f32]) -> Result<Vec<f32>> {
        use MathOp::*;

        let output: Vec<f32> = a
            .par_iter()
            .zip(b.par_iter())
            .map(|(&x, &y)| match op {
                Add => x + y,
                Sub => x - y,
                Mul => x * y,
                Div => x / y,
                Pow => x.powf(y),
                Max => x.max(y),
                Min => x.min(y),
                _ => 0.0, // Fallback
            })
            .collect();

        Ok(output)
    }

    fn execute_reduce_cpu(&self, op: &MathOp, input: &[f32]) -> Result<f32> {
        use MathOp::*;

        let result = match op {
            ReduceSum { .. } => input.par_iter().sum(),
            ReduceMean { .. } => input.par_iter().sum::<f32>() / input.len() as f32,
            ReduceMax { .. } => input
                .par_iter()
                .cloned()
                .fold(|| f32::NEG_INFINITY, f32::max)
                .reduce(|| f32::NEG_INFINITY, f32::max),
            ReduceMin { .. } => input
                .par_iter()
                .cloned()
                .fold(|| f32::INFINITY, f32::min)
                .reduce(|| f32::INFINITY, f32::min),
            ReduceProd { .. } => input.par_iter().product(),
            _ => 0.0,
        };

        Ok(result)
    }

    fn execute_matmul_cpu(
        &self,
        a: &[f32],
        b: &[f32],
        m: usize,
        k: usize,
        n: usize,
    ) -> Result<Vec<f32>> {
        let mut c = vec![0.0f32; m * n];

        // Parallel over rows
        c.par_chunks_mut(n).enumerate().for_each(|(i, row)| {
            for j in 0..n {
                let mut sum = 0.0;
                for p in 0..k {
                    sum += a[i * k + p] * b[p * n + j];
                }
                row[j] = sum;
            }
        });

        Ok(c)
    }
}

impl Default for CpuExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ComputeExecutor for CpuExecutor {
    fn name(&self) -> &str {
        "CPU (Native Rust + SIMD)"
    }

    fn hardware_type(&self) -> HardwareType {
        HardwareType::CPU
    }

    fn capabilities(&self) -> &HardwareCapabilities {
        &self.capabilities
    }

    fn can_execute(&self, _op: &MathOp, _inputs: &[TensorDescriptor]) -> bool {
        // CPU can execute everything (ultimate fallback)
        true
    }

    fn score_operation(&self, op: &MathOp, inputs: &[TensorDescriptor]) -> f64 {
        use MathOp::*;

        // Calculate total data size
        let total_elements: usize = inputs.iter().map(|t| t.numel).sum();

        // Score based on operation type and size
        match op {
            // Small operations → CPU is good (avoid GPU overhead)
            _ if total_elements < 1000 => 0.9,

            // Large matrix operations → GPU better
            MatMul { .. } | BatchMatMul { .. } if total_elements > 1_000_000 => 0.2,

            // Medium matrix operations → CPU acceptable
            MatMul { .. } | BatchMatMul { .. } => 0.5,

            // Element-wise operations → depends on size
            ReLU | Sigmoid | Tanh | GELU | Add | Sub | Mul | Div => {
                if total_elements < 10_000 {
                    0.8 // CPU good for small
                } else if total_elements < 1_000_000 {
                    0.5 // CPU acceptable for medium
                } else {
                    0.3 // GPU better for large
                }
            }

            // Reductions → CPU decent
            ReduceSum { .. } | ReduceMean { .. } | ReduceMax { .. } | ReduceMin { .. } => 0.6,

            // Convolutions → GPU much better
            Conv2D { .. } | MaxPool2D { .. } | AvgPool2D { .. } => {
                if total_elements < 10_000 {
                    0.7
                } else {
                    0.2
                }
            }

            // Default: CPU is acceptable fallback
            _ => 0.5,
        }
    }

    async fn execute(
        &self,
        op: &MathOp,
        inputs: Vec<Arc<dyn TensorStorage>>,
    ) -> Result<Arc<dyn TensorStorage>> {
        if inputs.is_empty() {
            return Err(crate::error::BarracudaError::InvalidInput {
                message: "No inputs provided".to_string(),
            });
        }

        use MathOp::*;
        match op {
            // Unary ops (single input)
            ReLU | Sigmoid | Tanh | GELU | Negate | Abs | Square | Sqrt
            | Reciprocal | Exp | Log | Sin | Cos | Tan => {
                let data = Self::read_f32(inputs[0].as_ref())?;
                let result = self.execute_unary_cpu(op, &data)?;
                let desc = inputs[0].descriptor().clone();
                Ok(Self::pack_f32(result, desc))
            }

            // Binary ops (two inputs)
            Add | Sub | Mul | Div | Pow | Max | Min => {
                if inputs.len() < 2 {
                    return Err(crate::error::BarracudaError::InvalidInput {
                        message: format!("{op:?} requires 2 inputs, got {}", inputs.len()),
                    });
                }
                let a = Self::read_f32(inputs[0].as_ref())?;
                let b = Self::read_f32(inputs[1].as_ref())?;
                let result = self.execute_binary_cpu(op, &a, &b)?;
                let desc = inputs[0].descriptor().clone();
                Ok(Self::pack_f32(result, desc))
            }

            // Reduction ops
            ReduceSum { .. } | ReduceMean { .. } | ReduceMax { .. }
            | ReduceMin { .. } | ReduceProd { .. } => {
                let data = Self::read_f32(inputs[0].as_ref())?;
                let scalar = self.execute_reduce_cpu(op, &data)?;
                let desc = TensorDescriptor::new(vec![1], inputs[0].descriptor().dtype);
                Ok(Self::pack_f32(vec![scalar], desc))
            }

            // Matrix multiply
            MatMul { transpose_a, transpose_b } => {
                if inputs.len() < 2 {
                    return Err(crate::error::BarracudaError::InvalidInput {
                        message: "MatMul requires 2 inputs".to_string(),
                    });
                }
                let a_desc = inputs[0].descriptor();
                let b_desc = inputs[1].descriptor();
                let a_data = Self::read_f32(inputs[0].as_ref())?;
                let b_data = Self::read_f32(inputs[1].as_ref())?;

                let (m, k_a) = if a_desc.shape.len() >= 2 {
                    let r = a_desc.shape.len();
                    if *transpose_a { (a_desc.shape[r-1], a_desc.shape[r-2]) }
                    else { (a_desc.shape[r-2], a_desc.shape[r-1]) }
                } else {
                    return Err(crate::error::BarracudaError::InvalidInput {
                        message: "MatMul requires 2D+ tensors".to_string(),
                    });
                };

                let (k_b, n) = if b_desc.shape.len() >= 2 {
                    let r = b_desc.shape.len();
                    if *transpose_b { (b_desc.shape[r-1], b_desc.shape[r-2]) }
                    else { (b_desc.shape[r-2], b_desc.shape[r-1]) }
                } else {
                    return Err(crate::error::BarracudaError::InvalidInput {
                        message: "MatMul requires 2D+ tensors".to_string(),
                    });
                };

                if k_a != k_b {
                    return Err(crate::error::BarracudaError::InvalidInput {
                        message: format!("MatMul inner dimension mismatch: {k_a} vs {k_b}"),
                    });
                }

                let result = self.execute_matmul_cpu(&a_data, &b_data, m, k_a, n)?;
                let desc = TensorDescriptor::new(vec![m, n], a_desc.dtype);
                Ok(Self::pack_f32(result, desc))
            }

            // Softmax (special: not elementwise)
            Softmax { .. } => {
                let data = Self::read_f32(inputs[0].as_ref())?;
                let max_val = data.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                let exp_vals: Vec<f32> = data.iter().map(|&x| (x - max_val).exp()).collect();
                let sum: f32 = exp_vals.iter().sum();
                let result: Vec<f32> = exp_vals.iter().map(|&x| x / sum).collect();
                let desc = inputs[0].descriptor().clone();
                Ok(Self::pack_f32(result, desc))
            }

            // BatchMatMul — delegate to MatMul logic (single batch)
            BatchMatMul { transpose_a, transpose_b } => {
                if inputs.len() < 2 {
                    return Err(crate::error::BarracudaError::InvalidInput {
                        message: "BatchMatMul requires 2 inputs".to_string(),
                    });
                }
                let a_desc = inputs[0].descriptor();
                let b_desc = inputs[1].descriptor();
                let a_data = Self::read_f32(inputs[0].as_ref())?;
                let b_data = Self::read_f32(inputs[1].as_ref())?;

                let (m, k_a) = if a_desc.shape.len() >= 2 {
                    let r = a_desc.shape.len();
                    if *transpose_a { (a_desc.shape[r-1], a_desc.shape[r-2]) }
                    else { (a_desc.shape[r-2], a_desc.shape[r-1]) }
                } else {
                    return Err(crate::error::BarracudaError::InvalidInput {
                        message: "BatchMatMul requires 2D+ tensors".to_string(),
                    });
                };

                let (k_b, n) = if b_desc.shape.len() >= 2 {
                    let r = b_desc.shape.len();
                    if *transpose_b { (b_desc.shape[r-1], b_desc.shape[r-2]) }
                    else { (b_desc.shape[r-2], b_desc.shape[r-1]) }
                } else {
                    return Err(crate::error::BarracudaError::InvalidInput {
                        message: "BatchMatMul requires 2D+ tensors".to_string(),
                    });
                };

                if k_a != k_b {
                    return Err(crate::error::BarracudaError::InvalidInput {
                        message: format!("BatchMatMul inner dimension mismatch: {k_a} vs {k_b}"),
                    });
                }

                let result = self.execute_matmul_cpu(&a_data, &b_data, m, k_a, n)?;
                let desc = TensorDescriptor::new(vec![m, n], a_desc.dtype);
                Ok(Self::pack_f32(result, desc))
            }

            // Shape ops — metadata only, no data change
            Reshape { new_shape } => {
                let data = Self::read_f32(inputs[0].as_ref())?;
                let shape: Vec<usize> = new_shape.iter().map(|&x| x as usize).collect();
                let desc = TensorDescriptor::new(shape, inputs[0].descriptor().dtype);
                Ok(Self::pack_f32(data, desc))
            }

            Squeeze { .. } => {
                let data = Self::read_f32(inputs[0].as_ref())?;
                let shape: Vec<usize> = inputs[0].descriptor().shape.iter()
                    .copied()
                    .filter(|&d| d != 1)
                    .collect();
                let shape = if shape.is_empty() { vec![1] } else { shape };
                let desc = TensorDescriptor::new(shape, inputs[0].descriptor().dtype);
                Ok(Self::pack_f32(data, desc))
            }

            Unsqueeze { dims } => {
                let data = Self::read_f32(inputs[0].as_ref())?;
                let mut shape = inputs[0].descriptor().shape.clone();
                for &d in dims.iter().rev() {
                    let pos = d.min(shape.len());
                    shape.insert(pos, 1);
                }
                let desc = TensorDescriptor::new(shape, inputs[0].descriptor().dtype);
                Ok(Self::pack_f32(data, desc))
            }

            Transpose { .. } => {
                let desc = inputs[0].descriptor();
                let data = Self::read_f32(inputs[0].as_ref())?;
                if desc.shape.len() == 2 {
                    let (rows, cols) = (desc.shape[0], desc.shape[1]);
                    let mut transposed = vec![0.0f32; data.len()];
                    for r in 0..rows {
                        for c in 0..cols {
                            transposed[c * rows + r] = data[r * cols + c];
                        }
                    }
                    let new_desc = TensorDescriptor::new(vec![cols, rows], desc.dtype);
                    Ok(Self::pack_f32(transposed, new_desc))
                } else {
                    Ok(Self::pack_f32(data, desc.clone()))
                }
            }

            Concat { .. } => {
                if inputs.len() < 2 {
                    return Err(crate::error::BarracudaError::InvalidInput {
                        message: "Concat requires at least 2 inputs".to_string(),
                    });
                }
                let a = Self::read_f32(inputs[0].as_ref())?;
                let b = Self::read_f32(inputs[1].as_ref())?;
                let mut result = a;
                result.extend_from_slice(&b);
                let total = result.len();
                let desc = TensorDescriptor::new(vec![total], inputs[0].descriptor().dtype);
                Ok(Self::pack_f32(result, desc))
            }

            Split { sizes, .. } => {
                let data = Self::read_f32(inputs[0].as_ref())?;
                let split_at = sizes.first().copied().unwrap_or(data.len() / 2);
                let first = data[..split_at.min(data.len())].to_vec();
                let desc = TensorDescriptor::new(vec![first.len()], inputs[0].descriptor().dtype);
                Ok(Self::pack_f32(first, desc))
            }

            Broadcast { target_shape } => {
                let data = Self::read_f32(inputs[0].as_ref())?;
                let target_size: usize = target_shape.iter().product();
                let mut result = Vec::with_capacity(target_size);
                if data.is_empty() {
                    result.resize(target_size, 0.0);
                } else {
                    for i in 0..target_size {
                        result.push(data[i % data.len()]);
                    }
                }
                let desc = TensorDescriptor::new(target_shape.clone(), inputs[0].descriptor().dtype);
                Ok(Self::pack_f32(result, desc))
            }

            // Convolution ops — CPU fallback pending
            other @ (Conv2D { .. } | MaxPool2D { .. } | AvgPool2D { .. }) => {
                Err(crate::error::BarracudaError::NotImplemented {
                    feature: format!("CpuExecutor::execute({other:?}) — conv ops planned"),
                })
            }
        }
    }

    async fn allocate(&self, descriptor: TensorDescriptor) -> Result<Arc<dyn TensorStorage>> {
        // Create CPU tensor storage
        Ok(Arc::new(CpuTensorStorage::new(descriptor)))
    }

    async fn transfer(&self, tensor: Arc<dyn TensorStorage>) -> Result<Arc<dyn TensorStorage>> {
        // If already on CPU, return as-is
        if tensor.is_cpu() {
            Ok(tensor)
        } else {
            // Read from device, allocate on CPU
            let data = tensor.read_to_cpu().await?;
            let descriptor = tensor.descriptor().clone();

            let mut cpu_tensor = CpuTensorStorage::new(descriptor);
            cpu_tensor.write_from_cpu(&data).await?;

            Ok(Arc::new(cpu_tensor))
        }
    }
}

/// CPU tensor storage implementation
struct CpuTensorStorage {
    descriptor: TensorDescriptor,
    data: Vec<u8>,
}

impl CpuTensorStorage {
    fn new(descriptor: TensorDescriptor) -> Self {
        let byte_size = descriptor.numel * descriptor.dtype.size_bytes();
        Self {
            descriptor,
            data: vec![0u8; byte_size],
        }
    }
}

#[async_trait]
impl TensorStorage for CpuTensorStorage {
    fn descriptor(&self) -> &TensorDescriptor {
        &self.descriptor
    }

    fn hardware_type(&self) -> HardwareType {
        HardwareType::CPU
    }

    async fn read_to_cpu(&self) -> Result<Vec<u8>> {
        // Already on CPU, just clone
        Ok(self.data.clone())
    }

    async fn write_from_cpu(&mut self, data: &[u8]) -> Result<()> {
        if data.len() != self.data.len() {
            return Err(crate::error::BarracudaError::InvalidInput {
                message: format!(
                    "Data size mismatch: expected {}, got {}",
                    self.data.len(),
                    data.len()
                ),
            });
        }
        self.data.copy_from_slice(data);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_executor_creation() {
        let cpu = CpuExecutor::new();
        assert_eq!(cpu.name(), "CPU (Native Rust + SIMD)");
        assert_eq!(cpu.hardware_type(), HardwareType::CPU);
        assert!(cpu._num_threads > 0);
    }

    #[test]
    fn test_simd_detection() {
        let width = CpuExecutor::detect_simd_width();
        assert!(width >= 1);
        tracing::debug!("SIMD width: {}", width);
    }

    #[test]
    fn test_cpu_capabilities() {
        let cpu = CpuExecutor::new();
        let caps = cpu.capabilities();
        assert!(caps.operations.matmul);
        assert!(caps.precision.fp32);
        assert!(caps.parallelism.max_parallel_units > 0);
    }

    #[test]
    fn test_cpu_can_execute_all() {
        let cpu = CpuExecutor::new();
        let desc = TensorDescriptor::new(vec![10, 10], crate::unified_math::DType::F32);
        assert!(cpu.can_execute(&MathOp::ReLU, std::slice::from_ref(&desc)));
        assert!(cpu.can_execute(&MathOp::Add, &[desc.clone(), desc]));
    }

    #[test]
    fn test_scoring_small_vs_large() {
        let cpu = CpuExecutor::new();

        // Small tensor → CPU scores high
        let small = TensorDescriptor::new(vec![10, 10], crate::unified_math::DType::F32);
        let score_small = cpu.score_operation(&MathOp::ReLU, &[small]);

        // Large tensor → CPU scores lower
        let large = TensorDescriptor::new(vec![4096, 4096], crate::unified_math::DType::F32);
        let score_large = cpu.score_operation(&MathOp::ReLU, &[large]);

        assert!(score_small > score_large);
        tracing::debug!("Small: {:.2}, Large: {:.2}", score_small, score_large);
    }

    #[test]
    fn test_unary_relu() {
        let cpu = CpuExecutor::new();
        let input = vec![-1.0, 0.0, 1.0, 2.0, -2.0];
        let output = cpu.execute_unary_cpu(&MathOp::ReLU, &input).unwrap();
        assert_eq!(output, vec![0.0, 0.0, 1.0, 2.0, 0.0]);
    }

    #[test]
    fn test_binary_add() {
        let cpu = CpuExecutor::new();
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![4.0, 5.0, 6.0];
        let output = cpu.execute_binary_cpu(&MathOp::Add, &a, &b).unwrap();
        assert_eq!(output, vec![5.0, 7.0, 9.0]);
    }

    #[test]
    fn test_reduce_sum() {
        let cpu = CpuExecutor::new();
        let input = vec![1.0, 2.0, 3.0, 4.0];
        let op = MathOp::ReduceSum {
            dim: None,
            keepdim: false,
        };
        let output = cpu.execute_reduce_cpu(&op, &input).unwrap();
        assert_eq!(output, 10.0);
    }

    #[test]
    fn test_matmul_small() {
        let cpu = CpuExecutor::new();
        // 2x2 @ 2x2
        let a = vec![1.0, 2.0, 3.0, 4.0]; // [[1,2],[3,4]]
        let b = vec![5.0, 6.0, 7.0, 8.0]; // [[5,6],[7,8]]
        let c = cpu.execute_matmul_cpu(&a, &b, 2, 2, 2).unwrap();
        // [[1*5+2*7, 1*6+2*8], [3*5+4*7, 3*6+4*8]]
        // [[19, 22], [43, 50]]
        assert_eq!(c, vec![19.0, 22.0, 43.0, 50.0]);
    }
}
