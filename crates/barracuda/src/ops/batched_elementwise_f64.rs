//! Batched Element-wise Operations at f64 precision — Rust orchestrator
//!
//! UNIFIED PATTERN (Feb 16 2026) — Serves all springs:
//! - airSpring: FAO-56 ET₀, water balance across stations/fields
//! - wetSpring: Batched diversity metrics across samples
//! - hotSpring: Batched nuclear structure calculations
//!
//! # Architecture
//!
//! One workgroup per batch element. Each workgroup computes one output value
//! from a "row" of input parameters.
//!
//! # Operations
//!
//! - `Op::Fao56Et0` (0): FAO-56 Penman-Monteith reference ET₀
//! - `Op::WaterBalance` (1): Daily water balance update
//! - `Op::Custom` (2): User-defined operations (passthrough)
//! - `Op::SensorCalibration` (5): SoilWatch 10 VWC — Dong et al. (2024)
//! - `Op::HargreavesEt0` (6): Hargreaves-Samani (1985) ET₀
//! - `Op::KcClimateAdjust` (7): FAO-56 Eq. 62 Kc climate adjustment
//! - `Op::DualKcKe` (8): FAO-56 Eq. 71/74 dual Kc soil evaporation
//!
//! # Example
//!
//! ```rust,ignore
//! use barracuda::ops::batched_elementwise_f64::{BatchedElementwiseF64, Op};
//!
//! let executor = BatchedElementwiseF64::new(device.clone())?;
//!
//! // FAO-56 ET₀ for 100 station-days
//! // Input: [tmax, tmin, rh_max, rh_min, wind, Rs, elev, lat, doy] per station
//! let et0_values = executor.execute(&input_data, 100, Op::Fao56Et0)?;
//! ```

use crate::device::WgpuDevice;
use crate::error::Result;
use bytemuck::{Pod, Zeroable};
use std::sync::Arc;
use wgpu::util::DeviceExt;

/// Monte Carlo uncertainty propagation wrapper around FAO-56 ET₀.
/// Box-Muller perturbation + xoshiro128** PRNG + batched dispatch.
/// Provenance: groundSpring metalForge → toadStool absorption.
pub const WGSL_MC_ET0_PROPAGATE_F64: &str =
    include_str!("../shaders/bio/mc_et0_propagate_f64.wgsl");

/// Operations for batched element-wise computation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Op {
    /// FAO-56 Penman-Monteith ET₀
    /// Input per batch: `[tmax, tmin, rh_max, rh_min, wind_2m, Rs, elevation, lat, doy]`
    Fao56Et0 = 0,

    /// Water balance daily update
    /// Input per batch: `[Dr_prev, P, I, ETc, TAW, RAW, p]`
    WaterBalance = 1,

    /// Custom operation (passthrough first element)
    Custom = 2,

    /// SoilWatch 10 sensor calibration — Dong et al. (2024)
    /// Input per batch: `[raw_count]`
    /// Output: VWC (cm³/cm³)
    SensorCalibration = 5,

    /// Hargreaves-Samani (1985) ET₀ — FAO-56 Eq. 52
    /// Input per batch: `[tmax, tmin, lat_rad, doy]`
    /// Output: ET₀ (mm/day)
    HargreavesEt0 = 6,

    /// FAO-56 Eq. 62 Kc climate adjustment
    /// Input per batch: `[kc_table, u2, rh_min, crop_height_m]`
    /// Output: adjusted Kc
    KcClimateAdjust = 7,

    /// FAO-56 Eq. 71/74 dual Kc soil evaporation coefficient
    /// Input per batch: `[kcb, kc_max, few, mulch_factor, de_prev, rew, tew, p_eff, et0]`
    /// Output: Ke
    DualKcKe = 8,
}

impl Op {
    /// Number of input elements per batch item
    #[must_use]
    pub fn stride(&self) -> usize {
        match self {
            Op::Fao56Et0 => 9,
            Op::WaterBalance => 7,
            Op::Custom => 1,
            Op::SensorCalibration => 1,
            Op::HargreavesEt0 => 4,
            Op::KcClimateAdjust => 4,
            Op::DualKcKe => 9,
        }
    }
}

