// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tensor Operations - Compute-Intensive Pattern
//!
//! High-compute operations with tiling:
//! - MatMul: Blocked matrix multiplication
//! - Conv2D: Direct convolution with nested loops
//! - Pooling: Sliding window reductions
//!
//! All implementations use pure Rust with no external dependencies.

use crate::types::*;

/// Tile size for cache-efficient matrix multiplication
const TILE_SIZE: usize = 32;

/// Execute matrix multiplication: C = A * B
///
/// Uses tiled/blocked algorithm for cache efficiency.
/// Time complexity: O(n³), but cache-friendly access patterns.
#[inline]
pub(super) fn execute_matmul(workload: Workload) -> Result<WorkloadData, ComputeError> {
    match workload.input {
        WorkloadData::F32MatrixPair(a_data, a_rows, a_cols, b_data, b_rows, b_cols) => {
            // Validate dimensions: A is (m x k), B is (k x n), result is (m x n)
            if a_cols != b_rows {
                return Err(ComputeError::ExecutionFailed(format!(
                    "Matrix dimension mismatch: A cols ({a_cols}) != B rows ({b_rows})"
                )));
            }

            let m = a_rows;
            let k = a_cols;
            let n = b_cols;

            // Initialize result matrix C (m x n)
            let mut c_data = vec![0.0f32; m * n];

            // Tiled matrix multiplication for cache efficiency
            for i_tile in (0..m).step_by(TILE_SIZE) {
                for j_tile in (0..n).step_by(TILE_SIZE) {
                    for k_tile in (0..k).step_by(TILE_SIZE) {
                        // Process tile
                        let i_end = (i_tile + TILE_SIZE).min(m);
                        let j_end = (j_tile + TILE_SIZE).min(n);
                        let k_end = (k_tile + TILE_SIZE).min(k);

                        for i in i_tile..i_end {
                            for kk in k_tile..k_end {
                                let a_val = a_data[i * k + kk];
                                for j in j_tile..j_end {
                                    c_data[i * n + j] += a_val * b_data[kk * n + j];
                                }
                            }
                        }
                    }
                }
            }

            Ok(WorkloadData::F32Matrix(c_data, m, n))
        }
        _ => Err(ComputeError::ExecutionFailed(
            "MatMul requires F32MatrixPair input".to_string(),
        )),
    }
}

/// Execute 2D convolution
///
/// Direct convolution with 7 nested loops (batch, out_channels, in_channels, h, w, kh, kw).
/// Supports padding, stride, and optional bias.
#[inline]
pub(super) fn execute_conv(workload: Workload) -> Result<WorkloadData, ComputeError> {
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

            // Initialize output tensor
            let out_size = batch_size * out_channels * out_h * out_w;
            let mut output = vec![0.0f32; out_size];

            // Direct convolution
            for b in 0..batch_size {
                for oc in 0..out_channels {
                    for oh in 0..out_h {
                        for ow in 0..out_w {
                            let mut sum = 0.0f32;

                            for ic in 0..in_channels {
                                for kh in 0..kernel_h {
                                    for kw in 0..kernel_w {
                                        let ih = (oh * stride + kh) as isize - padding as isize;
                                        let iw = (ow * stride + kw) as isize - padding as isize;

                                        // Check bounds (handles padding)
                                        if ih >= 0
                                            && ih < height as isize
                                            && iw >= 0
                                            && iw < width as isize
                                        {
                                            let ih = ih as usize;
                                            let iw = iw as usize;

                                            // Input index: [b, ic, ih, iw] in NCHW format
                                            let input_idx = b * (in_channels * height * width)
                                                + ic * (height * width)
                                                + ih * width
                                                + iw;

                                            // Kernel index: [oc, ic, kh, kw]
                                            let kernel_idx = oc
                                                * (in_channels * kernel_h * kernel_w)
                                                + ic * (kernel_h * kernel_w)
                                                + kh * kernel_w
                                                + kw;

                                            sum += input[input_idx] * kernel[kernel_idx];
                                        }
                                    }
                                }
                            }

                            // Add bias if present
                            let bias_val = bias
                                .as_ref()
                                .map(|b| b.get(oc).copied().unwrap_or(0.0))
                                .unwrap_or(0.0);

                            // Output index: [b, oc, oh, ow]
                            let out_idx = b * (out_channels * out_h * out_w)
                                + oc * (out_h * out_w)
                                + oh * out_w
                                + ow;
                            output[out_idx] = sum + bias_val;
                        }
                    }
                }
            }

            // Return as matrix (flattened)
            Ok(WorkloadData::F32Matrix(
                output,
                batch_size * out_channels,
                out_h * out_w,
            ))
        }
        _ => Err(ComputeError::ExecutionFailed(
            "Conv2D requires F32Conv2D input".to_string(),
        )),
    }
}

