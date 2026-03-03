// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shape and layout operations: Reshape, Slice, Pad, Transpose, Squeeze, Unsqueeze, Expand

use crate::error::{BarracudaError, Result, ResultExt};

/// Reshape Operation
///
/// Changes tensor dimensions without copying data (zero-copy when possible).
///
/// ## Deep Debt Architecture
///
/// - **Self-Knowledge**: Validates shape compatibility
/// - **Error Handling**: Rich context on dimension mismatches
/// - **Zero Unsafe**: Pure Rust implementation
/// - **Capability-Based**: Works on any GPU
///
/// ## Algorithm
///
/// 1. Validate total element count matches
/// 2. Compute stride changes
/// 3. Update metadata (zero-copy)
/// 4. Return reshaped view
///
/// ## Use Cases
///
/// - Model input format conversion (e.g., NCHW ↔ NHWC)
/// - Flatten for FC layers
/// - Batch processing
/// - Neuromorphic model preprocessing
///
/// ## Example
///
/// ```rust,ignore
/// // Reshape [2, 3, 4] → [2, 12]
/// let reshaped = Reshape::execute(&input, &[2, 3, 4], &[2, 12])?;
/// ```
pub struct Reshape;

impl Reshape {
    /// Execute reshape operation
    ///
    /// # Arguments
    ///
    /// * `data` - Input tensor data (flat buffer)
    /// * `old_shape` - Original dimensions
    /// * `new_shape` - Target dimensions
    ///
    /// # Returns
    ///
    /// Reshaped tensor (zero-copy view when possible)
    ///
    /// # Errors
    ///
    /// Returns `BarracudaError::InvalidParameters` if:
    /// - Element counts don't match
    /// - Shapes contain zeros or negatives
    pub fn execute(data: &[f32], old_shape: &[usize], new_shape: &[usize]) -> Result<Vec<f32>> {
        // Validate shapes
        Self::validate_shapes(old_shape, new_shape).context("Reshape validation failed")?;

        // Compute element counts
        let old_count: usize = old_shape.iter().product();
        let new_count: usize = new_shape.iter().product();

        // Verify data length matches
        if data.len() != old_count {
            return Err(BarracudaError::invalid_params(
                "Reshape",
                format!(
                    "Data length {} doesn't match old_shape {} elements",
                    data.len(),
                    old_count
                ),
            ));
        }

        // Verify shape compatibility
        if old_count != new_count {
            return Err(
                BarracudaError::shape_mismatch(old_shape.to_vec(), new_shape.to_vec())
                    .with_context("Element counts must match for reshape"),
            );
        }

        // Zero-copy: just return the same data (shape metadata handled externally)
        // In a full implementation, this would return a tensor handle with new shape
        Ok(data.to_vec())
    }

    /// Validate shape dimensions
    fn validate_shapes(old_shape: &[usize], new_shape: &[usize]) -> Result<()> {
        // Check for empty shapes
        if old_shape.is_empty() {
            return Err(BarracudaError::invalid_params(
                "Reshape",
                "Old shape cannot be empty",
            ));
        }
        if new_shape.is_empty() {
            return Err(BarracudaError::invalid_params(
                "Reshape",
                "New shape cannot be empty",
            ));
        }

        // Check for zero dimensions
        if old_shape.contains(&0) {
            return Err(BarracudaError::invalid_params(
                "Reshape",
                format!("Old shape contains zero: {old_shape:?}"),
            ));
        }
        if new_shape.contains(&0) {
            return Err(BarracudaError::invalid_params(
                "Reshape",
                format!("New shape contains zero: {new_shape:?}"),
            ));
        }

        Ok(())
    }

    /// Compute flattened index from multi-dimensional indices
    pub fn compute_flat_index(indices: &[usize], shape: &[usize]) -> usize {
        let mut flat_idx = 0;
        let mut stride = 1;

        for i in (0..shape.len()).rev() {
            flat_idx += indices[i] * stride;
            stride *= shape[i];
        }

        flat_idx
    }
}

