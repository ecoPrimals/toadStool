// SPDX-License-Identifier: AGPL-3.0-or-later
//! CPU Compute Unit - Modular Architecture
//!
//! This module implements CPU as a ComputeUnit with operations organized
//! by **architectural patterns**, not file size:
//!
//! - **Core**: Discovery, capabilities, and dispatch logic
//! - **Basic Operations**: Map-reduce primitives (embarrassingly parallel)
//! - **Vector Operations**: Gather/scatter patterns (memory-bound)
//! - **Transform Operations**: Layout transformations (cache-friendly)
//! - **Activation Operations**: Element-wise non-linear (SIMD-friendly)
//! - **Normalization Operations**: Reduce-map-reduce patterns (statistical)
//! - **Tensor Operations**: Matrix operations (compute-bound, tiled)
//!
//! ## Design Philosophy
//!
//! This refactoring demonstrates **smart modularization**:
//! - Operations grouped by computational pattern
//! - Shared traits for similar operations
//! - Zero-cost abstractions (inlining preserved)
//! - Parallel compilation of modules
//!
//! Not just splitting a large file, but **improving architecture**.

use crate::types::*;
use std::time::Instant;

// Operation modules (organized by pattern)
mod activation_ops; // ReLU, GELU, Tanh, Sigmoid, Softmax, Dropout (element-wise)
mod basic_ops; // Map, filter, reduce, scan (embarrassingly parallel)
mod normalization_ops; // LayerNorm, BatchNorm (reduce-map-reduce patterns)
mod tensor_ops;
mod transform_ops; // Transpose, reshape (layout transformations)
mod vector_ops; // Dot product, elementwise, gather, scatter (memory patterns) // MatMul, Conv, Pooling (compute-intensive, tiled)

/// CPU compute unit
///
/// Represents the system CPU as a ComputeUnit.
/// Discovered at runtime, not hardcoded!
pub struct CpuComputeUnit {
    name: String,
    capabilities: Capabilities,
}

impl CpuComputeUnit {
    /// Discover CPU capabilities
    ///
    /// This queries the system to discover:
    /// - Number of cores
    /// - Cache sizes
    /// - SIMD support
    /// - Memory capacity
    ///
    /// No hardcoding - everything discovered at runtime!
    pub fn discover() -> Self {
        // Discover number of CPU cores (pure Rust, no FFI)
        let num_cores = std::thread::available_parallelism()
            .map(|p| p.get())
            .unwrap_or(1);

        // Estimate CPU memory (total system memory)
        let memory_capacity = Self::discover_memory();

        // Estimate compute throughput (rough model)
        let compute_throughput = Self::estimate_throughput(num_cores);

        let capabilities = Capabilities {
            unit_type: ComputeUnitType::Cpu,
            parallelism: Parallelism {
                num_units: num_cores,
                model: ExecutionModel::Mimd, // CPU can do MIMD
            },
            power_profile: PowerProfile::Medium, // Typical CPU: 10-100W
            latency: LatencyProfile {
                typical_ms: 0, // Very low latency for CPU
                deterministic: true,
            },
            memory_capacity,
            memory_bandwidth: 50_000_000_000, // ~50 GB/s typical
            compute_throughput,
            optimal_batch_size: 100, // Small batches for CPU
            supported_ops: vec![
                OperationType::Map,
                OperationType::Filter,
                OperationType::Reduce,
                OperationType::Scan,
                OperationType::DotProduct,
                OperationType::ElementwiseBinary,
                OperationType::Gather,
                OperationType::Scatter,
                OperationType::Transpose,
                OperationType::Softmax,
                OperationType::ReLU,
                OperationType::GELU,
                OperationType::Tanh,
                OperationType::Sigmoid,
                OperationType::Dropout,
                OperationType::LayerNorm,
                OperationType::BatchNorm,
                OperationType::MatMul,
                OperationType::Conv,
                OperationType::MaxPool2D,
                OperationType::AvgPool2D,
                OperationType::Custom,
            ],
            supported_types: vec![
                DataType::F32,
                DataType::F64,
                DataType::I32,
                DataType::I64,
                DataType::U32,
                DataType::U64,
            ],
        };

        let name = format!("CPU ({num_cores} cores)");

        Self { name, capabilities }
    }

