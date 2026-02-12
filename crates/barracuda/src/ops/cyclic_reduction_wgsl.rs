//! Cyclic Reduction for Tridiagonal Systems (WGSL GPU Implementation)
//!
//! This is the SHADER-FIRST implementation of tridiagonal solvers.
//! Uses cyclic reduction (odd-even elimination) for O(log n) parallel steps
//! instead of O(n) sequential steps in the Thomas algorithm.
//!
//! # Algorithm
//!
//! Cyclic reduction eliminates odd-indexed unknowns in parallel:
//! 1. Reduction phase: O(log n) steps, each eliminating half the unknowns
//! 2. Solve the 1-element system at the center
//! 3. Substitution phase: O(log n) steps, recovering eliminated unknowns
//!
//! # When to Use
//!
//! - **Large systems (n > 256)**: Cyclic reduction wins on GPU
//! - **Small systems (n ≤ 256)**: Single workgroup shared memory version
//! - **Batched systems**: Many independent tridiagonal systems in parallel
//!
//! # Future Hardware
//!
//! Same math runs on:
//! - Current GPUs (fp32)
//! - Future fp64 GPUs (seamless transition via ToadStool dispatch)
//! - Quantum sequential compute (same algorithm, different substrate)

use crate::device::wgpu_device::WgpuDevice;
use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;
use bytemuck::{Pod, Zeroable};

/// Parameters for cyclic reduction shader
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct CyclicReductionParams {
    n: u32,
    step: u32,
    phase: u32,
    _pad: u32,
}

const SHADER_SOURCE: &str = include_str!("../shaders/linalg/cyclic_reduction.wgsl");

/// Solve tridiagonal system using GPU cyclic reduction.
///
/// Solves: a[i]*x[i-1] + b[i]*x[i] + c[i]*x[i+1] = d[i]
///
/// # Arguments
///
/// * `a` - Sub-diagonal tensor (length n-1, padded to n)
/// * `b` - Main diagonal tensor (length n)
/// * `c` - Super-diagonal tensor (length n-1, padded to n)
/// * `d` - Right-hand side tensor (length n)
///
/// # Returns
///
/// Solution tensor x of length n.
///
/// # Example
///
/// ```ignore
/// use barracuda::ops::cyclic_reduction_wgsl::tridiagonal_solve_gpu;
/// use barracuda::tensor::Tensor;
///
/// let a = Tensor::from_vec(vec![1.0f32, 1.0, 0.0], vec![3]);
/// let b = Tensor::from_vec(vec![4.0f32, 4.0, 4.0], vec![3]);
/// let c = Tensor::from_vec(vec![1.0f32, 1.0, 0.0], vec![3]);
/// let d = Tensor::from_vec(vec![5.0f32, 6.0, 5.0], vec![3]);
///
/// let x = tridiagonal_solve_gpu(&a, &b, &c, &d)?;
/// ```
pub fn tridiagonal_solve_gpu(
    a: &Tensor,
    b: &Tensor,
    c: &Tensor,
    d: &Tensor,
) -> Result<Tensor> {
    let n = b.len();
    
    if n == 0 {
        return Err(BarracudaError::InvalidInput {
            message: "System size must be > 0".to_string(),
        });
    }
    
    // For small systems, use the shared memory single-pass version
    if n <= 256 {
        return tridiagonal_solve_gpu_small(a, b, c, d);
    }
    
    // For large systems, use multi-pass cyclic reduction
    tridiagonal_solve_gpu_large(a, b, c, d)
}