/// Slice Operation
///
/// Extracts a subtensor with support for strided slicing.
///
/// ## Algorithm
///
/// 1. Parse slice specification (start, end, stride)
/// 2. Validate bounds
/// 3. Extract elements according to strides
/// 4. Return sliced tensor
///
/// ## Use Cases
///
/// - Feature extraction
/// - Windowing for convolutions
/// - Data augmentation
/// - Neuromorphic preprocessing
pub struct Slice;

impl Slice {
    /// Execute slice operation
    ///
    /// # Arguments
    ///
    /// * `data` - Input tensor data
    /// * `shape` - Tensor dimensions
    /// * `ranges` - Slice ranges per dimension (start, end, step)
    ///
    /// # Returns
    ///
    /// Sliced tensor
    ///
    /// # Errors
    ///
    /// Returns error if ranges are out of bounds or invalid
    pub fn execute(
        data: &[f32],
        shape: &[usize],
        ranges: &[(usize, usize, usize)], // (start, end, step) per dimension
    ) -> Result<Vec<f32>> {
        // Validate inputs
        Self::validate_inputs(data, shape, ranges)?;

        // Compute output shape
        let output_shape: Vec<usize> = ranges
            .iter()
            .map(|(start, end, step)| (end - start).div_ceil(*step))
            .collect();

        // Compute total output size
        let output_size: usize = output_shape.iter().product();
        let mut output = Vec::with_capacity(output_size);

        // Extract elements
        Self::extract_elements(data, shape, ranges, &output_shape, &mut output)?;

        Ok(output)
    }

    fn validate_inputs(
        data: &[f32],
        shape: &[usize],
        ranges: &[(usize, usize, usize)],
    ) -> Result<()> {
        // Validate dimensions match
        if shape.len() != ranges.len() {
            return Err(BarracudaError::invalid_params(
                "Slice",
                format!(
                    "Shape has {} dimensions but got {} range specs",
                    shape.len(),
                    ranges.len()
                ),
            ));
        }

        // Validate each range
        for (dim, (&(start, end, step), &size)) in ranges.iter().zip(shape.iter()).enumerate() {
            if step == 0 {
                return Err(BarracudaError::invalid_params(
                    "Slice",
                    format!("Step cannot be zero in dimension {dim}"),
                ));
            }
            if start >= end {
                return Err(BarracudaError::invalid_params(
                    "Slice",
                    format!("Start {start} >= end {end} in dimension {dim}"),
                ));
            }
            if end > size {
                return Err(BarracudaError::invalid_params(
                    "Slice",
                    format!("End {end} exceeds size {size} in dimension {dim}"),
                ));
            }
        }

        // Validate data length
        let expected_size: usize = shape.iter().product();
        if data.len() != expected_size {
            return Err(BarracudaError::invalid_params(
                "Slice",
                format!(
                    "Data length {} doesn't match shape {} elements",
                    data.len(),
                    expected_size
                ),
            ));
        }

        Ok(())
    }

    fn extract_elements(
        data: &[f32],
        shape: &[usize],
        ranges: &[(usize, usize, usize)],
        _output_shape: &[usize],
        output: &mut Vec<f32>,
    ) -> Result<()> {
        // For simplicity, implement 1D-3D cases explicitly
        match shape.len() {
            1 => Self::extract_1d(data, ranges, output),
            2 => Self::extract_2d(data, shape, ranges, output),
            3 => Self::extract_3d(data, shape, ranges, output),
            _ => Err(BarracudaError::UnsupportedOperation {
                operation: "Slice".to_string(),
                reason: format!("Dimensions > 3 not yet implemented (got {})", shape.len()),
            }),
        }
    }

    fn extract_1d(
        data: &[f32],
        ranges: &[(usize, usize, usize)],
        output: &mut Vec<f32>,
    ) -> Result<()> {
        let (start, end, step) = ranges[0];
        for i in (start..end).step_by(step) {
            output.push(data[i]);
        }
        Ok(())
    }