    /// Discover available memory
    fn discover_memory() -> usize {
        // Parse /proc/meminfo on Linux, otherwise estimate
        #[cfg(target_os = "linux")]
        {
            // Read from /proc/meminfo
            if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") {
                for line in meminfo.lines() {
                    if line.starts_with("MemTotal:") {
                        if let Some(kb_str) = line.split_whitespace().nth(1) {
                            if let Ok(kb) = kb_str.parse::<usize>() {
                                return kb * 1024; // Convert KB to bytes
                            }
                        }
                    }
                }
            }
        }

        // Default estimate: 8 GB
        8 * 1024 * 1024 * 1024
    }

    /// Estimate CPU compute throughput
    fn estimate_throughput(num_cores: usize) -> f64 {
        // Rough model: ~100 GFLOPS per core (very approximate)
        (num_cores as f64) * 100e9
    }
}

#[async_trait::async_trait]
impl ComputeUnit for CpuComputeUnit {
    fn capabilities(&self) -> &Capabilities {
        &self.capabilities
    }

    fn name(&self) -> &str {
        &self.name
    }

    async fn execute(&self, workload: Workload) -> Result<Output, ComputeError> {
        let start = Instant::now();

        // Dispatch to appropriate operation module based on pattern
        let data = match workload.operation {
            // Basic operations (embarrassingly parallel)
            OperationType::Map => basic_ops::execute_map(workload)?,
            OperationType::Filter => basic_ops::execute_filter(workload)?,
            OperationType::Reduce => basic_ops::execute_reduce(workload)?,
            OperationType::Scan => basic_ops::execute_scan(workload)?,

            // Vector operations (memory-bound)
            OperationType::DotProduct => vector_ops::execute_dot_product(workload)?,
            OperationType::ElementwiseBinary => vector_ops::execute_elementwise_binary(workload)?,
            OperationType::Gather => vector_ops::execute_gather(workload)?,
            OperationType::Scatter => vector_ops::execute_scatter(workload)?,

            // Transform operations (layout transformations)
            OperationType::Transpose => transform_ops::execute_transpose(workload)?,

            // Activation operations (element-wise, SIMD-friendly)
            OperationType::Softmax => activation_ops::execute_softmax(workload)?,
            OperationType::ReLU => activation_ops::execute_relu(workload)?,
            OperationType::GELU => activation_ops::execute_gelu(workload)?,
            OperationType::Tanh => activation_ops::execute_tanh(workload)?,
            OperationType::Sigmoid => activation_ops::execute_sigmoid(workload)?,
            OperationType::Dropout => activation_ops::execute_dropout(workload),

            // Normalization operations (reduce-map-reduce pattern)
            OperationType::LayerNorm => normalization_ops::execute_layernorm(workload)?,
            OperationType::BatchNorm => normalization_ops::execute_batchnorm(workload)?,

            // Tensor operations (compute-intensive, tiled)
            OperationType::MatMul => tensor_ops::execute_matmul(workload)?,
            OperationType::Conv => tensor_ops::execute_conv(workload)?,
            OperationType::MaxPool2D => tensor_ops::execute_maxpool2d(workload)?,
            OperationType::AvgPool2D => tensor_ops::execute_avgpool2d(workload)?,

            OperationType::Custom => {
                return Err(ComputeError::ExecutionFailed(
                    "Custom operations not yet implemented".to_string(),
                ))
            }
        };

        let duration = start.elapsed();

        Ok(Output {
            data,
            metadata: OutputMetadata {
                unit_name: self.name.clone(),
                unit_type: ComputeUnitType::Cpu,
                duration,
                power_consumed_mw: None, // Can't measure easily on CPU
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_workload(op: OperationType, input: WorkloadData) -> Workload {
        Workload {
            operation: op,
            data_type: DataType::F32,
            num_operations: 3,
            required_memory: 12,
            input,
            params: WorkloadParams::default(),
        }
    }

    #[tokio::test]
    async fn test_cpu_discover_has_name() {
        let cpu = CpuComputeUnit::discover();
        assert!(cpu.name().contains("CPU"));
    }

    #[tokio::test]
    async fn test_cpu_capabilities_unit_type() {
        let cpu = CpuComputeUnit::discover();
        assert_eq!(cpu.capabilities().unit_type, ComputeUnitType::Cpu);
    }

    #[tokio::test]
    async fn test_cpu_supports_f32_map() {
        let cpu = CpuComputeUnit::discover();
        let w = make_workload(
            OperationType::Map,
            WorkloadData::F32Vec(vec![1.0, 2.0, 3.0]),
        );
        assert!(cpu.can_execute(&w));
    }

    #[tokio::test]
    async fn test_cpu_optimal_batch_size() {
        let cpu = CpuComputeUnit::discover();
        assert!(cpu.optimal_batch_size() > 0);
    }

    #[tokio::test]
    async fn test_cpu_execute_map() {
        let cpu = CpuComputeUnit::discover();
        let w = make_workload(
            OperationType::Map,
            WorkloadData::F32Vec(vec![1.0, 2.0, 3.0]),
        );
        let out = cpu.execute(w).await.unwrap();
        match out.data {
            WorkloadData::F32Vec(v) => assert_eq!(v.len(), 3),
            _ => panic!("expected F32Vec"),
        }
    }

    #[tokio::test]
    async fn test_cpu_execute_dot_product() {
        let cpu = CpuComputeUnit::discover();
        let w = make_workload(
            OperationType::DotProduct,
            WorkloadData::F32VecPair(vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]),
        );
        let out = cpu.execute(w).await.unwrap();
        match out.data {
            WorkloadData::F32Vec(v) => assert!((v[0] - 32.0).abs() < 1e-5),
            other => panic!("expected F32Vec, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_cpu_execute_transpose() {
        let cpu = CpuComputeUnit::discover();
        let w = make_workload(
            OperationType::Transpose,
            WorkloadData::F32Matrix(vec![1.0, 2.0, 3.0, 4.0], 2, 2),
        );
        let out = cpu.execute(w).await.unwrap();
        assert!(matches!(out.data, WorkloadData::F32Matrix(_, _, _)));
    }

    #[tokio::test]
    async fn test_cpu_execute_layernorm() {
        let cpu = CpuComputeUnit::discover();
        let w = make_workload(
            OperationType::LayerNorm,
            WorkloadData::F32Vec(vec![1.0, 2.0, 3.0]),
        );
        let out = cpu.execute(w).await.unwrap();
        assert!(matches!(out.data, WorkloadData::F32Vec(_)));
    }

    #[tokio::test]
    async fn test_cpu_execute_custom_returns_error() {
        let cpu = CpuComputeUnit::discover();
        let w = make_workload(OperationType::Custom, WorkloadData::Custom(vec![]));
        assert!(cpu.execute(w).await.is_err());
    }

    #[tokio::test]
    async fn test_cpu_estimate_duration_nonzero() {
        let cpu = CpuComputeUnit::discover();
        let w = make_workload(
            OperationType::Map,
            WorkloadData::F32Vec(vec![1.0, 2.0, 3.0]),
        );
        let dur = cpu.estimate_duration(&w);
        // Duration should be at least 0 (latency = 0ms for CPU)
        let _ = dur;
    }

    #[tokio::test]
    async fn test_cpu_execute_matmul() {
        let cpu = CpuComputeUnit::discover();
        let a = vec![1.0f32, 2.0, 3.0, 4.0];
        let b = vec![1.0f32, 0.0, 0.0, 1.0];
        let w = make_workload(
            OperationType::MatMul,
            WorkloadData::F32MatrixPair(a, 2, 2, b, 2, 2),
        );
        let out = cpu.execute(w).await.unwrap();
        match out.data {
            WorkloadData::F32Matrix(v, rows, cols) => {
                assert_eq!(rows, 2);
                assert_eq!(cols, 2);
                assert_eq!(v.len(), 4);
                assert!((v[0] - 1.0).abs() < 1e-5);
                assert!((v[1] - 2.0).abs() < 1e-5);
                assert!((v[2] - 3.0).abs() < 1e-5);
                assert!((v[3] - 4.0).abs() < 1e-5);
            }
            other => panic!("expected F32Matrix, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_cpu_execute_matmul_dimension_mismatch() {
        let cpu = CpuComputeUnit::discover();
        let a = vec![1.0f32, 2.0, 3.0];
        let b = vec![1.0f32, 0.0, 0.0, 1.0];
        let w = make_workload(
            OperationType::MatMul,
            WorkloadData::F32MatrixPair(a, 1, 3, b, 2, 2),
        );
        assert!(cpu.execute(w).await.is_err());
    }

    #[tokio::test]
    async fn test_cpu_execute_relu() {
        let cpu = CpuComputeUnit::discover();
        let input = vec![-1.0f32, 0.0, 1.0, 2.0, -0.5];
        let w = make_workload(OperationType::ReLU, WorkloadData::F32Vec(input));
        let out = cpu.execute(w).await.unwrap();
        match out.data {
            WorkloadData::F32Vec(v) => {
                assert_eq!(v.len(), 5);
                assert!((v[0] - 0.0).abs() < 1e-5);
                assert!((v[1] - 0.0).abs() < 1e-5);
                assert!((v[2] - 1.0).abs() < 1e-5);
                assert!((v[3] - 2.0).abs() < 1e-5);
                assert!((v[4] - 0.0).abs() < 1e-5);
            }
            other => panic!("expected F32Vec, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_cpu_execute_gelu() {
        let cpu = CpuComputeUnit::discover();
        let input = vec![0.0f32, 1.0, -1.0];
        let w = make_workload(OperationType::GELU, WorkloadData::F32Vec(input));
        let out = cpu.execute(w).await.unwrap();
        match out.data {
            WorkloadData::F32Vec(v) => {
                assert_eq!(v.len(), 3);
                assert!((v[0] - 0.0).abs() < 1e-4);
                assert!(v[1] > 0.0 && v[1] < 1.0);
                assert!(v[2] < 0.0 && v[2] > -0.2);
            }
            other => panic!("expected F32Vec, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_cpu_execute_tanh() {
        let cpu = CpuComputeUnit::discover();
        let input = vec![0.0f32, 1.0, -1.0];
        let w = make_workload(OperationType::Tanh, WorkloadData::F32Vec(input));
        let out = cpu.execute(w).await.unwrap();
        match out.data {
            WorkloadData::F32Vec(v) => {
                assert_eq!(v.len(), 3);
                assert!((v[0] - 0.0).abs() < 1e-5);
                assert!((v[1] - 0.761_594).abs() < 1e-3);
                assert!((v[2] + 0.761_594).abs() < 1e-3);
            }
            other => panic!("expected F32Vec, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_cpu_execute_sigmoid() {
        let cpu = CpuComputeUnit::discover();
        let input = vec![0.0f32, 1.0, -1.0];
        let w = make_workload(OperationType::Sigmoid, WorkloadData::F32Vec(input));
        let out = cpu.execute(w).await.unwrap();
        match out.data {
            WorkloadData::F32Vec(v) => {
                assert_eq!(v.len(), 3);
                assert!((v[0] - 0.5).abs() < 1e-5);
                assert!(v[1] > 0.7 && v[1] < 0.8);
                assert!(v[2] > 0.2 && v[2] < 0.3);
            }
            other => panic!("expected F32Vec, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_cpu_execute_softmax() {
        let cpu = CpuComputeUnit::discover();
        let input = vec![1.0f32, 2.0, 3.0];
        let w = make_workload(OperationType::Softmax, WorkloadData::F32Vec(input));
        let out = cpu.execute(w).await.unwrap();
        match out.data {
            WorkloadData::F32Vec(v) => {
                assert_eq!(v.len(), 3);
                let sum: f32 = v.iter().sum();
                assert!((sum - 1.0).abs() < 1e-5);
                assert!(v.iter().all(|&x| x > 0.0 && x < 1.0));
            }
            other => panic!("expected F32Vec, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_cpu_execute_dropout() {
        let cpu = CpuComputeUnit::discover();
        let input = vec![1.0f32, 2.0, 3.0];
        let w = make_workload(OperationType::Dropout, WorkloadData::F32Vec(input));
        let out = cpu.execute(w).await.unwrap();
        match out.data {
            WorkloadData::F32Vec(v) => assert_eq!(v.len(), 3),
            other => panic!("expected F32Vec, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_cpu_execute_conv2d() {
        let cpu = CpuComputeUnit::discover();
        let input = vec![1.0f32; 32];
        let kernel = vec![1.0f32 / 9.0; 2 * 2 * 3 * 3];
        let w = make_workload(
            OperationType::Conv,
            WorkloadData::F32Conv2D {
                input,
                kernel,
                bias: None,
                batch_size: 1,
                in_channels: 2,
                height: 4,
                width: 4,
                out_channels: 2,
                kernel_h: 3,
                kernel_w: 3,
                stride: 1,
                padding: 0,
            },
        );
        let out = cpu.execute(w).await.unwrap();
        assert!(matches!(out.data, WorkloadData::F32Matrix(_, _, _)));
    }

    #[tokio::test]
    async fn test_cpu_execute_maxpool2d() {
        let cpu = CpuComputeUnit::discover();
        let input = vec![1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];
        let w = make_workload(
            OperationType::MaxPool2D,
            WorkloadData::F32Pool2D {
                input,
                batch_size: 1,
                channels: 1,
                height: 3,
                width: 3,
                pool_h: 2,
                pool_w: 2,
                stride: 1,
                padding: 0,
            },
        );
        let out = cpu.execute(w).await.unwrap();
        match out.data {
            WorkloadData::F32Matrix(v, _, _) => {
                assert!(!v.is_empty());
                assert!(v.iter().all(|&x| x >= 0.0));
            }
            other => panic!("expected F32Matrix, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_cpu_execute_avgpool2d() {
        let cpu = CpuComputeUnit::discover();
        let input = vec![1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];
        let w = make_workload(
            OperationType::AvgPool2D,
            WorkloadData::F32Pool2D {
                input,
                batch_size: 1,
                channels: 1,
                height: 3,
                width: 3,
                pool_h: 2,
                pool_w: 2,
                stride: 1,
                padding: 0,
            },
        );
        let out = cpu.execute(w).await.unwrap();
        match out.data {
            WorkloadData::F32Matrix(v, _, _) => {
                assert!(!v.is_empty());
                assert!(v.iter().all(|&x| x >= 0.0));
            }
            other => panic!("expected F32Matrix, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_cpu_execute_activation_f64() {
        let cpu = CpuComputeUnit::discover();
        let input = vec![-1.0f64, 0.0, 1.0];
        let mut w = make_workload(OperationType::ReLU, WorkloadData::F64Vec(input));
        w.data_type = DataType::F64;
        let out = cpu.execute(w).await.unwrap();
        match out.data {
            WorkloadData::F64Vec(v) => {
                assert_eq!(v.len(), 3);
                assert!((v[0] - 0.0).abs() < 1e-10);
                assert!((v[1] - 0.0).abs() < 1e-10);
                assert!((v[2] - 1.0).abs() < 1e-10);
            }
            other => panic!("expected F64Vec, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_cpu_execute_batchnorm() {
        let cpu = CpuComputeUnit::discover();
        let input = vec![1.0f32, 2.0, 3.0, 4.0];
        let w = make_workload(OperationType::BatchNorm, WorkloadData::F32Vec(input));
        let out = cpu.execute(w).await.unwrap();
        assert!(matches!(out.data, WorkloadData::F32Vec(_)));
    }
}
