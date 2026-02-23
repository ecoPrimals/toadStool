//! Indexing operations: Argmax, TopK, Where

use crate::error::{BarracudaError, Result};

pub struct Argmax;

impl Argmax {
    /// Find argmax along last axis
    ///
    /// # Arguments
    ///
    /// * `data` - Input tensor
    /// * `shape` - Tensor dimensions
    ///
    /// # Returns
    ///
    /// Indices of maximum values
    pub fn execute(data: &[f32], shape: &[usize]) -> Result<Vec<usize>> {
        if shape.is_empty() {
            return Err(BarracudaError::invalid_params(
                "Argmax",
                "Shape cannot be empty",
            ));
        }

        let last_dim = shape[shape.len() - 1];
        let num_groups = data.len() / last_dim;

        let mut indices = Vec::with_capacity(num_groups);

        for group_idx in 0..num_groups {
            let start = group_idx * last_dim;
            let end = start + last_dim;
            let group = &data[start..end];

            // Find index of maximum
            let max_idx = group
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(idx, _)| idx)
                .unwrap_or(0);

            indices.push(max_idx);
        }

        Ok(indices)
    }
}

/// TopK Operation
///
/// Finds K largest elements and their indices.
///
/// ## Use Cases
///
/// - Beam search
/// - Top-K accuracy
/// - Confidence filtering
/// - Multi-label classification
pub struct TopK;

impl TopK {
    /// Find top K largest elements
    ///
    /// # Arguments
    ///
    /// * `data` - Input tensor
    /// * `k` - Number of top elements
    ///
    /// # Returns
    ///
    /// Tuple of (values, indices) for top K elements
    pub fn execute(data: &[f32], k: usize) -> Result<(Vec<f32>, Vec<usize>)> {
        if k == 0 {
            return Err(BarracudaError::invalid_params("TopK", "K must be > 0"));
        }

        if k > data.len() {
            return Err(BarracudaError::invalid_params(
                "TopK",
                format!("K ({}) exceeds data length ({})", k, data.len()),
            ));
        }

        // Create (value, index) pairs
        let mut indexed: Vec<(f32, usize)> =
            data.iter().enumerate().map(|(i, &v)| (v, i)).collect();

        // Partial sort to get top K
        indexed.select_nth_unstable_by(k - 1, |a, b| {
            b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Take top K and sort them
        let mut top_k: Vec<(f32, usize)> = indexed[..k].to_vec();
        top_k.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        // Separate values and indices
        let values: Vec<f32> = top_k.iter().map(|(v, _)| *v).collect();
        let indices: Vec<usize> = top_k.iter().map(|(_, i)| *i).collect();

        Ok((values, indices))
    }
}

/// Where Operation (Conditional Select)
///
/// Selects elements from `true_vals` or `false_vals` based on condition.
///
/// ## Use Cases
///
/// - Conditional masking
/// - ReLU variants: max(0, x) = where(x > 0, x, 0)
/// - Thresholding
/// - Neuromorphic decision logic
pub struct Where;

impl Where {
    /// Select elements based on boolean condition
    pub fn execute(condition: &[bool], true_vals: &[f32], false_vals: &[f32]) -> Result<Vec<f32>> {
        Self::validate_inputs(condition, true_vals, false_vals)?;

        let result = condition
            .iter()
            .zip(true_vals.iter())
            .zip(false_vals.iter())
            .map(|((&cond, &t), &f)| if cond { t } else { f })
            .collect();

        Ok(result)
    }

    fn validate_inputs(condition: &[bool], true_vals: &[f32], false_vals: &[f32]) -> Result<()> {
        if condition.len() != true_vals.len() {
            return Err(BarracudaError::invalid_params(
                "Where",
                format!(
                    "Condition length {} != true_vals length {}",
                    condition.len(),
                    true_vals.len()
                ),
            ));
        }

        if condition.len() != false_vals.len() {
            return Err(BarracudaError::invalid_params(
                "Where",
                format!(
                    "Condition length {} != false_vals length {}",
                    condition.len(),
                    false_vals.len()
                ),
            ));
        }

        Ok(())
    }
}