    fn extract_2d(
        data: &[f32],
        shape: &[usize],
        ranges: &[(usize, usize, usize)],
        output: &mut Vec<f32>,
    ) -> Result<()> {
        let (start0, end0, step0) = ranges[0];
        let (start1, end1, step1) = ranges[1];
        let dim1 = shape[1];

        for i in (start0..end0).step_by(step0) {
            for j in (start1..end1).step_by(step1) {
                let idx = i * dim1 + j;
                output.push(data[idx]);
            }
        }
        Ok(())
    }

    fn extract_3d(
        data: &[f32],
        shape: &[usize],
        ranges: &[(usize, usize, usize)],
        output: &mut Vec<f32>,
    ) -> Result<()> {
        let (start0, end0, step0) = ranges[0];
        let (start1, end1, step1) = ranges[1];
        let (start2, end2, step2) = ranges[2];
        let dim1 = shape[1];
        let dim2 = shape[2];

        for i in (start0..end0).step_by(step0) {
            for j in (start1..end1).step_by(step1) {
                for k in (start2..end2).step_by(step2) {
                    let idx = i * dim1 * dim2 + j * dim2 + k;
                    output.push(data[idx]);
                }
            }
        }
        Ok(())
    }
}

/// Pad Operation
///
/// Adds padding to tensors with various modes (constant, reflect, replicate).
///
/// ## Padding Modes
///
/// - **Constant**: Fill with a constant value (default 0.0)
/// - **Reflect**: Mirror reflection (without edge duplication)
/// - **Replicate**: Edge values repeated
///
/// ## Use Cases
///
/// - Convolution boundary handling
/// - Maintaining spatial dimensions
/// - Data augmentation
/// - Neuromorphic preprocessing
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PadMode {
    Constant(f32),
    Reflect,
    Replicate,
}

pub struct Pad;

impl Pad {
    /// Execute pad operation
    ///
    /// # Arguments
    ///
    /// * `data` - Input tensor
    /// * `shape` - Input dimensions
    /// * `padding` - Padding per dimension [(before, after), ...]
    /// * `mode` - Padding mode
    ///
    /// # Returns
    ///
    /// Padded tensor
    pub fn execute(
        data: &[f32],
        shape: &[usize],
        padding: &[(usize, usize)], // (before, after) for each dim
        mode: PadMode,
    ) -> Result<Vec<f32>> {
        // Validate
        Self::validate_inputs(data, shape, padding)?;

        // Compute output shape
        let output_shape: Vec<usize> = shape
            .iter()
            .zip(padding.iter())
            .map(|(&dim, &(before, after))| dim + before + after)
            .collect();

        let output_size: usize = output_shape.iter().product();
        let mut output = vec![0.0; output_size];

        // Apply padding
        match shape.len() {
            1 => Self::pad_1d(data, shape, padding, mode, &mut output),
            2 => Self::pad_2d(data, shape, padding, mode, &mut output),
            3 => Self::pad_3d(data, shape, padding, mode, &mut output),
            _ => {
                return Err(BarracudaError::UnsupportedOperation {
                    operation: "Pad".to_string(),
                    reason: format!("Dimensions > 3 not implemented (got {})", shape.len()),
                })
            }
        }

        Ok(output)
    }

    fn validate_inputs(data: &[f32], shape: &[usize], padding: &[(usize, usize)]) -> Result<()> {
        if shape.len() != padding.len() {
            return Err(BarracudaError::invalid_params(
                "Pad",
                format!(
                    "Shape has {} dims but got {} padding specs",
                    shape.len(),
                    padding.len()
                ),
            ));
        }

        let expected_size: usize = shape.iter().product();
        if data.len() != expected_size {
            return Err(BarracudaError::invalid_params(
                "Pad",
                format!(
                    "Data length {} doesn't match shape {} elements",
                    data.len(),
                    expected_size
                ),
            ));
        }

        Ok(())
    }