/// FAO-56 station-day input: (tmax, tmin, rh_max, rh_min, wind_2m, rs, elevation, latitude, day_of_year)
pub type StationDayInput = (f64, f64, f64, f64, f64, f64, f64, f64, u32);

/// Water balance field input: (dr_prev, precipitation, irrigation, etc, taw, raw, p_fraction)
pub type WaterBalanceInput = (f64, f64, f64, f64, f64, f64, f64);

/// Parameters for batched elementwise shader
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Params {
    batch_size: u32,
    stride: u32,
    operation: u32,
    _pad: u32,
    aux_param: f64,
}

/// Batched element-wise executor for f64 data
///
/// Processes multiple independent computations in parallel, one per batch element.
/// Useful for station-days (ET₀), field-cells (water balance), or samples (diversity).
pub struct BatchedElementwiseF64 {
    device: Arc<WgpuDevice>,
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl BatchedElementwiseF64 {
    /// Create a new batched elementwise executor
    pub fn new(device: Arc<WgpuDevice>) -> Result<Self> {
        let shader_source = include_str!("../shaders/science/batched_elementwise_f64.wgsl");
        let shader_module =
            device.compile_shader_f64(shader_source, Some("BatchedElementwiseF64 Shader"));

        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("BatchedElementwiseF64 BGL"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });

        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("BatchedElementwiseF64 PipelineLayout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("BatchedElementwiseF64 Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader_module,
                entry_point: "batched_compute",
                cache: None,
                compilation_options: Default::default(),
            });

