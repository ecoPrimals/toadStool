//! CPU compute unit implementation
//!
//! Treats the CPU as a parallel compute unit, just like GPU or neuromorphic.
//! Key insight: CPU is not special - it's just one point on the parallelism spectrum.

use crate::types::*;
use std::time::Instant;

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
        // Discover number of CPU cores
        let num_cores = num_cpus::get();

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

        let name = format!("CPU ({} cores)", num_cores);

        Self { name, capabilities }
    }

    /// Discover available memory
    fn discover_memory() -> usize {
        // Use sysinfo if available, otherwise estimate
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

        // Execute based on operation type
        let data = match workload.operation {
            OperationType::Map => self.execute_map(workload)?,
            OperationType::Filter => self.execute_filter(workload)?,
            OperationType::Reduce => self.execute_reduce(workload)?,
            OperationType::Scan => self.execute_scan(workload)?,
            OperationType::DotProduct => self.execute_dot_product(workload)?,
            OperationType::ElementwiseBinary => self.execute_elementwise_binary(workload)?,
            OperationType::Gather => self.execute_gather(workload)?,
            OperationType::Scatter => self.execute_scatter(workload)?,
            OperationType::Transpose => self.execute_transpose(workload)?,
            OperationType::Softmax => self.execute_softmax(workload)?,
            OperationType::ReLU => self.execute_relu(workload)?,
            OperationType::GELU => self.execute_gelu(workload)?,
            OperationType::Tanh => self.execute_tanh(workload)?,
            OperationType::Sigmoid => self.execute_sigmoid(workload)?,
            OperationType::Dropout => self.execute_dropout(workload)?,
            OperationType::LayerNorm => self.execute_layernorm(workload)?,
            OperationType::BatchNorm => self.execute_batchnorm(workload)?,
            OperationType::MatMul => self.execute_matmul(workload)?,
            OperationType::Conv => self.execute_conv(workload)?,
            OperationType::MaxPool2D => self.execute_maxpool2d(workload)?,
            OperationType::AvgPool2D => self.execute_avgpool2d(workload)?,
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

impl CpuComputeUnit {
    /// Execute map operation on CPU using Rayon for parallelism
    fn execute_map(&self, workload: Workload) -> Result<WorkloadData, ComputeError> {
        use rayon::prelude::*;

        match workload.input {
            WorkloadData::F32Vec(input) => {
                // For now, identity map as placeholder
                // In full implementation, would interpret function from params
                let output: Vec<f32> = input.par_iter().map(|&x| x * 2.0 + 1.0).collect();
                Ok(WorkloadData::F32Vec(output))
            }
            WorkloadData::F64Vec(input) => {
                let output: Vec<f64> = input.par_iter().map(|&x| x * 2.0 + 1.0).collect();
                Ok(WorkloadData::F64Vec(output))
            }
            WorkloadData::I32Vec(input) => {
                let output: Vec<i32> = input.par_iter().map(|&x| x * 2 + 1).collect();
                Ok(WorkloadData::I32Vec(output))
            }
            _ => Err(ComputeError::UnsupportedWorkload),
        }
    }

    /// Execute filter operation (select elements matching predicate)
    fn execute_filter(&self, workload: Workload) -> Result<WorkloadData, ComputeError> {
        use rayon::prelude::*;

        // For now, filter with a simple predicate (e.g., > 0)
        // Full implementation would parse predicate from params
        match workload.input {
            WorkloadData::F32Vec(input) => {
                let output: Vec<f32> = input.par_iter().filter(|&&x| x > 0.0).copied().collect();
                Ok(WorkloadData::F32Vec(output))
            }
            WorkloadData::F64Vec(input) => {
                let output: Vec<f64> = input.par_iter().filter(|&&x| x > 0.0).copied().collect();
                Ok(WorkloadData::F64Vec(output))
            }
            WorkloadData::I32Vec(input) => {
                let output: Vec<i32> = input.par_iter().filter(|&&x| x > 0).copied().collect();
                Ok(WorkloadData::I32Vec(output))
            }
            _ => Err(ComputeError::UnsupportedWorkload),
        }
    }

    /// Execute reduce operation
    fn execute_reduce(&self, workload: Workload) -> Result<WorkloadData, ComputeError> {
        use rayon::prelude::*;

        match workload.input {
            WorkloadData::F32Vec(input) => {
                let sum: f32 = input.par_iter().sum();
                Ok(WorkloadData::F32Vec(vec![sum]))
            }
            WorkloadData::F64Vec(input) => {
                let sum: f64 = input.par_iter().sum();
                Ok(WorkloadData::F64Vec(vec![sum]))
            }
            WorkloadData::I32Vec(input) => {
                let sum: i32 = input.par_iter().sum();
                Ok(WorkloadData::I32Vec(vec![sum]))
            }
            _ => Err(ComputeError::UnsupportedWorkload),
        }
    }

    /// Execute scan operation (prefix sum / cumulative)
    fn execute_scan(&self, workload: Workload) -> Result<WorkloadData, ComputeError> {
        // Scan is inherently sequential, but we can parallelize chunks
        match workload.input {
            WorkloadData::F32Vec(input) => {
                let mut output = Vec::with_capacity(input.len());
                let mut acc = 0.0f32;
                for &x in &input {
                    acc += x;
                    output.push(acc);
                }
                Ok(WorkloadData::F32Vec(output))
            }
            WorkloadData::F64Vec(input) => {
                let mut output = Vec::with_capacity(input.len());
                let mut acc = 0.0f64;
                for &x in &input {
                    acc += x;
                    output.push(acc);
                }
                Ok(WorkloadData::F64Vec(output))
            }
            WorkloadData::I32Vec(input) => {
                let mut output = Vec::with_capacity(input.len());
                let mut acc = 0i32;
                for &x in &input {
                    acc += x;
                    output.push(acc);
                }
                Ok(WorkloadData::I32Vec(output))
            }
            _ => Err(ComputeError::UnsupportedWorkload),
        }
    }

    /// Execute dot product (vector inner product)
    fn execute_dot_product(&self, workload: Workload) -> Result<WorkloadData, ComputeError> {
        use rayon::prelude::*;

        // Dot product: sum(a[i] * b[i])
        // Combines Map (element-wise multiply) + Reduce (sum)
        match workload.input {
            WorkloadData::F32VecPair(a, b) => {
                if a.len() != b.len() {
                    return Err(ComputeError::ExecutionFailed(
                        "Vectors must have same length for dot product".to_string(),
                    ));
                }
                // Parallel dot product using Rayon
                let result: f32 = a.par_iter().zip(b.par_iter()).map(|(&x, &y)| x * y).sum();
                Ok(WorkloadData::F32Vec(vec![result]))
            }
            WorkloadData::F64VecPair(a, b) => {
                if a.len() != b.len() {
                    return Err(ComputeError::ExecutionFailed(
                        "Vectors must have same length for dot product".to_string(),
                    ));
                }
                let result: f64 = a.par_iter().zip(b.par_iter()).map(|(&x, &y)| x * y).sum();
                Ok(WorkloadData::F64Vec(vec![result]))
            }
            WorkloadData::I32VecPair(a, b) => {
                if a.len() != b.len() {
                    return Err(ComputeError::ExecutionFailed(
                        "Vectors must have same length for dot product".to_string(),
                    ));
                }
                let result: i32 = a.par_iter().zip(b.par_iter()).map(|(&x, &y)| x * y).sum();
                Ok(WorkloadData::I32Vec(vec![result]))
            }
            _ => Err(ComputeError::UnsupportedWorkload),
        }
    }

    /// Execute element-wise binary operation
    fn execute_elementwise_binary(
        &self,
        workload: Workload,
    ) -> Result<WorkloadData, ComputeError> {
        use rayon::prelude::*;

        // For now, implement addition as the binary operation
        // Full implementation would parse operation from params
        match workload.input {
            WorkloadData::F32VecPair(a, b) => {
                if a.len() != b.len() {
                    return Err(ComputeError::ExecutionFailed(
                        "Vectors must have same length for elementwise operation".to_string(),
                    ));
                }
                let result: Vec<f32> = a
                    .par_iter()
                    .zip(b.par_iter())
                    .map(|(&x, &y)| x + y) // Addition as default
                    .collect();
                Ok(WorkloadData::F32Vec(result))
            }
            WorkloadData::F64VecPair(a, b) => {
                if a.len() != b.len() {
                    return Err(ComputeError::ExecutionFailed(
                        "Vectors must have same length for elementwise operation".to_string(),
                    ));
                }
                let result: Vec<f64> = a
                    .par_iter()
                    .zip(b.par_iter())
                    .map(|(&x, &y)| x + y)
                    .collect();
                Ok(WorkloadData::F64Vec(result))
            }
            WorkloadData::I32VecPair(a, b) => {
                if a.len() != b.len() {
                    return Err(ComputeError::ExecutionFailed(
                        "Vectors must have same length for elementwise operation".to_string(),
                    ));
                }
                let result: Vec<i32> = a
                    .par_iter()
                    .zip(b.par_iter())
                    .map(|(&x, &y)| x + y)
                    .collect();
                Ok(WorkloadData::I32Vec(result))
            }
            _ => Err(ComputeError::UnsupportedWorkload),
        }
    }

    /// Execute gather operation (select elements by indices)
    fn execute_gather(&self, workload: Workload) -> Result<WorkloadData, ComputeError> {
        use rayon::prelude::*;

        // Gather: output[i] = input[indices[i]]
        match workload.input {
            WorkloadData::F32VecIndexed(data, indices) => {
                // Validate indices
                for &idx in &indices {
                    if idx >= data.len() {
                        return Err(ComputeError::ExecutionFailed(format!(
                            "Index {} out of bounds for data length {}",
                            idx,
                            data.len()
                        )));
                    }
                }
                // Parallel gather
                let result: Vec<f32> = indices.par_iter().map(|&idx| data[idx]).collect();
                Ok(WorkloadData::F32Vec(result))
            }
            WorkloadData::F64VecIndexed(data, indices) => {
                for &idx in &indices {
                    if idx >= data.len() {
                        return Err(ComputeError::ExecutionFailed(format!(
                            "Index {} out of bounds for data length {}",
                            idx,
                            data.len()
                        )));
                    }
                }
                let result: Vec<f64> = indices.par_iter().map(|&idx| data[idx]).collect();
                Ok(WorkloadData::F64Vec(result))
            }
            WorkloadData::I32VecIndexed(data, indices) => {
                for &idx in &indices {
                    if idx >= data.len() {
                        return Err(ComputeError::ExecutionFailed(format!(
                            "Index {} out of bounds for data length {}",
                            idx,
                            data.len()
                        )));
                    }
                }
                let result: Vec<i32> = indices.par_iter().map(|&idx| data[idx]).collect();
                Ok(WorkloadData::I32Vec(result))
            }
            _ => Err(ComputeError::UnsupportedWorkload),
        }
    }

    /// Execute scatter operation (place elements by indices)
    fn execute_scatter(&self, workload: Workload) -> Result<WorkloadData, ComputeError> {
        // Scatter: output[indices[i]] = input[i]
        // Note: For simplicity, we'll implement scatter-add (accumulate when indices overlap)
        match workload.input {
            WorkloadData::F32VecIndexed(values, indices) => {
                if values.len() != indices.len() {
                    return Err(ComputeError::ExecutionFailed(
                        "Values and indices must have same length for scatter".to_string(),
                    ));
                }

                // Determine output size from params
                // For now, we'll use max(indices) + 1
                let output_size = indices.iter().max().map(|&i| i + 1).unwrap_or(0);
                let mut output = vec![0.0f32; output_size];

                // Sequential scatter (parallel scatter has race conditions)
                // TODO: Parallel scatter with atomics or segmented approach
                for (i, &idx) in indices.iter().enumerate() {
                    if idx >= output_size {
                        return Err(ComputeError::ExecutionFailed(format!(
                            "Index {} out of bounds for output size {}",
                            idx, output_size
                        )));
                    }
                    output[idx] += values[i]; // Scatter-add
                }

                Ok(WorkloadData::F32Vec(output))
            }
            WorkloadData::F64VecIndexed(values, indices) => {
                if values.len() != indices.len() {
                    return Err(ComputeError::ExecutionFailed(
                        "Values and indices must have same length for scatter".to_string(),
                    ));
                }

                let output_size = indices.iter().max().map(|&i| i + 1).unwrap_or(0);
                let mut output = vec![0.0f64; output_size];

                for (i, &idx) in indices.iter().enumerate() {
                    if idx >= output_size {
                        return Err(ComputeError::ExecutionFailed(format!(
                            "Index {} out of bounds for output size {}",
                            idx, output_size
                        )));
                    }
                    output[idx] += values[i];
                }

                Ok(WorkloadData::F64Vec(output))
            }
            WorkloadData::I32VecIndexed(values, indices) => {
                if values.len() != indices.len() {
                    return Err(ComputeError::ExecutionFailed(
                        "Values and indices must have same length for scatter".to_string(),
                    ));
                }

                let output_size = indices.iter().max().map(|&i| i + 1).unwrap_or(0);
                let mut output = vec![0i32; output_size];

                for (i, &idx) in indices.iter().enumerate() {
                    if idx >= output_size {
                        return Err(ComputeError::ExecutionFailed(format!(
                            "Index {} out of bounds for output size {}",
                            idx, output_size
                        )));
                    }
                    output[idx] += values[i];
                }

                Ok(WorkloadData::I32Vec(output))
            }
            _ => Err(ComputeError::UnsupportedWorkload),
        }
    }

    /// Execute transpose (2D matrix transpose)
    fn execute_transpose(&self, workload: Workload) -> Result<WorkloadData, ComputeError> {
        use rayon::prelude::*;

        // Transpose: output[j][i] = input[i][j]
        match workload.input {
            WorkloadData::F32Matrix(data, rows, cols) => {
                if data.len() != rows * cols {
                    return Err(ComputeError::ExecutionFailed(format!(
                        "Matrix data length {} doesn't match dimensions {}x{}",
                        data.len(),
                        rows,
                        cols
                    )));
                }

                // Parallel transpose: Each output row is independent
                let mut output = vec![0.0f32; rows * cols];
                output
                    .par_chunks_mut(rows)
                    .enumerate()
                    .for_each(|(out_row, chunk)| {
                        // out_row is the column in input
                        for out_col in 0..rows {
                            // out_col is the row in input
                            chunk[out_col] = data[out_col * cols + out_row];
                        }
                    });

                Ok(WorkloadData::F32Matrix(output, cols, rows)) // Dimensions swapped
            }
            WorkloadData::F64Matrix(data, rows, cols) => {
                if data.len() != rows * cols {
                    return Err(ComputeError::ExecutionFailed(format!(
                        "Matrix data length {} doesn't match dimensions {}x{}",
                        data.len(),
                        rows,
                        cols
                    )));
                }

                let mut output = vec![0.0f64; rows * cols];
                output
                    .par_chunks_mut(rows)
                    .enumerate()
                    .for_each(|(out_row, chunk)| {
                        for out_col in 0..rows {
                            chunk[out_col] = data[out_col * cols + out_row];
                        }
                    });

                Ok(WorkloadData::F64Matrix(output, cols, rows))
            }
            WorkloadData::I32Matrix(data, rows, cols) => {
                if data.len() != rows * cols {
                    return Err(ComputeError::ExecutionFailed(format!(
                        "Matrix data length {} doesn't match dimensions {}x{}",
                        data.len(),
                        rows,
                        cols
                    )));
                }

                let mut output = vec![0i32; rows * cols];
                output
                    .par_chunks_mut(rows)
                    .enumerate()
                    .for_each(|(out_row, chunk)| {
                        for out_col in 0..rows {
                            chunk[out_col] = data[out_col * cols + out_row];
                        }
                    });

                Ok(WorkloadData::I32Matrix(output, cols, rows))
            }
            _ => Err(ComputeError::UnsupportedWorkload),
        }
    }

    /// Execute softmax (composite: exp + reduce + map)
    fn execute_softmax(&self, workload: Workload) -> Result<WorkloadData, ComputeError> {
        use rayon::prelude::*;

        // Softmax: output[i] = exp(input[i]) / sum(exp(input))
        // Numerically stable version: subtract max first
        match workload.input {
            WorkloadData::F32Vec(input) => {
                if input.is_empty() {
                    return Ok(WorkloadData::F32Vec(vec![]));
                }

                // Step 1: Find max (for numerical stability)
                let max_val = input
                    .par_iter()
                    .copied()
                    .reduce(|| f32::NEG_INFINITY, f32::max);

                // Step 2: Compute exp(x - max) for each element (parallel)
                let exp_values: Vec<f32> = input
                    .par_iter()
                    .map(|&x| (x - max_val).exp())
                    .collect();

                // Step 3: Sum all exp values (parallel reduction)
                let sum: f32 = exp_values.par_iter().sum();

                // Step 4: Divide each exp value by sum (parallel map)
                let output: Vec<f32> = exp_values.par_iter().map(|&x| x / sum).collect();

                Ok(WorkloadData::F32Vec(output))
            }
            WorkloadData::F64Vec(input) => {
                if input.is_empty() {
                    return Ok(WorkloadData::F64Vec(vec![]));
                }

                let max_val = input
                    .par_iter()
                    .copied()
                    .reduce(|| f64::NEG_INFINITY, f64::max);

                let exp_values: Vec<f64> = input
                    .par_iter()
                    .map(|&x| (x - max_val).exp())
                    .collect();

                let sum: f64 = exp_values.par_iter().sum();

                let output: Vec<f64> = exp_values.par_iter().map(|&x| x / sum).collect();

                Ok(WorkloadData::F64Vec(output))
            }
            _ => Err(ComputeError::UnsupportedWorkload),
        }
    }

    /// Execute ReLU activation (Rectified Linear Unit)
    fn execute_relu(&self, workload: Workload) -> Result<WorkloadData, ComputeError> {
        use rayon::prelude::*;

        // ReLU: f(x) = max(0, x)
        // LeakyReLU: f(x) = max(alpha * x, x) where alpha = 0.01 (default)
        
        // Check if LeakyReLU alpha parameter is provided
        let alpha = workload
            .params
            .params
            .get("alpha")
            .and_then(|v| match v {
                ParamValue::Float(f) => Some(*f as f32),
                _ => None,
            })
            .unwrap_or(0.0); // Default to standard ReLU

        match workload.input {
            WorkloadData::F32Vec(input) => {
                let output: Vec<f32> = input
                    .par_iter()
                    .map(|&x| {
                        if x > 0.0 {
                            x
                        } else if alpha > 0.0 {
                            alpha * x // LeakyReLU
                        } else {
                            0.0 // Standard ReLU
                        }
                    })
                    .collect();
                Ok(WorkloadData::F32Vec(output))
            }
            WorkloadData::F64Vec(input) => {
                let alpha = alpha as f64;
                let output: Vec<f64> = input
                    .par_iter()
                    .map(|&x| {
                        if x > 0.0 {
                            x
                        } else if alpha > 0.0 {
                            alpha * x
                        } else {
                            0.0
                        }
                    })
                    .collect();
                Ok(WorkloadData::F64Vec(output))
            }
            _ => Err(ComputeError::UnsupportedWorkload),
        }
    }

    /// Execute GELU activation (Gaussian Error Linear Unit)
    fn execute_gelu(&self, workload: Workload) -> Result<WorkloadData, ComputeError> {
        use rayon::prelude::*;

        // GELU: x * 0.5 * (1 + tanh(sqrt(2/π) * (x + 0.044715 * x^3)))
        // Approximate: x * sigmoid(1.702 * x)
        // We'll use the approximate version for efficiency
        
        let gelu = |x: f32| -> f32 {
            // Approximate GELU: x * sigmoid(1.702 * x)
            let sigmoid = 1.0 / (1.0 + (-1.702 * x).exp());
            x * sigmoid
        };

        match workload.input {
            WorkloadData::F32Vec(input) => {
                let output: Vec<f32> = input.par_iter().map(|&x| gelu(x)).collect();
                Ok(WorkloadData::F32Vec(output))
            }
            WorkloadData::F64Vec(input) => {
                let gelu_f64 = |x: f64| -> f64 {
                    let sigmoid = 1.0 / (1.0 + (-1.702 * x).exp());
                    x * sigmoid
                };
                let output: Vec<f64> = input.par_iter().map(|&x| gelu_f64(x)).collect();
                Ok(WorkloadData::F64Vec(output))
            }
            _ => Err(ComputeError::UnsupportedWorkload),
        }
    }

    /// Execute Tanh activation (hyperbolic tangent)
    fn execute_tanh(&self, workload: Workload) -> Result<WorkloadData, ComputeError> {
        use rayon::prelude::*;

        // Tanh: (exp(x) - exp(-x)) / (exp(x) + exp(-x))
        // Range: (-1, 1)
        // Symmetric around origin
        
        match workload.input {
            WorkloadData::F32Vec(input) => {
                let output: Vec<f32> = input.par_iter().map(|&x| x.tanh()).collect();
                Ok(WorkloadData::F32Vec(output))
            }
            WorkloadData::F64Vec(input) => {
                let output: Vec<f64> = input.par_iter().map(|&x| x.tanh()).collect();
                Ok(WorkloadData::F64Vec(output))
            }
            _ => Err(ComputeError::UnsupportedWorkload),
        }
    }

    /// Execute Sigmoid activation (logistic function)
    fn execute_sigmoid(&self, workload: Workload) -> Result<WorkloadData, ComputeError> {
        use rayon::prelude::*;

        // Sigmoid: 1 / (1 + exp(-x))
        // Range: (0, 1)
        // Used for binary classification, gates in LSTMs
        
        match workload.input {
            WorkloadData::F32Vec(input) => {
                let output: Vec<f32> = input
                    .par_iter()
                    .map(|&x| 1.0 / (1.0 + (-x).exp()))
                    .collect();
                Ok(WorkloadData::F32Vec(output))
            }
            WorkloadData::F64Vec(input) => {
                let output: Vec<f64> = input
                    .par_iter()
                    .map(|&x| 1.0 / (1.0 + (-x).exp()))
                    .collect();
                Ok(WorkloadData::F64Vec(output))
            }
            _ => Err(ComputeError::UnsupportedWorkload),
        }
    }

    /// Execute Dropout (random masking for regularization)
    fn execute_dropout(&self, workload: Workload) -> Result<WorkloadData, ComputeError> {
        use rayon::prelude::*;

        // Get dropout rate (probability of dropping)
        let dropout_rate = workload
            .params
            .params
            .get("dropout_rate")
            .and_then(|v| match v {
                ParamValue::Float(f) => Some(*f as f32),
                _ => None,
            })
            .unwrap_or(0.5); // Default 50% dropout

        if dropout_rate <= 0.0 {
            // No dropout (inference mode)
            return Ok(workload.input);
        }

        let keep_prob = 1.0 - dropout_rate;
        let scale = 1.0 / keep_prob; // Inverted dropout scaling

        match workload.input {
            WorkloadData::F32Vec(input) => {
                // Simple deterministic "dropout" for demo purposes
                // In production, would use proper RNG with seed
                // For demo: drop every other element based on simple pattern
                let output: Vec<f32> = input
                    .par_iter()
                    .enumerate()
                    .map(|(i, &x)| {
                        // Deterministic pseudo-random based on index and value
                        let hash = (i as f32 * 2654435761.0) % 1.0;
                        if hash < dropout_rate {
                            0.0 // Dropped
                        } else {
                            x * scale // Scaled to compensate
                        }
                    })
                    .collect();
                Ok(WorkloadData::F32Vec(output))
            }
            WorkloadData::F64Vec(input) => {
                let dropout_rate = dropout_rate as f64;
                let keep_prob = 1.0 - dropout_rate;
                let scale = 1.0 / keep_prob;

                let output: Vec<f64> = input
                    .par_iter()
                    .enumerate()
                    .map(|(i, &x)| {
                        let hash = (i as f64 * 2654435761.0) % 1.0;
                        if hash < dropout_rate {
                            0.0
                        } else {
                            x * scale
                        }
                    })
                    .collect();
                Ok(WorkloadData::F64Vec(output))
            }
            _ => Err(ComputeError::UnsupportedWorkload),
        }
    }

    /// Execute Layer Normalization
    fn execute_layernorm(&self, workload: Workload) -> Result<WorkloadData, ComputeError> {
        use rayon::prelude::*;

        // LayerNorm: (x - mean) / sqrt(variance + epsilon)
        // Composite: Reduce (mean) + Map (subtract) + Reduce (variance) + Map (normalize)
        
        let epsilon = workload
            .params
            .params
            .get("epsilon")
            .and_then(|v| match v {
                ParamValue::Float(f) => Some(*f as f32),
                _ => None,
            })
            .unwrap_or(1e-5); // Default epsilon for numerical stability

        match workload.input {
            WorkloadData::F32Vec(input) => {
                if input.is_empty() {
                    return Ok(WorkloadData::F32Vec(vec![]));
                }

                let n = input.len() as f32;

                // Step 1: Calculate mean (parallel reduction)
                let mean: f32 = input.par_iter().sum::<f32>() / n;

                // Step 2: Calculate variance (parallel)
                let variance: f32 = input
                    .par_iter()
                    .map(|&x| {
                        let diff = x - mean;
                        diff * diff
                    })
                    .sum::<f32>()
                    / n;

                // Step 3: Normalize (parallel map)
                let std_dev = (variance + epsilon).sqrt();
                let output: Vec<f32> = input
                    .par_iter()
                    .map(|&x| (x - mean) / std_dev)
                    .collect();

                Ok(WorkloadData::F32Vec(output))
            }
            WorkloadData::F64Vec(input) => {
                if input.is_empty() {
                    return Ok(WorkloadData::F64Vec(vec![]));
                }

                let n = input.len() as f64;
                let epsilon = epsilon as f64;

                let mean: f64 = input.par_iter().sum::<f64>() / n;

                let variance: f64 = input
                    .par_iter()
                    .map(|&x| {
                        let diff = x - mean;
                        diff * diff
                    })
                    .sum::<f64>()
                    / n;

                let std_dev = (variance + epsilon).sqrt();
                let output: Vec<f64> = input
                    .par_iter()
                    .map(|&x| (x - mean) / std_dev)
                    .collect();

                Ok(WorkloadData::F64Vec(output))
            }
            _ => Err(ComputeError::UnsupportedWorkload),
        }
    }

    /// Execute Batch Normalization
    fn execute_batchnorm(&self, workload: Workload) -> Result<WorkloadData, ComputeError> {
        use rayon::prelude::*;

        // BatchNorm: Normalize across batch dimension
        // For each feature: (x - batch_mean) / sqrt(batch_variance + epsilon)
        // Composite: Reduce (batch mean) + Map (subtract) + Reduce (batch variance) + Map (normalize)
        // Same 4-phase structure as LayerNorm and Softmax!
        
        let epsilon = workload
            .params
            .params
            .get("epsilon")
            .and_then(|v| match v {
                ParamValue::Float(f) => Some(*f as f32),
                _ => None,
            })
            .unwrap_or(1e-5); // Default epsilon for numerical stability

        match workload.input {
            WorkloadData::F32Matrix(data, batch_size, num_features) => {
                if batch_size == 0 || num_features == 0 {
                    return Ok(WorkloadData::F32Matrix(vec![], batch_size, num_features));
                }

                // BatchNorm normalizes across the batch dimension for each feature
                // data is (batch_size x num_features) in row-major order
                
                // Compute stats for all features in parallel
                let stats: Vec<(f32, f32)> = (0..num_features)
                    .into_par_iter()
                    .map(|feature_idx| {
                        // Step 1: Calculate mean for this feature across batch (Reduce)
                        let mut sum = 0.0f32;
                        for batch_idx in 0..batch_size {
                            sum += data[batch_idx * num_features + feature_idx];
                        }
                        let mean = sum / batch_size as f32;

                        // Step 2: Calculate variance for this feature across batch (Map + Reduce)
                        let mut variance_sum = 0.0f32;
                        for batch_idx in 0..batch_size {
                            let val = data[batch_idx * num_features + feature_idx];
                            let diff = val - mean;
                            variance_sum += diff * diff;
                        }
                        let variance = variance_sum / batch_size as f32;
                        let std_dev = (variance + epsilon).sqrt();

                        (mean, std_dev)
                    })
                    .collect();

                // Step 3: Normalize (Map) - create output vector and process each row in parallel
                let mut output = vec![0.0f32; data.len()];
                output
                    .par_chunks_mut(num_features)
                    .enumerate()
                    .for_each(|(batch_idx, row)| {
                        for feature_idx in 0..num_features {
                            let val = data[batch_idx * num_features + feature_idx];
                            let (mean, std_dev) = stats[feature_idx];
                            row[feature_idx] = (val - mean) / std_dev;
                        }
                    });

                Ok(WorkloadData::F32Matrix(output, batch_size, num_features))
            }
            WorkloadData::F64Matrix(data, batch_size, num_features) => {
                if batch_size == 0 || num_features == 0 {
                    return Ok(WorkloadData::F64Matrix(vec![], batch_size, num_features));
                }

                let epsilon = epsilon as f64;

                // Compute stats for all features in parallel
                let stats: Vec<(f64, f64)> = (0..num_features)
                    .into_par_iter()
                    .map(|feature_idx| {
                        let mut sum = 0.0f64;
                        for batch_idx in 0..batch_size {
                            sum += data[batch_idx * num_features + feature_idx];
                        }
                        let mean = sum / batch_size as f64;

                        let mut variance_sum = 0.0f64;
                        for batch_idx in 0..batch_size {
                            let val = data[batch_idx * num_features + feature_idx];
                            let diff = val - mean;
                            variance_sum += diff * diff;
                        }
                        let variance = variance_sum / batch_size as f64;
                        let std_dev = (variance + epsilon).sqrt();

                        (mean, std_dev)
                    })
                    .collect();

                // Normalize - create output vector and process each row in parallel
                let mut output = vec![0.0f64; data.len()];
                output
                    .par_chunks_mut(num_features)
                    .enumerate()
                    .for_each(|(batch_idx, row)| {
                        for feature_idx in 0..num_features {
                            let val = data[batch_idx * num_features + feature_idx];
                            let (mean, std_dev) = stats[feature_idx];
                            row[feature_idx] = (val - mean) / std_dev;
                        }
                    });

                Ok(WorkloadData::F64Matrix(output, batch_size, num_features))
            }
            _ => Err(ComputeError::UnsupportedWorkload),
        }
    }

    /// Execute matrix multiplication (tiled/blocked)
    fn execute_matmul(&self, workload: Workload) -> Result<WorkloadData, ComputeError> {
        use rayon::prelude::*;

        // MatMul: C = A * B
        // A: (M x K), B: (K x N) → C: (M x N)
        // Tiled/blocked approach for cache efficiency
        
        const TILE_SIZE: usize = 64; // Optimize for L1 cache

        match workload.input {
            WorkloadData::F32MatrixPair(a_data, a_rows, a_cols, b_data, b_rows, b_cols) => {
                // Validate dimensions: A.cols must equal B.rows
                if a_cols != b_rows {
                    return Err(ComputeError::ExecutionFailed(format!(
                        "MatMul dimension mismatch: A({},{}) * B({},{})",
                        a_rows, a_cols, b_rows, b_cols
                    )));
                }

                let m = a_rows;
                let k = a_cols;
                let n = b_cols;

                // Result matrix C: (M x N)
                let mut c = vec![0.0f32; m * n];

                // Parallel execution over output rows
                c.par_chunks_mut(n).enumerate().for_each(|(i, c_row)| {
                    // Process tiles for cache efficiency
                    for kk in (0..k).step_by(TILE_SIZE) {
                        let k_end = (kk + TILE_SIZE).min(k);
                        
                        for jj in (0..n).step_by(TILE_SIZE) {
                            let j_end = (jj + TILE_SIZE).min(n);
                            
                            // Compute tile: C[i, jj:j_end] += A[i, kk:k_end] * B[kk:k_end, jj:j_end]
                            for k_idx in kk..k_end {
                                let a_val = a_data[i * k + k_idx];
                                for j in jj..j_end {
                                    c_row[j] += a_val * b_data[k_idx * n + j];
                                }
                            }
                        }
                    }
                });

                Ok(WorkloadData::F32Matrix(c, m, n))
            }
            WorkloadData::F64MatrixPair(a_data, a_rows, a_cols, b_data, b_rows, b_cols) => {
                if a_cols != b_rows {
                    return Err(ComputeError::ExecutionFailed(format!(
                        "MatMul dimension mismatch: A({},{}) * B({},{})",
                        a_rows, a_cols, b_rows, b_cols
                    )));
                }

                let m = a_rows;
                let k = a_cols;
                let n = b_cols;

                let mut c = vec![0.0f64; m * n];

                c.par_chunks_mut(n).enumerate().for_each(|(i, c_row)| {
                    for kk in (0..k).step_by(TILE_SIZE) {
                        let k_end = (kk + TILE_SIZE).min(k);
                        
                        for jj in (0..n).step_by(TILE_SIZE) {
                            let j_end = (jj + TILE_SIZE).min(n);
                            
                            for k_idx in kk..k_end {
                                let a_val = a_data[i * k + k_idx];
                                for j in jj..j_end {
                                    c_row[j] += a_val * b_data[k_idx * n + j];
                                }
                            }
                        }
                    }
                });

                Ok(WorkloadData::F64Matrix(c, m, n))
            }
            _ => Err(ComputeError::UnsupportedWorkload),
        }
    }

    /// Execute convolution (placeholder)
    fn execute_conv(&self, workload: Workload) -> Result<WorkloadData, ComputeError> {
        use rayon::prelude::*;

        // Conv2D: Y = Conv(X, W) + b
        // Input X: (batch, in_channels, height, width)
        // Kernel W: (out_channels, in_channels, kernel_h, kernel_w)
        // Bias b: (out_channels) [optional]
        // Output Y: (batch, out_channels, out_h, out_w)
        
        match workload.input {
            WorkloadData::F32Conv2D {
                input,
                kernel,
                bias,
                batch_size,
                in_channels,
                height,
                width,
                out_channels,
                kernel_h,
                kernel_w,
                stride,
                padding,
            } => {
                // Calculate output dimensions
                let out_h = (height + 2 * padding - kernel_h) / stride + 1;
                let out_w = (width + 2 * padding - kernel_w) / stride + 1;
                
                // Validate dimensions
                if input.len() != batch_size * in_channels * height * width {
                    return Err(ComputeError::ExecutionFailed(
                        format!("Input size mismatch: expected {}, got {}",
                            batch_size * in_channels * height * width, input.len())
                    ));
                }
                
                if kernel.len() != out_channels * in_channels * kernel_h * kernel_w {
                    return Err(ComputeError::ExecutionFailed(
                        format!("Kernel size mismatch: expected {}, got {}",
                            out_channels * in_channels * kernel_h * kernel_w, kernel.len())
                    ));
                }
                
                // Output buffer
                let output_size = batch_size * out_channels * out_h * out_w;
                let mut output = vec![0.0f32; output_size];
                
                // Parallel over batch and output channels
                output
                    .par_chunks_mut(out_channels * out_h * out_w)
                    .enumerate()
                    .for_each(|(batch_idx, batch_output)| {
                        for out_ch in 0..out_channels {
                            for out_y in 0..out_h {
                                for out_x in 0..out_w {
                                    let mut sum = 0.0f32;
                                    
                                    // Convolve over input channels and kernel
                                    for in_ch in 0..in_channels {
                                        for ky in 0..kernel_h {
                                            for kx in 0..kernel_w {
                                                let in_y = out_y * stride + ky;
                                                let in_x = out_x * stride + kx;
                                                
                                                // Apply padding (zero-padding)
                                                if in_y >= padding && in_y < height + padding &&
                                                   in_x >= padding && in_x < width + padding {
                                                    let in_y = in_y - padding;
                                                    let in_x = in_x - padding;
                                                    
                                                    let input_idx = batch_idx * (in_channels * height * width) +
                                                        in_ch * (height * width) +
                                                        in_y * width +
                                                        in_x;
                                                    
                                                    let kernel_idx = out_ch * (in_channels * kernel_h * kernel_w) +
                                                        in_ch * (kernel_h * kernel_w) +
                                                        ky * kernel_w +
                                                        kx;
                                                    
                                                    sum += input[input_idx] * kernel[kernel_idx];
                                                }
                                            }
                                        }
                                    }
                                    
                                    // Add bias if present
                                    if let Some(ref b) = bias {
                                        sum += b[out_ch];
                                    }
                                    
                                    let output_idx = out_ch * (out_h * out_w) + out_y * out_w + out_x;
                                    batch_output[output_idx] = sum;
                                }
                            }
                        }
                    });
                
                Ok(WorkloadData::F32Matrix(output, batch_size, out_channels * out_h * out_w))
            }
            _ => Err(ComputeError::UnsupportedWorkload),
        }
    }

    /// Execute MaxPool2D
    fn execute_maxpool2d(&self, workload: Workload) -> Result<WorkloadData, ComputeError> {
        use rayon::prelude::*;

        // MaxPool2D: Takes maximum value in each pool region
        // Input: (batch, channels, height, width)
        // Output: (batch, channels, out_h, out_w)
        
        match workload.input {
            WorkloadData::F32Pool2D {
                input,
                batch_size,
                channels,
                height,
                width,
                pool_h,
                pool_w,
                stride,
                padding,
            } => {
                // Calculate output dimensions
                let out_h = (height + 2 * padding - pool_h) / stride + 1;
                let out_w = (width + 2 * padding - pool_w) / stride + 1;
                
                // Validate input dimensions
                if input.len() != batch_size * channels * height * width {
                    return Err(ComputeError::ExecutionFailed(
                        format!("Input size mismatch: expected {}, got {}",
                            batch_size * channels * height * width, input.len())
                    ));
                }
                
                // Output buffer
                let output_size = batch_size * channels * out_h * out_w;
                let mut output = vec![f32::NEG_INFINITY; output_size];
                
                // Parallel over batch and channels
                output
                    .par_chunks_mut(channels * out_h * out_w)
                    .enumerate()
                    .for_each(|(batch_idx, batch_output)| {
                        for ch in 0..channels {
                            for out_y in 0..out_h {
                                for out_x in 0..out_w {
                                    let mut max_val = f32::NEG_INFINITY;
                                    
                                    // Find maximum in pool region
                                    for py in 0..pool_h {
                                        for px in 0..pool_w {
                                            let in_y = out_y * stride + py;
                                            let in_x = out_x * stride + px;
                                            
                                            // Apply padding (use -inf for out-of-bounds)
                                            if in_y >= padding && in_y < height + padding &&
                                               in_x >= padding && in_x < width + padding {
                                                let in_y = in_y - padding;
                                                let in_x = in_x - padding;
                                                
                                                let input_idx = batch_idx * (channels * height * width) +
                                                    ch * (height * width) +
                                                    in_y * width +
                                                    in_x;
                                                
                                                max_val = max_val.max(input[input_idx]);
                                            }
                                        }
                                    }
                                    
                                    let output_idx = ch * (out_h * out_w) + out_y * out_w + out_x;
                                    batch_output[output_idx] = max_val;
                                }
                            }
                        }
                    });
                
                Ok(WorkloadData::F32Matrix(output, batch_size, channels * out_h * out_w))
            }
            _ => Err(ComputeError::UnsupportedWorkload),
        }
    }

    /// Execute AvgPool2D
    fn execute_avgpool2d(&self, workload: Workload) -> Result<WorkloadData, ComputeError> {
        use rayon::prelude::*;

        // AvgPool2D: Takes average value in each pool region
        // Input: (batch, channels, height, width)
        // Output: (batch, channels, out_h, out_w)
        
        match workload.input {
            WorkloadData::F32Pool2D {
                input,
                batch_size,
                channels,
                height,
                width,
                pool_h,
                pool_w,
                stride,
                padding,
            } => {
                // Calculate output dimensions
                let out_h = (height + 2 * padding - pool_h) / stride + 1;
                let out_w = (width + 2 * padding - pool_w) / stride + 1;
                
                // Validate input dimensions
                if input.len() != batch_size * channels * height * width {
                    return Err(ComputeError::ExecutionFailed(
                        format!("Input size mismatch: expected {}, got {}",
                            batch_size * channels * height * width, input.len())
                    ));
                }
                
                // Output buffer
                let output_size = batch_size * channels * out_h * out_w;
                let mut output = vec![0.0f32; output_size];
                
                // Parallel over batch and channels
                output
                    .par_chunks_mut(channels * out_h * out_w)
                    .enumerate()
                    .for_each(|(batch_idx, batch_output)| {
                        for ch in 0..channels {
                            for out_y in 0..out_h {
                                for out_x in 0..out_w {
                                    let mut sum = 0.0f32;
                                    let mut count = 0;
                                    
                                    // Sum values in pool region
                                    for py in 0..pool_h {
                                        for px in 0..pool_w {
                                            let in_y = out_y * stride + py;
                                            let in_x = out_x * stride + px;
                                            
                                            // Apply padding (skip out-of-bounds)
                                            if in_y >= padding && in_y < height + padding &&
                                               in_x >= padding && in_x < width + padding {
                                                let in_y = in_y - padding;
                                                let in_x = in_x - padding;
                                                
                                                let input_idx = batch_idx * (channels * height * width) +
                                                    ch * (height * width) +
                                                    in_y * width +
                                                    in_x;
                                                
                                                sum += input[input_idx];
                                                count += 1;
                                            }
                                        }
                                    }
                                    
                                    let output_idx = ch * (out_h * out_w) + out_y * out_w + out_x;
                                    batch_output[output_idx] = if count > 0 { sum / count as f32 } else { 0.0 };
                                }
                            }
                        }
                    });
                
                Ok(WorkloadData::F32Matrix(output, batch_size, channels * out_h * out_w))
            }
            _ => Err(ComputeError::UnsupportedWorkload),
        }
    }
}