    fn pad_1d(
        data: &[f32],
        shape: &[usize],
        padding: &[(usize, usize)],
        mode: PadMode,
        output: &mut [f32],
    ) {
        let (pad_before, pad_after) = padding[0];
        let dim = shape[0];

        // Fill padding before
        for i in 0..pad_before {
            output[i] = match mode {
                PadMode::Constant(val) => val,
                PadMode::Reflect => data[pad_before - i],
                PadMode::Replicate => data[0],
            };
        }

        // Copy data
        output[pad_before..pad_before + dim].copy_from_slice(data);

        // Fill padding after
        for i in 0..pad_after {
            let idx = pad_before + dim + i;
            output[idx] = match mode {
                PadMode::Constant(val) => val,
                PadMode::Reflect => data[dim - 2 - i],
                PadMode::Replicate => data[dim - 1],
            };
        }
    }

    fn pad_2d(
        data: &[f32],
        shape: &[usize],
        padding: &[(usize, usize)],
        mode: PadMode,
        output: &mut [f32],
    ) {
        let (pad_h_before, pad_h_after) = padding[0];
        let (pad_w_before, pad_w_after) = padding[1];
        let (h, w) = (shape[0], shape[1]);
        let out_w = w + pad_w_before + pad_w_after;

        for i in 0..(h + pad_h_before + pad_h_after) {
            for j in 0..(w + pad_w_before + pad_w_after) {
                let out_idx = i * out_w + j;

                // Determine if we're in padding region
                let in_h_range = i >= pad_h_before && i < pad_h_before + h;
                let in_w_range = j >= pad_w_before && j < pad_w_before + w;

                output[out_idx] = if in_h_range && in_w_range {
                    // Copy from input
                    let in_i = i - pad_h_before;
                    let in_j = j - pad_w_before;
                    data[in_i * w + in_j]
                } else {
                    // Padding value
                    match mode {
                        PadMode::Constant(val) => val,
                        PadMode::Reflect | PadMode::Replicate => {
                            // Simplified: use edge value for now
                            let safe_i = i.saturating_sub(pad_h_before).min(h - 1);
                            let safe_j = j.saturating_sub(pad_w_before).min(w - 1);
                            data[safe_i * w + safe_j]
                        }
                    }
                };
            }
        }
    }

    fn pad_3d(
        _data: &[f32],
        _shape: &[usize],
        _padding: &[(usize, usize)],
        _mode: PadMode,
        _output: &mut [f32],
    ) {
        // Placeholder for 3D padding (similar pattern to 2D)
        // Implementation follows same logic with 3 nested loops
        unimplemented!("3D padding not yet implemented");
    }
}

/// Cast Operation
///
/// Convert between data types (f32, f16, i8, u8).
///
/// ## Use Cases
///
/// - Quantized model inference (f32 → int8)
/// - Mixed precision training
/// - Memory optimization
/// - Neuromorphic chip integration

pub struct Transpose;

impl Transpose {
    /// Execute transpose operation
    ///
    /// # Arguments
    ///
    /// * `data` - Input tensor data (flat)
    /// * `shape` - Input dimensions
    /// * `dim0` - First dimension to swap
    /// * `dim1` - Second dimension to swap
    ///
    /// # Returns
    ///
    /// Transposed tensor
    ///
    /// # Errors
    ///
    /// Returns error if dimensions are invalid or out of bounds
    pub fn execute(data: &[f32], shape: &[usize], dim0: usize, dim1: usize) -> Result<Vec<f32>> {
        // Validate inputs
        Self::validate_inputs(data, shape, dim0, dim1)?;

        // Compute output shape (swap dims)
        let mut output_shape = shape.to_vec();
        output_shape.swap(dim0, dim1);

        // Allocate output
        let output_size: usize = shape.iter().product();
        let mut output = vec![0.0; output_size];

        // Perform transpose based on dimensionality
        match shape.len() {
            2 => Self::transpose_2d(data, shape, dim0, dim1, &mut output),
            3 => Self::transpose_3d(data, shape, dim0, dim1, &mut output),
            4 => Self::transpose_4d(data, shape, dim0, dim1, &mut output),
            _ => {
                return Err(BarracudaError::UnsupportedOperation {
                    operation: "Transpose".to_string(),
                    reason: format!("Only 2D-4D supported, got {}D", shape.len()),
                })
            }
        }

        Ok(output)
    }