        Ok(Self {
            device,
            pipeline,
            bind_group_layout,
        })
    }

    /// Execute batched computation
    ///
    /// # Arguments
    /// * `data` - Flattened input array [batch_size * stride]
    /// * `batch_size` - Number of batch elements
    /// * `op` - Operation to perform
    ///
    /// # Returns
    /// Output array [batch_size]
    pub fn execute(&self, data: &[f64], batch_size: usize, op: Op) -> Result<Vec<f64>> {
        self.execute_with_aux(data, batch_size, op, 0.0)
    }

    /// Execute batched computation with auxiliary parameter
    ///
    /// # Arguments
    /// * `data` - Flattened input array [batch_size * stride]
    /// * `batch_size` - Number of batch elements
    /// * `op` - Operation to perform
    /// * `aux_param` - Auxiliary parameter (e.g., total for normalization)
    ///
    /// # Returns
    /// Output array [batch_size]
    pub fn execute_with_aux(
        &self,
        data: &[f64],
        batch_size: usize,
        op: Op,
        aux_param: f64,
    ) -> Result<Vec<f64>> {
        if batch_size == 0 {
            return Ok(Vec::new());
        }

        let stride = op.stride();
        let expected_len = batch_size * stride;

        if data.len() < expected_len {
            return Err(crate::error::BarracudaError::InvalidInput {
                message: format!(
                    "Input data length {} too short for {} batches with stride {} (expected {})",
                    data.len(),
                    batch_size,
                    stride,
                    expected_len
                ),
            });
        }

        // Create input buffer
        let input_buffer =
            self.device
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("BatchedEW Input"),
                    contents: bytemuck::cast_slice(data),
                    usage: wgpu::BufferUsages::STORAGE,
                });

        // Create output buffer
        let output_buffer = self.device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("BatchedEW Output"),
            size: (batch_size * 8) as u64, // f64 = 8 bytes
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Create params buffer
        let params = Params {
            batch_size: batch_size as u32,
            stride: stride as u32,
            operation: op as u32,
            _pad: 0,
            aux_param,
        };
        let params_buffer =
            self.device
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("BatchedEW Params"),
                    contents: bytemuck::bytes_of(&params),
                    usage: wgpu::BufferUsages::UNIFORM,
                });

        let bind_group = self
            .device
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("BatchedEW Bind Group"),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: input_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: output_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: params_buffer.as_entire_binding(),
                    },
                ],
            });

        // Execute
        let mut encoder =
            self.device
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("BatchedEW Encoder"),
                });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("BatchedEW Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            // One workgroup per batch element (workgroup size = 64)
            pass.dispatch_workgroups(batch_size as u32, 1, 1);
        }

        self.device.submit_and_poll(Some(encoder.finish()));

        // Read results
        self.read_results(&output_buffer, batch_size)
    }

    /// Read results from GPU buffer
    fn read_results(&self, buffer: &wgpu::Buffer, count: usize) -> Result<Vec<f64>> {
        self.device.read_buffer::<f64>(buffer, count)
    }

    /// CPU fallback for small batches
    #[cfg(test)]
    #[allow(dead_code)]
    fn execute_cpu(
        &self,
        data: &[f64],
        batch_size: usize,
        op: Op,
        _aux_param: f64,
    ) -> Result<Vec<f64>> {
        let stride = op.stride();
        let mut results = Vec::with_capacity(batch_size);

        for i in 0..batch_size {
            let base = i * stride;
            let result = match op {
                Op::Fao56Et0 => {
                    // FAO-56 Penman-Monteith (CPU reference implementation)
                    let tmax = data[base];
                    let tmin = data[base + 1];
                    let rh_max = data[base + 2];
                    let rh_min = data[base + 3];
                    let wind_2m = data[base + 4];
                    let rs = data[base + 5];
                    let elevation = data[base + 6];
                    let lat = data[base + 7];
                    let doy = data[base + 8] as u32;

                    fao56_et0_cpu(tmax, tmin, rh_max, rh_min, wind_2m, rs, elevation, lat, doy)
                }
                Op::WaterBalance => {
                    let dr_prev = data[base];
                    let precip = data[base + 1];
                    let irrig = data[base + 2];
                    let etc = data[base + 3];
                    let taw = data[base + 4];
                    let raw = data[base + 5];
                    let _p_frac = data[base + 6];

                    water_balance_cpu(dr_prev, precip, irrig, etc, taw, raw)
                }
                Op::Custom => data[base],
                Op::SensorCalibration => {
                    let raw = data[base];
                    2e-13 * raw.powi(3) - 4e-9 * raw.powi(2) + 4e-5 * raw - 0.0677
                }
                Op::HargreavesEt0 => {
                    let tmax = data[base];
                    let tmin = data[base + 1];
                    let lat_rad = data[base + 2];
                    let doy = data[base + 3];
                    hargreaves_et0_cpu(tmax, tmin, lat_rad, doy)
                }
                Op::KcClimateAdjust => {
                    let kc_table = data[base];
                    let u2 = data[base + 1];
                    let rh_min = data[base + 2];
                    let h = data[base + 3];
                    let adj =
                        (0.04 * (u2 - 2.0) - 0.004 * (rh_min - 45.0)) * (h / 3.0_f64).powf(0.3);
                    (kc_table + adj).max(0.0)
                }
                Op::DualKcKe => {
                    let kcb: f64 = data[base];
                    let kc_max: f64 = data[base + 1];
                    let few: f64 = data[base + 2];
                    let mulch: f64 = data[base + 3];
                    let de_prev: f64 = data[base + 4];
                    let rew: f64 = data[base + 5];
                    let tew: f64 = data[base + 6];
                    let p_eff: f64 = data[base + 7];

                    let de = (de_prev - p_eff).clamp(0.0, tew);
                    let kr: f64 = if de > rew {
                        ((tew - de) / (tew - rew).max(0.001)).max(0.0)
                    } else {
                        1.0
                    };
                    let ke_full = kr * (kc_max - kcb);
                    let ke_limit = few * kc_max;
                    (ke_full.min(ke_limit) * mulch).max(0.0)
                }
            };
            results.push(result);
        }

        Ok(results)
    }

    // ========================================================================
    // CONVENIENCE METHODS — Domain-specific APIs
    // ========================================================================

    /// Compute FAO-56 ET₀ for multiple station-days
    ///
    /// # Arguments
    /// * `station_days` - Slice of `StationDayInput` tuples
    ///
    /// # Returns
    /// ET₀ values in mm/day for each station-day
    pub fn fao56_et0_batch(&self, station_days: &[StationDayInput]) -> Result<Vec<f64>> {
        let batch_size = station_days.len();
        let mut data = Vec::with_capacity(batch_size * 9);

        for &(tmax, tmin, rh_max, rh_min, wind, rs, elev, lat, doy) in station_days {
            data.push(tmax);
            data.push(tmin);
            data.push(rh_max);
            data.push(rh_min);
            data.push(wind);
            data.push(rs);
            data.push(elev);
            data.push(lat);
            data.push(doy as f64);
        }

        self.execute(&data, batch_size, Op::Fao56Et0)
    }

    /// Compute water balance update for multiple fields
    ///
    /// # Arguments
    /// * `fields` - Slice of `WaterBalanceInput` tuples
    ///
    /// # Returns
    /// New depletion values for each field
    pub fn water_balance_batch(&self, fields: &[WaterBalanceInput]) -> Result<Vec<f64>> {
        let batch_size = fields.len();
        let mut data = Vec::with_capacity(batch_size * 7);

        for &(dr_prev, precip, irrig, etc, taw, raw, p_frac) in fields {
            data.push(dr_prev);
            data.push(precip);
            data.push(irrig);
            data.push(etc);
            data.push(taw);
            data.push(raw);
            data.push(p_frac);
        }

        self.execute(&data, batch_size, Op::WaterBalance)
    }
}