/// Execute 2D max pooling
///
/// Sliding window operation that takes the maximum value in each window.
#[inline]
pub(super) fn execute_maxpool2d(workload: Workload) -> Result<WorkloadData, ComputeError> {
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

            // Initialize output tensor
            let out_size = batch_size * channels * out_h * out_w;
            let mut output = vec![f32::NEG_INFINITY; out_size];

            // Max pooling
            for b in 0..batch_size {
                for c in 0..channels {
                    for oh in 0..out_h {
                        for ow in 0..out_w {
                            let mut max_val = f32::NEG_INFINITY;

                            for ph in 0..pool_h {
                                for pw in 0..pool_w {
                                    let ih = (oh * stride + ph) as isize - padding as isize;
                                    let iw = (ow * stride + pw) as isize - padding as isize;

                                    if ih >= 0
                                        && ih < height as isize
                                        && iw >= 0
                                        && iw < width as isize
                                    {
                                        let ih = ih as usize;
                                        let iw = iw as usize;

                                        let input_idx = b * (channels * height * width)
                                            + c * (height * width)
                                            + ih * width
                                            + iw;

                                        max_val = max_val.max(input[input_idx]);
                                    }
                                }
                            }

                            let out_idx = b * (channels * out_h * out_w)
                                + c * (out_h * out_w)
                                + oh * out_w
                                + ow;
                            output[out_idx] = max_val;
                        }
                    }
                }
            }

            Ok(WorkloadData::F32Matrix(
                output,
                batch_size * channels,
                out_h * out_w,
            ))
        }
        _ => Err(ComputeError::ExecutionFailed(
            "MaxPool2D requires F32Pool2D input".to_string(),
        )),
    }
}

