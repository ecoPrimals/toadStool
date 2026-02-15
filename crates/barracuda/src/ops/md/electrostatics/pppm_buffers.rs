//! PPPM GPU buffer helpers
//!
//! Extracted from pppm_gpu.rs for modularity (Feb 14, 2026).
//! Delegates to `crate::linalg::sparse::SparseBuffers` for shared implementation (Feb 15, 2026).

use crate::error::Result;
use crate::linalg::sparse::SparseBuffers;
use wgpu::util::DeviceExt;

/// PPPM buffer utilities for GPU memory management
///
/// Thin wrapper over `SparseBuffers` for electrostatics code paths that use raw
/// `wgpu::Device` and `wgpu::Queue` (e.g. PppmGpu).
pub struct PppmBuffers;

impl PppmBuffers {
    /// Create f64 buffer initialized with data
    pub fn f64_from_slice(device: &wgpu::Device, label: &str, data: &[f64]) -> wgpu::Buffer {
        SparseBuffers::f64_from_slice_raw(device, label, data)
    }

    /// Create zero-initialized f64 buffer
    pub fn f64_zeros(device: &wgpu::Device, label: &str, count: usize) -> wgpu::Buffer {
        SparseBuffers::f64_zeros_raw(device, label, count)
    }

    /// Create zero-initialized i32 buffer
    pub fn i32_zeros(device: &wgpu::Device, label: &str, count: usize) -> wgpu::Buffer {
        SparseBuffers::i32_zeros_raw(device, label, count)
    }

    /// Create i32 buffer from slice
    pub fn i32_from_slice(device: &wgpu::Device, label: &str, data: &[i32]) -> wgpu::Buffer {
        let bytes: Vec<u8> = data.iter().flat_map(|v| v.to_le_bytes()).collect();
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: &bytes,
            usage: wgpu::BufferUsages::STORAGE,
        })
    }

    /// Read f64 buffer back to CPU (sync; async wrapper for API compatibility)
    pub async fn read_f64(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        buffer: &wgpu::Buffer,
        count: usize,
    ) -> Result<Vec<f64>> {
        SparseBuffers::read_f64_raw(device, queue, buffer, count)
    }

    /// Read i32 buffer back to CPU (sync; async wrapper for API compatibility)
    pub async fn read_i32(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        buffer: &wgpu::Buffer,
        count: usize,
    ) -> Result<Vec<i32>> {
        SparseBuffers::read_i32_raw(device, queue, buffer, count)
    }
}

/// CPU FFT utilities for PPPM (fallback when GPU FFT not available)
pub struct PppmCpuFft;

impl PppmCpuFft {
    /// Forward 3D FFT (real input → complex output)
    pub fn forward_3d(mesh: &[f64], kx: usize, ky: usize, kz: usize) -> Vec<f64> {
        let size = kx * ky * kz;

        // Convert real mesh to complex
        let mut complex = vec![0.0f64; size * 2];
        for i in 0..size {
            complex[i * 2] = mesh[i];
        }

        // 3D FFT via 1D transforms
        Self::fft_3d(&mut complex, kx, ky, kz, false);

        complex
    }

    /// Inverse 3D FFT (complex input → real output, normalized)
    pub fn inverse_3d(phi_k: &[f64], kx: usize, ky: usize, kz: usize) -> Vec<f64> {
        let size = kx * ky * kz;

        let mut complex = phi_k.to_vec();
        Self::fft_3d(&mut complex, kx, ky, kz, true);

        // Extract real part and normalize
        let norm = 1.0 / (size as f64);
        (0..size).map(|i| complex[i * 2] * norm).collect()
    }

    /// 3D FFT via 1D transforms along each axis
    fn fft_3d(data: &mut [f64], kx: usize, ky: usize, kz: usize, inverse: bool) {
        // Transform along z
        for ix in 0..kx {
            for iy in 0..ky {
                let mut row: Vec<f64> = (0..kz)
                    .flat_map(|iz| {
                        let idx = (ix * ky * kz + iy * kz + iz) * 2;
                        vec![data[idx], data[idx + 1]]
                    })
                    .collect();
                Self::fft_1d(&mut row, kz, inverse);
                for iz in 0..kz {
                    let idx = (ix * ky * kz + iy * kz + iz) * 2;
                    data[idx] = row[iz * 2];
                    data[idx + 1] = row[iz * 2 + 1];
                }
            }
        }

        // Transform along y
        for ix in 0..kx {
            for iz in 0..kz {
                let mut row: Vec<f64> = (0..ky)
                    .flat_map(|iy| {
                        let idx = (ix * ky * kz + iy * kz + iz) * 2;
                        vec![data[idx], data[idx + 1]]
                    })
                    .collect();
                Self::fft_1d(&mut row, ky, inverse);
                for iy in 0..ky {
                    let idx = (ix * ky * kz + iy * kz + iz) * 2;
                    data[idx] = row[iy * 2];
                    data[idx + 1] = row[iy * 2 + 1];
                }
            }
        }

        // Transform along x
        for iy in 0..ky {
            for iz in 0..kz {
                let mut row: Vec<f64> = (0..kx)
                    .flat_map(|ix| {
                        let idx = (ix * ky * kz + iy * kz + iz) * 2;
                        vec![data[idx], data[idx + 1]]
                    })
                    .collect();
                Self::fft_1d(&mut row, kx, inverse);
                for ix in 0..kx {
                    let idx = (ix * ky * kz + iy * kz + iz) * 2;
                    data[idx] = row[ix * 2];
                    data[idx + 1] = row[ix * 2 + 1];
                }
            }
        }
    }

    /// Cooley-Tukey radix-2 1D FFT
    fn fft_1d(data: &mut [f64], n: usize, inverse: bool) {
        use std::f64::consts::PI;

        // Bit-reversal permutation
        let mut j = 0usize;
        for i in 0..n {
            if i < j {
                data.swap(i * 2, j * 2);
                data.swap(i * 2 + 1, j * 2 + 1);
            }
            let mut m = n / 2;
            while m >= 1 && j >= m {
                j -= m;
                m /= 2;
            }
            j += m;
        }

        // Cooley-Tukey iterations
        let sign = if inverse { 1.0 } else { -1.0 };
        let mut len = 2;
        while len <= n {
            let half = len / 2;
            let mut angle: f64 = 0.0;
            let angle_step = sign * PI / half as f64;

            for _ in 0..half {
                let (cos_a, sin_a) = (angle.cos(), angle.sin());
                for i in (0..n).step_by(len) {
                    let a_idx = (i + half) * 2;
                    let b_idx = i * 2;

                    let a_re = data[a_idx];
                    let a_im = data[a_idx + 1];

                    let t_re = cos_a * a_re - sin_a * a_im;
                    let t_im = sin_a * a_re + cos_a * a_im;

                    data[a_idx] = data[b_idx] - t_re;
                    data[a_idx + 1] = data[b_idx + 1] - t_im;
                    data[b_idx] += t_re;
                    data[b_idx + 1] += t_im;
                }
                angle += angle_step;
            }
            len *= 2;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_fft_roundtrip() {
        // Simple roundtrip test: forward then inverse should return original
        let mesh = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let kx = 2;
        let ky = 2;
        let kz = 2;

        let fft_result = PppmCpuFft::forward_3d(&mesh, kx, ky, kz);
        let recovered = PppmCpuFft::inverse_3d(&fft_result, kx, ky, kz);

        for (orig, rec) in mesh.iter().zip(recovered.iter()) {
            assert!((orig - rec).abs() < 1e-10, "FFT roundtrip failed");
        }
    }
}