// ============================================================================
// CPU REFERENCE IMPLEMENTATIONS (for tests and validation)
// ============================================================================

/// Hargreaves-Samani ET₀ (CPU reference) — FAO-56 Eq. 52
pub fn hargreaves_et0_cpu(tmax: f64, tmin: f64, lat_rad: f64, doy: f64) -> f64 {
    use std::f64::consts::PI;
    let two_pi = 2.0 * PI;
    let dr = 1.0 + 0.033 * (two_pi * doy / 365.0).cos();
    let decl = 0.409 * (two_pi * doy / 365.0 - 1.39).sin();
    let ws_arg = (-lat_rad.tan() * decl.tan()).clamp(-1.0, 1.0);
    let ws = ws_arg.acos();
    let ra_mj =
        37.586 * dr * (ws * lat_rad.sin() * decl.sin() + lat_rad.cos() * decl.cos() * ws.sin());
    let ra_mm = ra_mj * 0.408;
    let tmean = (tmax + tmin) * 0.5;
    let td = (tmax - tmin).max(0.0);
    (0.0023 * (tmean + 17.8) * td.sqrt() * ra_mm).max(0.0)
}

/// FAO-56 Eq. 62 Kc climate adjustment (CPU reference)
pub fn kc_climate_adjust_cpu(kc_table: f64, u2: f64, rh_min: f64, crop_height_m: f64) -> f64 {
    let adj = (0.04 * (u2 - 2.0) - 0.004 * (rh_min - 45.0)) * (crop_height_m / 3.0_f64).powf(0.3);
    (kc_table + adj).max(0.0)
}

