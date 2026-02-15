//! 3D Fast Fourier Transform Operation (f64 Precision)
//!
//! **Purpose**: 3D FFT for PPPM molecular dynamics with f64 precision
//! **Algorithm**: Dimension-wise decomposition using 1D FFT
//!
//! **CRITICAL FOR PPPM**: Unblocks full GPU PPPM electrostatics!
//!
//! ## Precision Philosophy
//!
//! **Full f64 precision** via WGSL native f64 and SPIR-V/Vulkan.
//! FP64 performance is 1:2-3 (not 1:32 like CUDA consumer GPUs).

use super::Fft1DF64;
use crate::device::WgpuDevice;
use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;
use std::sync::Arc;

/// 3D Complex FFT operation (f64 precision)
///
/// Performs 3D FFT via row-column decomposition using 1D f64 FFTs.
/// Designed for PPPM/Ewald electrostatics integration.
pub struct Fft3DF64 {
    device: Arc<WgpuDevice>,
    nx: usize,
    ny: usize,
    nz: usize,
}

impl Fft3DF64 {
    /// Create a new 3D FFT operation
    ///
    /// # Arguments
    /// * `device` - WgpuDevice for GPU execution
    /// * `nx`, `ny`, `nz` - FFT dimensions (must be powers of 2)
    pub fn new(device: Arc<WgpuDevice>, nx: usize, ny: usize, nz: usize) -> Result<Self> {
        // Validate dimensions are powers of 2
        if !nx.is_power_of_two() || !ny.is_power_of_two() || !nz.is_power_of_two() {
            return Err(BarracudaError::InvalidInput {
                message: format!(
                    "FFT 3D dimensions must be powers of 2, got ({}, {}, {})",
                    nx, ny, nz
                ),
            });
        }

        Ok(Self { device, nx, ny, nz })
    }

    /// Execute forward 3D FFT
    ///
    /// # Arguments
    /// * `data` - Complex input data as interleaved f64 `[re, im, re, im, ...]`
    ///   Size must be `nx*ny*nz*2`
    ///
    /// # Returns
    /// Complex output in same format
    pub async fn forward(&self, data: &[f64]) -> Result<Vec<f64>> {
        self.execute_internal(data, false).await
    }

    /// Execute inverse 3D FFT
    ///
    /// # Arguments
    /// * `data` - Complex input data as interleaved f64
    ///
    /// # Returns
    /// Complex output (caller should divide by nx*ny*nz for proper normalization)
    pub async fn inverse(&self, data: &[f64]) -> Result<Vec<f64>> {
        self.execute_internal(data, true).await
    }

    async fn execute_internal(&self, data: &[f64], inverse: bool) -> Result<Vec<f64>> {
        let size = self.nx * self.ny * self.nz;
        let expected_len = size * 2;

        if data.len() != expected_len {
            return Err(BarracudaError::InvalidInput {
                message: format!(
                    "FFT 3D data length {} doesn't match expected {} ({}x{}x{}x2)",
                    data.len(),
                    expected_len,
                    self.nx,
                    self.ny,
                    self.nz
                ),
            });
        }

        let mut working = data.to_vec();

        // Transform along Z (innermost)
        for ix in 0..self.nx {
            for iy in 0..self.ny {
                let mut pencil = self.extract_z_pencil(&working, ix, iy);
                pencil = self.fft_1d(&pencil, self.nz, inverse).await?;
                self.insert_z_pencil(&mut working, ix, iy, &pencil);
            }
        }

        // Transform along Y
        for ix in 0..self.nx {
            for iz in 0..self.nz {
                let mut pencil = self.extract_y_pencil(&working, ix, iz);
                pencil = self.fft_1d(&pencil, self.ny, inverse).await?;
                self.insert_y_pencil(&mut working, ix, iz, &pencil);
            }
        }

        // Transform along X (outermost)
        for iy in 0..self.ny {
            for iz in 0..self.nz {
                let mut pencil = self.extract_x_pencil(&working, iy, iz);
                pencil = self.fft_1d(&pencil, self.nx, inverse).await?;
                self.insert_x_pencil(&mut working, iy, iz, &pencil);
            }
        }

        Ok(working)
    }