/// Single-pass solver for small systems (n ≤ 256)
/// Uses shared memory for efficiency
fn tridiagonal_solve_gpu_small(
    a: &Tensor,
    b: &Tensor,
    c: &Tensor,
    d: &Tensor,
) -> Result<Tensor> {
    let n = b.len();
    let device = WgpuDevice::new()?;
    
    // Create padded copies of the diagonals
    let mut a_data = a.to_vec_f32()?;
    let mut b_data = b.to_vec_f32()?;
    let mut c_data = c.to_vec_f32()?;
    let mut d_data = d.to_vec_f32()?;
    
    // Pad to power of 2 for cyclic reduction
    let n_padded = n.next_power_of_two();
    a_data.resize(n_padded, 0.0);
    b_data.resize(n_padded, 1.0);  // Identity for padded rows
    c_data.resize(n_padded, 0.0);
    d_data.resize(n_padded, 0.0);
    
    let params = CyclicReductionParams {
        n: n as u32,
        step: 0,
        phase: 0,
        _pad: 0,
    };
    
    // Create GPU buffers
    let params_buffer = device.create_uniform_buffer(&params);
    let a_buffer = device.create_storage_buffer_init(&a_data);
    let b_buffer = device.create_storage_buffer_init(&b_data);
    let c_buffer = device.create_storage_buffer_init(&c_data);
    let d_buffer = device.create_storage_buffer_init(&d_data);
    
    // Create compute pipeline
    let pipeline = device.create_compute_pipeline(SHADER_SOURCE, "solve_small")?;
    
    // Bind group
    let bind_group = device.create_bind_group(
        &pipeline,
        &[&params_buffer, &a_buffer, &b_buffer, &c_buffer, &d_buffer],
    );
    
    // Dispatch single workgroup
    device.dispatch(&pipeline, &bind_group, (1, 1, 1))?;
    
    // Read back result (d now contains solution)
    let result = device.read_buffer_f32(&d_buffer, n)?;
    
    Ok(Tensor::from_vec(result, vec![n]))
}

/// Multi-pass solver for large systems (n > 256)
fn tridiagonal_solve_gpu_large(
    a: &Tensor,
    b: &Tensor,
    c: &Tensor,
    d: &Tensor,
) -> Result<Tensor> {
    let n = b.len();
    let device = WgpuDevice::new()?;
    
    // Pad to power of 2
    let n_padded = n.next_power_of_two();
    let num_steps = (n_padded as f64).log2() as u32;
    
    let mut a_data = a.to_vec_f32()?;
    let mut b_data = b.to_vec_f32()?;
    let mut c_data = c.to_vec_f32()?;
    let mut d_data = d.to_vec_f32()?;
    
    a_data.resize(n_padded, 0.0);
    b_data.resize(n_padded, 1.0);
    c_data.resize(n_padded, 0.0);
    d_data.resize(n_padded, 0.0);
    
    // Create buffers (read_write for iterative updates)
    let a_buffer = device.create_storage_buffer_init(&a_data);
    let b_buffer = device.create_storage_buffer_init(&b_data);
    let c_buffer = device.create_storage_buffer_init(&c_data);
    let d_buffer = device.create_storage_buffer_init(&d_data);
    
    // Create pipelines
    let reduction_pipeline = device.create_compute_pipeline(SHADER_SOURCE, "reduction")?;
    let substitution_pipeline = device.create_compute_pipeline(SHADER_SOURCE, "substitution")?;
    
    // Reduction phase
    for step in 0..num_steps {
        let params = CyclicReductionParams {
            n: n_padded as u32,
            step,
            phase: 0,
            _pad: 0,
        };
        let params_buffer = device.create_uniform_buffer(&params);
        
        let bind_group = device.create_bind_group(
            &reduction_pipeline,
            &[&params_buffer, &a_buffer, &b_buffer, &c_buffer, &d_buffer],
        );
        
        let stride = 1 << (step + 1);
        let workgroups = ((n_padded / stride) + 255) / 256;
        device.dispatch(&reduction_pipeline, &bind_group, (workgroups.max(1), 1, 1))?;
    }
    
    // Solve center element (trivial - just d[n/2] / b[n/2])
    // This is implicitly handled by the reduction
    
    // Substitution phase (reverse order)
    for step in (0..num_steps).rev() {
        let params = CyclicReductionParams {
            n: n_padded as u32,
            step,
            phase: 1,
            _pad: 0,
        };
        let params_buffer = device.create_uniform_buffer(&params);
        
        let bind_group = device.create_bind_group(
            &substitution_pipeline,
            &[&params_buffer, &a_buffer, &b_buffer, &c_buffer, &d_buffer],
        );
        
        let stride = 1 << (step + 1);
        let workgroups = ((n_padded / stride) + 255) / 256;
        device.dispatch(&substitution_pipeline, &bind_group, (workgroups.max(1), 1, 1))?;
    }
    
    // Read back result
    let result = device.read_buffer_f32(&d_buffer, n)?;
    
    Ok(Tensor::from_vec(result, vec![n]))
}