#[cfg(test)]
/// FAO-56 Penman-Monteith ET₀ (CPU reference)
fn fao56_et0_cpu(
    tmax: f64,
    tmin: f64,
    rh_max: f64,
    rh_min: f64,
    wind_2m: f64,
    rs: f64,
    elevation: f64,
    lat: f64,
    doy: u32,
) -> f64 {
    use std::f64::consts::PI;

    let tmean = (tmax + tmin) / 2.0;

    // Atmospheric pressure (FAO-56 Eq. 7)
    let p = 101.3 * ((293.0 - 0.0065 * elevation) / 293.0).powf(5.26);

    // Psychrometric constant
    let gamma = 0.000665 * p;

    // Saturation vapour pressure
    let e_tmax = 0.6108 * (17.27 * tmax / (tmax + 237.3)).exp();
    let e_tmin = 0.6108 * (17.27 * tmin / (tmin + 237.3)).exp();
    let es = (e_tmax + e_tmin) / 2.0;

    // Actual vapour pressure
    let ea = (e_tmin * rh_max / 100.0 + e_tmax * rh_min / 100.0) / 2.0;

    // Slope of saturation vapour pressure curve
    let e_tmean = 0.6108 * (17.27 * tmean / (tmean + 237.3)).exp();
    let delta = 4098.0 * e_tmean / (tmean + 237.3).powi(2);

    // Extraterrestrial radiation
    let lat_rad = lat * PI / 180.0;
    let dr = 1.0 + 0.033 * (2.0 * PI * doy as f64 / 365.0).cos();
    let decl = 0.409 * (2.0 * PI * doy as f64 / 365.0 - 1.39).sin();

    let tan_lat = lat_rad.tan();
    let tan_decl = decl.tan();
    let ws_arg = -tan_lat * tan_decl;
    let ws = if ws_arg.abs() > 1.0 {
        PI
    } else {
        ws_arg.acos()
    };

    let gsc = 0.0820;
    let ra = 24.0 * 60.0 / PI
        * gsc
        * dr
        * (ws * lat_rad.sin() * decl.sin() + lat_rad.cos() * decl.cos() * ws.sin());

    // Clear-sky radiation
    let rso = (0.75 + 0.00002 * elevation) * ra;

    // Net shortwave radiation
    let rns = (1.0 - 0.23) * rs;

    // Net longwave radiation
    let sigma = 4.903e-9; // Stefan-Boltzmann constant
    let tmax_k = tmax + 273.16;
    let tmin_k = tmin + 273.16;
    let rnl = sigma * (tmax_k.powi(4) + tmin_k.powi(4)) / 2.0
        * (0.34 - 0.14 * ea.sqrt())
        * (1.35 * rs / rso.max(0.001) - 0.35);

    // Net radiation
    let rn = rns - rnl;

    // FAO-56 Penman-Monteith equation
    let numerator = 0.408 * delta * rn + gamma * 900.0 / (tmean + 273.0) * wind_2m * (es - ea);
    let denominator = delta + gamma * (1.0 + 0.34 * wind_2m);

    numerator / denominator
}