    fn validate_inputs(data: &[f32], shape: &[usize], dim0: usize, dim1: usize) -> Result<()> {
        if shape.is_empty() {
            return Err(BarracudaError::invalid_params(
                "Transpose",
                "Shape cannot be empty",
            ));
        }

        if dim0 >= shape.len() {
            return Err(BarracudaError::invalid_params(
                "Transpose",
                format!("dim0 ({}) >= shape length ({})", dim0, shape.len()),
            ));
        }

        if dim1 >= shape.len() {
            return Err(BarracudaError::invalid_params(
                "Transpose",
                format!("dim1 ({}) >= shape length ({})", dim1, shape.len()),
            ));
        }

        let expected_size: usize = shape.iter().product();
        if data.len() != expected_size {
            return Err(BarracudaError::invalid_params(
                "Transpose",
                format!(
                    "Data length {} doesn't match shape {} elements",
                    data.len(),
                    expected_size
                ),
            ));
        }

        Ok(())
    }

    fn transpose_2d(data: &[f32], shape: &[usize], dim0: usize, dim1: usize, output: &mut [f32]) {
        let (rows, cols) = (shape[0], shape[1]);

        if dim0 == dim1 {
            // No-op transpose
            output.copy_from_slice(data);
            return;
        }

        // Standard 2D transpose
        for i in 0..rows {
            for j in 0..cols {
                let in_idx = i * cols + j;
                let out_idx = j * rows + i;
                output[out_idx] = data[in_idx];
            }
        }
    }

    fn transpose_3d(data: &[f32], shape: &[usize], dim0: usize, dim1: usize, output: &mut [f32]) {
        let (d0, d1, d2) = (shape[0], shape[1], shape[2]);

        if dim0 == dim1 {
            output.copy_from_slice(data);
            return;
        }

        // Create transposed shape
        let mut new_shape = [d0, d1, d2];
        new_shape.swap(dim0, dim1);

        // Compute strides
        let in_strides = [d1 * d2, d2, 1];
        let out_strides = [new_shape[1] * new_shape[2], new_shape[2], 1];

        // Perform transpose
        for i0 in 0..d0 {
            for i1 in 0..d1 {
                for i2 in 0..d2 {
                    let in_idx = i0 * in_strides[0] + i1 * in_strides[1] + i2 * in_strides[2];

                    // Map indices through permutation
                    let mut out_indices = [i0, i1, i2];
                    out_indices.swap(dim0, dim1);

                    let out_idx = out_indices[0] * out_strides[0]
                        + out_indices[1] * out_strides[1]
                        + out_indices[2] * out_strides[2];

                    output[out_idx] = data[in_idx];
                }
            }
        }
    }

    fn transpose_4d(data: &[f32], shape: &[usize], dim0: usize, dim1: usize, output: &mut [f32]) {
        let (d0, d1, d2, d3) = (shape[0], shape[1], shape[2], shape[3]);

        if dim0 == dim1 {
            output.copy_from_slice(data);
            return;
        }

        let mut new_shape = [d0, d1, d2, d3];
        new_shape.swap(dim0, dim1);

        let in_strides = [d1 * d2 * d3, d2 * d3, d3, 1];
        let out_strides = [
            new_shape[1] * new_shape[2] * new_shape[3],
            new_shape[2] * new_shape[3],
            new_shape[3],
            1,
        ];

        for i0 in 0..d0 {
            for i1 in 0..d1 {
                for i2 in 0..d2 {
                    for i3 in 0..d3 {
                        let in_idx = i0 * in_strides[0]
                            + i1 * in_strides[1]
                            + i2 * in_strides[2]
                            + i3 * in_strides[3];

                        let mut out_indices = [i0, i1, i2, i3];
                        out_indices.swap(dim0, dim1);

                        let out_idx = out_indices[0] * out_strides[0]
                            + out_indices[1] * out_strides[1]
                            + out_indices[2] * out_strides[2]
                            + out_indices[3] * out_strides[3];

                        output[out_idx] = data[in_idx];
                    }
                }
            }
        }
    }
}