/// Batch solve multiple independent tridiagonal systems in parallel.
///
/// Each system has the same size but different coefficients/RHS.
/// This is extremely efficient for:
/// - ADI methods (2D/3D PDE with row/column sweeps)
/// - Monte Carlo with multiple realizations
///
/// # Arguments
///
/// * `a_batch` - Sub-diagonals [batch_size × n]
/// * `b_batch` - Main diagonals [batch_size × n]
/// * `c_batch` - Super-diagonals [batch_size × n]
/// * `d_batch` - Right-hand sides [batch_size × n]
///
/// # Returns
///
/// Solutions [batch_size × n]
pub fn tridiagonal_solve_batch_gpu(
    a_batch: &Tensor,
    b_batch: &Tensor,
    c_batch: &Tensor,
    d_batch: &Tensor,
) -> Result<Tensor> {
    let shape = b_batch.shape();
    if shape.len() != 2 {
        return Err(BarracudaError::InvalidInput {
            message: "Batch tensors must be 2D [batch_size × n]".to_string(),
        });
    }
    
    let batch_size = shape[0];
    let n = shape[1];
    
    // For now, solve each system independently
    // TODO: True batched kernel with 2D dispatch
    let mut results = Vec::with_capacity(batch_size * n);
    
    for i in 0..batch_size {
        let a_i = a_batch.slice(i, 0)?;
        let b_i = b_batch.slice(i, 0)?;
        let c_i = c_batch.slice(i, 0)?;
        let d_i = d_batch.slice(i, 0)?;
        
        let x_i = tridiagonal_solve_gpu(&a_i, &b_i, &c_i, &d_i)?;
        results.extend(x_i.to_vec_f32()?);
    }
    
    Ok(Tensor::from_vec(results, vec![batch_size, n]))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_tridiagonal_small() {
        // Skip if no GPU
        if WgpuDevice::new().is_err() {
            return;
        }
        
        // Simple 3x3 system
        let a = Tensor::from_vec(vec![1.0f32, 1.0, 0.0], vec![3]);
        let b = Tensor::from_vec(vec![4.0f32, 4.0, 4.0], vec![3]);
        let c = Tensor::from_vec(vec![1.0f32, 1.0, 0.0], vec![3]);
        let d = Tensor::from_vec(vec![5.0f32, 6.0, 5.0], vec![3]);
        
        let x = tridiagonal_solve_gpu(&a, &b, &c, &d).unwrap();
        let x_data = x.to_vec_f32().unwrap();
        
        // Verify solution: A·x ≈ d
        // Row 0: 4*x[0] + 1*x[1] = 5
        let ax0 = 4.0 * x_data[0] + 1.0 * x_data[1];
        assert!((ax0 - 5.0).abs() < 0.1, "Row 0: {} vs 5.0", ax0);
    }
    
    #[test]
    fn test_tridiagonal_heat_equation() {
        if WgpuDevice::new().is_err() {
            return;
        }
        
        // Heat equation discretization: -u_{i-1} + 2u_i - u_{i+1} = f_i
        let n = 10;
        let a = Tensor::from_vec(vec![-1.0f32; n], vec![n]);
        let b = Tensor::from_vec(vec![2.0f32; n], vec![n]);
        let c = Tensor::from_vec(vec![-1.0f32; n], vec![n]);
        let d = Tensor::from_vec(vec![1.0f32; n], vec![n]);
        
        let x = tridiagonal_solve_gpu(&a, &b, &c, &d).unwrap();
        assert_eq!(x.len(), n);
    }
    
    #[test]
    fn test_tridiagonal_identity() {
        if WgpuDevice::new().is_err() {
            return;
        }
        
        // Identity matrix: b=1, a=c=0
        let a = Tensor::from_vec(vec![0.0f32, 0.0, 0.0], vec![3]);
        let b = Tensor::from_vec(vec![1.0f32, 1.0, 1.0], vec![3]);
        let c = Tensor::from_vec(vec![0.0f32, 0.0, 0.0], vec![3]);
        let d = Tensor::from_vec(vec![1.0f32, 2.0, 3.0], vec![3]);
        
        let x = tridiagonal_solve_gpu(&a, &b, &c, &d).unwrap();
        let x_data = x.to_vec_f32().unwrap();
        
        assert!((x_data[0] - 1.0).abs() < 0.01);
        assert!((x_data[1] - 2.0).abs() < 0.01);
        assert!((x_data[2] - 3.0).abs() < 0.01);
    }
}