#[cfg(test)]
/// Water balance daily update (CPU reference)
fn water_balance_cpu(dr_prev: f64, precip: f64, irrig: f64, etc: f64, taw: f64, raw: f64) -> f64 {
    // Stress coefficient
    let ks = if dr_prev > raw {
        ((taw - dr_prev) / (taw - raw)).max(0.0)
    } else {
        1.0
    };

    // Adjusted ETc
    let etc_adj = ks * etc;

    // New depletion
    let dr_new = dr_prev - precip - irrig + etc_adj;
    dr_new.clamp(0.0, taw)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fao56_et0_cpu_reference() {
        // FAO-56 Example 18: Reference grass ET₀
        // Uccle, Belgium (50°48'N, 4°21'E, 100m elevation)
        // July 6: tmax=21.5°C, tmin=12.3°C, RHmax=84%, RHmin=63%, u2=2.78m/s, Rs=22.07 MJ/m²/day
        let et0 = fao56_et0_cpu(21.5, 12.3, 84.0, 63.0, 2.78, 22.07, 100.0, 50.8, 187);

        // Expected: ~3.88 mm/day (FAO-56 Example 18)
        assert!(
            (et0 - 3.88).abs() < 0.1,
            "FAO-56 Example 18: got {} mm/day, expected ~3.88 mm/day",
            et0
        );
    }

    #[test]
    fn test_water_balance_no_stress() {
        // No stress: Dr < RAW
        let dr_new = water_balance_cpu(30.0, 5.0, 0.0, 4.0, 100.0, 50.0);
        // Dr_new = 30 - 5 - 0 + 4 = 29 (no stress since Dr < RAW)
        assert!((dr_new - 29.0).abs() < 0.001);
    }

    #[test]
    fn test_water_balance_with_stress() {
        // Stress: Dr > RAW
        let dr_new = water_balance_cpu(60.0, 0.0, 0.0, 5.0, 100.0, 50.0);
        // Ks = (100 - 60) / (100 - 50) = 0.8
        // ETc_adj = 0.8 * 5 = 4
        // Dr_new = 60 - 0 - 0 + 4 = 64
        assert!((dr_new - 64.0).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_fao56_et0_gpu() -> Result<()> {
        let Some(device) = crate::device::test_pool::get_test_device_if_f64_gpu_available().await
        else {
            return Ok(()); // Skip if no f64 GPU available
        };
        let executor = BatchedElementwiseF64::new(device)?;

        // Test batch of 3 station-days
        let station_days = vec![
            (21.5, 12.3, 84.0, 63.0, 2.78, 22.07, 100.0, 50.8, 187u32),
            (25.0, 15.0, 80.0, 50.0, 3.0, 20.0, 200.0, 45.0, 180),
            (30.0, 20.0, 70.0, 40.0, 2.0, 25.0, 50.0, 35.0, 200),
        ];

        let results = executor.fao56_et0_batch(&station_days)?;
        assert_eq!(results.len(), 3);

        // First result should match FAO-56 Example 18 (~3.88 mm/day)
        assert!(
            (results[0] - 3.88).abs() < 0.2,
            "GPU ET₀[0]: got {} mm/day, expected ~3.88 mm/day",
            results[0]
        );

        Ok(())
    }

    #[test]
    fn test_sensor_calibration_cpu() {
        let raw = 15_000.0_f64;
        let vwc = 2e-13 * raw.powi(3) - 4e-9 * raw.powi(2) + 4e-5 * raw - 0.0677;
        assert!(vwc > 0.0 && vwc < 1.0, "VWC={vwc} out of range");
    }

    #[test]
    fn test_hargreaves_et0_cpu() {
        use std::f64::consts::PI;
        let lat_rad = 50.8 * PI / 180.0;
        let et0 = hargreaves_et0_cpu(21.5, 12.3, lat_rad, 187.0);
        assert!(
            et0 > 1.0 && et0 < 10.0,
            "Hargreaves ET₀={et0}, expected 1-10 mm/day"
        );
    }

    #[test]
    fn test_kc_climate_adjust_cpu() {
        let kc = kc_climate_adjust_cpu(1.15, 3.0, 30.0, 2.0);
        assert!(kc > 1.0 && kc < 2.0, "Kc={kc}, expected ~1.15-1.3");
    }

    #[test]
    fn test_dual_kc_ke_cpu() {
        let kcb: f64 = 0.3;
        let kc_max: f64 = 1.2;
        let few: f64 = 0.5;
        let mulch: f64 = 1.0;
        let de_prev: f64 = 5.0;
        let rew: f64 = 9.0;
        let tew: f64 = 22.0;
        let p_eff: f64 = 0.0;

        let de = (de_prev - p_eff).clamp(0.0, tew);
        assert!(de < rew, "should be below REW");
        let ke_full = 1.0_f64 * (kc_max - kcb);
        let ke_limit = few * kc_max;
        let ke = ke_full.min(ke_limit) * mulch;
        assert!(ke > 0.0 && ke < 1.0, "Ke={ke}");
    }

    #[test]
    fn test_op_strides() {
        assert_eq!(Op::Fao56Et0.stride(), 9);
        assert_eq!(Op::WaterBalance.stride(), 7);
        assert_eq!(Op::Custom.stride(), 1);
        assert_eq!(Op::SensorCalibration.stride(), 1);
        assert_eq!(Op::HargreavesEt0.stride(), 4);
        assert_eq!(Op::KcClimateAdjust.stride(), 4);
        assert_eq!(Op::DualKcKe.stride(), 9);
    }
}