/// Squeeze Operation
///
/// Removes dimensions of size 1 from tensor shape.
///
/// ## Use Cases
///
/// - Remove batch dimension: [1, H, W] → [H, W]
/// - Simplify shape: [1, C, 1, H, W] → [C, H, W]
/// - Model output cleanup
/// - Neuromorphic output formatting
pub struct Squeeze;

impl Squeeze {
    /// Remove all size-1 dimensions
    pub fn execute_all(data: &[f32], shape: &[usize]) -> Result<(Vec<f32>, Vec<usize>)> {
        Self::validate_input(data, shape)?;

        // Filter out size-1 dimensions
        let new_shape: Vec<usize> = shape.iter().filter(|&&d| d != 1).copied().collect();

        // If all dims were 1, result is scalar (shape [1])
        let new_shape = if new_shape.is_empty() {
            vec![1]
        } else {
            new_shape
        };

        Ok((data.to_vec(), new_shape))
    }

    /// Remove specific dimension (only if size is 1)
    pub fn execute_dim(
        data: &[f32],
        shape: &[usize],
        dim: usize,
    ) -> Result<(Vec<f32>, Vec<usize>)> {
        Self::validate_input(data, shape)?;

        if dim >= shape.len() {
            return Err(BarracudaError::invalid_params(
                "Squeeze",
                format!("Dimension {dim} out of bounds for shape {shape:?}"),
            ));
        }

        if shape[dim] != 1 {
            return Err(BarracudaError::invalid_params(
                "Squeeze",
                format!("Cannot squeeze dimension {} with size {}", dim, shape[dim]),
            ));
        }

        let mut new_shape = shape.to_vec();
        new_shape.remove(dim);

        // If all dims were removed, result is scalar
        let new_shape = if new_shape.is_empty() {
            vec![1]
        } else {
            new_shape
        };

        Ok((data.to_vec(), new_shape))
    }

    fn validate_input(data: &[f32], shape: &[usize]) -> Result<()> {
        if shape.is_empty() {
            return Err(BarracudaError::invalid_params(
                "Squeeze",
                "Shape cannot be empty",
            ));
        }

        let expected_size: usize = shape.iter().product();
        if data.len() != expected_size {
            return Err(BarracudaError::invalid_params(
                "Squeeze",
                format!(
                    "Data length {} doesn't match shape {} elements",
                    data.len(),
                    expected_size
                ),
            ));
        }

        Ok(())
    }
}

/// Unsqueeze Operation
///
/// Adds a dimension of size 1 at specified position.
///
/// ## Use Cases
///
/// - Add batch dimension: [H, W] → [1, H, W]
/// - Prepare for broadcasting
/// - Shape alignment for operations
/// - Neuromorphic input formatting
pub struct Unsqueeze;

impl Unsqueeze {
    /// Add dimension of size 1 at specified position
    pub fn execute(data: &[f32], shape: &[usize], dim: usize) -> Result<(Vec<f32>, Vec<usize>)> {
        Self::validate_input(data, shape, dim)?;

        let mut new_shape = shape.to_vec();
        new_shape.insert(dim, 1);

        Ok((data.to_vec(), new_shape))
    }