/// Execute 2D average pooling
///
/// Sliding window operation that takes the average value in each window.
#[inline]
pub(super) fn execute_avgpool2d(workload: Workload) -> Result<WorkloadData, ComputeError> {
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

            // Initialize output tensor
            let out_size = batch_size * channels * out_h * out_w;
            let mut output = vec![0.0f32; out_size];

            // Average pooling
            for b in 0..batch_size {
                for c in 0..channels {
                    for oh in 0..out_h {
                        for ow in 0..out_w {
                            let mut sum = 0.0f32;
                            let mut count = 0usize;

                            for ph in 0..pool_h {
                                for pw in 0..pool_w {
                                    let ih = (oh * stride + ph) as isize - padding as isize;
                                    let iw = (ow * stride + pw) as isize - padding as isize;

                                    if ih >= 0
                                        && ih < height as isize
                                        && iw >= 0
                                        && iw < width as isize
                                    {
                                        let ih = ih as usize;
                                        let iw = iw as usize;

                                        let input_idx = b * (channels * height * width)
                                            + c * (height * width)
                                            + ih * width
                                            + iw;

                                        sum += input[input_idx];
                                        count += 1;
                                    }
                                }
                            }

                            let out_idx = b * (channels * out_h * out_w)
                                + c * (out_h * out_w)
                                + oh * out_w
                                + ow;

                            // Avoid division by zero
                            output[out_idx] = if count > 0 { sum / count as f32 } else { 0.0 };
                        }
                    }
                }
            }

            Ok(WorkloadData::F32Matrix(
                output,
                batch_size * channels,
                out_h * out_w,
            ))
        }
        _ => Err(ComputeError::ExecutionFailed(
            "AvgPool2D requires F32Pool2D input".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matmul_2x2() {
        // A = [[1, 2], [3, 4]]
        // B = [[5, 6], [7, 8]]
        // C = [[19, 22], [43, 50]]
        let a = vec![1.0, 2.0, 3.0, 4.0];
        let b = vec![5.0, 6.0, 7.0, 8.0];

        let workload = Workload {
            operation: OperationType::MatMul,
            data_type: DataType::F32,
            num_operations: 8,
            required_memory: 64,
            input: WorkloadData::F32MatrixPair(a, 2, 2, b, 2, 2),
            params: WorkloadParams::default(),
        };

        let result = execute_matmul(workload).unwrap();
        if let WorkloadData::F32Matrix(c, rows, cols) = result {
            assert_eq!(rows, 2);
            assert_eq!(cols, 2);
            assert!((c[0] - 19.0).abs() < 1e-6);
            assert!((c[1] - 22.0).abs() < 1e-6);
            assert!((c[2] - 43.0).abs() < 1e-6);
            assert!((c[3] - 50.0).abs() < 1e-6);
        } else {
            panic!("Expected F32Matrix result");
        }
    }

    #[test]
    fn test_matmul_dimension_mismatch() {
        let a = vec![1.0, 2.0, 3.0, 4.0];
        let b = vec![5.0, 6.0, 7.0];

        let workload = Workload {
            operation: OperationType::MatMul,
            data_type: DataType::F32,
            num_operations: 0,
            required_memory: 0,
            input: WorkloadData::F32MatrixPair(a, 2, 2, b, 3, 1),
            params: WorkloadParams::default(),
        };

        let result = execute_matmul(workload);
        assert!(result.is_err());
    }

    #[test]
    fn test_maxpool2d_simple() {
        // 1x1x4x4 input, 2x2 pool, stride 2
        // Input: [[1,2,3,4], [5,6,7,8], [9,10,11,12], [13,14,15,16]]
        // Expected: [[6,8], [14,16]]
        let input: Vec<f32> = (1..=16).map(|x| x as f32).collect();

        let workload = Workload {
            operation: OperationType::MaxPool2D,
            data_type: DataType::F32,
            num_operations: 16,
            required_memory: 64,
            input: WorkloadData::F32Pool2D {
                input,
                batch_size: 1,
                channels: 1,
                height: 4,
                width: 4,
                pool_h: 2,
                pool_w: 2,
                stride: 2,
                padding: 0,
            },
            params: WorkloadParams::default(),
        };

        let result = execute_maxpool2d(workload).unwrap();
        if let WorkloadData::F32Matrix(out, rows, cols) = result {
            assert_eq!(rows, 1); // batch * channels
            assert_eq!(cols, 4); // out_h * out_w (2x2)
            assert!((out[0] - 6.0).abs() < 1e-6);
            assert!((out[1] - 8.0).abs() < 1e-6);
            assert!((out[2] - 14.0).abs() < 1e-6);
            assert!((out[3] - 16.0).abs() < 1e-6);
        } else {
            panic!("Expected F32Matrix result");
        }
    }

    #[test]
    fn test_avgpool2d_simple() {
        // 1x1x4x4 input, 2x2 pool, stride 2
        // Input: [[1,2,3,4], [5,6,7,8], [9,10,11,12], [13,14,15,16]]
        // Expected: [[(1+2+5+6)/4, (3+4+7+8)/4], [(9+10+13+14)/4, (11+12+15+16)/4]]
        //         = [[3.5, 5.5], [11.5, 13.5]]
        let input: Vec<f32> = (1..=16).map(|x| x as f32).collect();

        let workload = Workload {
            operation: OperationType::AvgPool2D,
            data_type: DataType::F32,
            num_operations: 16,
            required_memory: 64,
            input: WorkloadData::F32Pool2D {
                input,
                batch_size: 1,
                channels: 1,
                height: 4,
                width: 4,
                pool_h: 2,
                pool_w: 2,
                stride: 2,
                padding: 0,
            },
            params: WorkloadParams::default(),
        };

        let result = execute_avgpool2d(workload).unwrap();
        if let WorkloadData::F32Matrix(out, rows, cols) = result {
            assert_eq!(rows, 1);
            assert_eq!(cols, 4);
            assert!((out[0] - 3.5).abs() < 1e-6);
            assert!((out[1] - 5.5).abs() < 1e-6);
            assert!((out[2] - 11.5).abs() < 1e-6);
            assert!((out[3] - 13.5).abs() < 1e-6);
        } else {
            panic!("Expected F32Matrix result");
        }
    }
}