    /// Run 1D FFT on a pencil using GPU
    async fn fft_1d(&self, pencil: &[f64], n: usize, inverse: bool) -> Result<Vec<f64>> {
        // Create tensor from pencil data
        let tensor = Tensor::from_f64_data(pencil, vec![n, 2], self.device.clone())?;

        // Create FFT operation
        let fft = Fft1DF64::new(tensor, n as u32)?;

        // Execute
        let result = if inverse {
            fft.execute_inverse().await?
        } else {
            fft.execute().await?
        };

        // Read back result
        result.to_f64_vec()
    }

    // Pencil extraction/insertion helpers (row-major layout: [x][y][z][complex])

    fn extract_z_pencil(&self, data: &[f64], ix: usize, iy: usize) -> Vec<f64> {
        let mut pencil = Vec::with_capacity(self.nz * 2);
        for iz in 0..self.nz {
            let idx = self.linear_index(ix, iy, iz);
            pencil.push(data[idx * 2]);
            pencil.push(data[idx * 2 + 1]);
        }
        pencil
    }

    fn insert_z_pencil(&self, data: &mut [f64], ix: usize, iy: usize, pencil: &[f64]) {
        for iz in 0..self.nz {
            let idx = self.linear_index(ix, iy, iz);
            data[idx * 2] = pencil[iz * 2];
            data[idx * 2 + 1] = pencil[iz * 2 + 1];
        }
    }

    fn extract_y_pencil(&self, data: &[f64], ix: usize, iz: usize) -> Vec<f64> {
        let mut pencil = Vec::with_capacity(self.ny * 2);
        for iy in 0..self.ny {
            let idx = self.linear_index(ix, iy, iz);
            pencil.push(data[idx * 2]);
            pencil.push(data[idx * 2 + 1]);
        }
        pencil
    }

    fn insert_y_pencil(&self, data: &mut [f64], ix: usize, iz: usize, pencil: &[f64]) {
        for iy in 0..self.ny {
            let idx = self.linear_index(ix, iy, iz);
            data[idx * 2] = pencil[iy * 2];
            data[idx * 2 + 1] = pencil[iy * 2 + 1];
        }
    }

    fn extract_x_pencil(&self, data: &[f64], iy: usize, iz: usize) -> Vec<f64> {
        let mut pencil = Vec::with_capacity(self.nx * 2);
        for ix in 0..self.nx {
            let idx = self.linear_index(ix, iy, iz);
            pencil.push(data[idx * 2]);
            pencil.push(data[idx * 2 + 1]);
        }
        pencil
    }

    fn insert_x_pencil(&self, data: &mut [f64], iy: usize, iz: usize, pencil: &[f64]) {
        for ix in 0..self.nx {
            let idx = self.linear_index(ix, iy, iz);
            data[idx * 2] = pencil[ix * 2];
            data[idx * 2 + 1] = pencil[ix * 2 + 1];
        }
    }

    #[inline]
    fn linear_index(&self, ix: usize, iy: usize, iz: usize) -> usize {
        ix * self.ny * self.nz + iy * self.nz + iz
    }

    /// Get dimensions
    pub fn dims(&self) -> (usize, usize, usize) {
        (self.nx, self.ny, self.nz)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fft_3d_f64_roundtrip() {
        let Some(device) = crate::device::test_pool::get_test_device_if_gpu_available().await
        else {
            return;
        };

        // 4×4×4 FFT test
        let n = 4;
        let size = n * n * n;

        // Create test signal: impulse at (0,0,0)
        let mut data = vec![0.0f64; size * 2];
        data[0] = 1.0; // Real part of (0,0,0)

        let fft = Fft3DF64::new(device.clone(), n, n, n).unwrap();

        // Forward FFT
        let freq = fft.forward(&data).await.unwrap();
        assert_eq!(freq.len(), size * 2);

        // For impulse input, all frequency bins should have magnitude 1
        // (DC component = 1.0 + 0i at each point)
        for i in 0..size {
            let re = freq[i * 2];
            let im = freq[i * 2 + 1];
            let mag = (re * re + im * im).sqrt();
            assert!(
                (mag - 1.0).abs() < 1e-10,
                "Expected magnitude 1.0, got {}",
                mag
            );
        }

        // Inverse FFT
        let back = fft.inverse(&freq).await.unwrap();

        // Normalize
        let norm = (size as f64).recip();
        let back_norm: Vec<f64> = back.iter().map(|x| x * norm).collect();

        // Should recover original impulse
        assert!((back_norm[0] - 1.0).abs() < 1e-10);
        for i in 1..size {
            assert!((back_norm[i * 2]).abs() < 1e-10);
            assert!((back_norm[i * 2 + 1]).abs() < 1e-10);
        }
    }
}