    fn validate_input(data: &[f32], shape: &[usize], dim: usize) -> Result<()> {
        if shape.is_empty() {
            return Err(BarracudaError::invalid_params(
                "Unsqueeze",
                "Shape cannot be empty",
            ));
        }

        if dim > shape.len() {
            return Err(BarracudaError::invalid_params(
                "Unsqueeze",
                format!(
                    "Dimension {} out of bounds (shape len {})",
                    dim,
                    shape.len()
                ),
            ));
        }

        let expected_size: usize = shape.iter().product();
        if data.len() != expected_size {
            return Err(BarracudaError::invalid_params(
                "Unsqueeze",
                format!(
                    "Data length {} doesn't match shape {} elements",
                    data.len(),
                    expected_size
                ),
            ));
        }

        Ok(())
    }
}

/// Expand Operation
///
/// Broadcasts tensor to a larger shape (repeats data).
///
/// ## Use Cases
///
/// - Broadcasting for element-wise operations
/// - Batch replication
/// - Feature expansion
/// - Memory-efficient tiling
pub struct Expand;

impl Expand {
    /// Broadcast tensor to target shape
    ///
    /// Dimensions of size 1 can be expanded to any size.
    /// Other dimensions must match.
    pub fn execute(data: &[f32], shape: &[usize], target_shape: &[usize]) -> Result<Vec<f32>> {
        Self::validate_inputs(data, shape, target_shape)?;

        let output_size: usize = target_shape.iter().product();
        let mut output = Vec::with_capacity(output_size);

        // Simple 1D and 2D cases
        if shape.len() == 1 && target_shape.len() == 2 {
            // [N] → [M, N]
            let repeats = target_shape[0];
            for _ in 0..repeats {
                output.extend_from_slice(data);
            }
        } else if shape.len() == 2 && target_shape.len() == 2 {
            // [1, N] → [M, N] or [M, 1] → [M, N]
            Self::expand_2d(data, shape, target_shape, &mut output);
        } else {
            return Err(BarracudaError::UnsupportedOperation {
                operation: "Expand".to_string(),
                reason: format!(
                    "Shape {} → {} not yet supported",
                    shape.len(),
                    target_shape.len()
                ),
            });
        }

        Ok(output)
    }

    fn expand_2d(data: &[f32], shape: &[usize], target_shape: &[usize], output: &mut Vec<f32>) {
        let (src_rows, src_cols) = (shape[0], shape[1]);
        let (tgt_rows, tgt_cols) = (target_shape[0], target_shape[1]);

        for i in 0..tgt_rows {
            for j in 0..tgt_cols {
                let src_i = if src_rows == 1 { 0 } else { i };
                let src_j = if src_cols == 1 { 0 } else { j };
                let src_idx = src_i * src_cols + src_j;
                output.push(data[src_idx]);
            }
        }
    }

    fn validate_inputs(data: &[f32], shape: &[usize], target_shape: &[usize]) -> Result<()> {
        if shape.len() != target_shape.len() && !(shape.len() == 1 && target_shape.len() == 2) {
            return Err(BarracudaError::invalid_params(
                "Expand",
                format!("Shape ranks must match or be broadcastable: {shape:?} → {target_shape:?}"),
            ));
        }

        // Check broadcastability
        let min_len = shape.len().min(target_shape.len());
        for i in 0..min_len {
            let src_dim = shape[shape.len() - 1 - i];
            let tgt_dim = target_shape[target_shape.len() - 1 - i];
            if src_dim != tgt_dim && src_dim != 1 {
                return Err(BarracudaError::invalid_params(
                    "Expand",
                    format!("Cannot broadcast dim {i} from {src_dim} to {tgt_dim}"),
                ));
            }
        }

        let expected_size: usize = shape.iter().product();
        if data.len() != expected_size {
            return Err(BarracudaError::invalid_params(
                "Expand",
                format!(
                    "Data length {} doesn't match shape {} elements",
                    data.len(),
                    expected_size
                ),
            ));
        }

        Ok(())
    }
}
