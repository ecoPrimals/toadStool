//! Cubic Spline Interpolation
//!
//! Natural and clamped cubic spline interpolation for smooth function approximation.
//!
//! # Algorithm
//!
//! Given n data points (x_i, y_i), construct n-1 cubic polynomials S_i(x) such that:
//! 1. S_i(x_i) = y_i (passes through data points)
//! 2. S_i(x_{i+1}) = y_{i+1} (continuous)
//! 3. S'_i(x_{i+1}) = S'_{i+1}(x_{i+1}) (C¹ continuous)
//! 4. S''_i(x_{i+1}) = S''_{i+1}(x_{i+1}) (C² continuous)
//!
//! # Boundary Conditions
//!
//! - **Natural**: S''(x_0) = S''(x_n) = 0
//! - **Clamped**: S'(x_0) = f'_0, S'(x_n) = f'_n (specified derivatives)
//! - **Not-a-knot**: Third derivative continuous at x_1 and x_{n-1}
//!
//! # Applications
//!
//! - Smooth curve fitting through data points
//! - Surrogate model interpolation
//! - Animation and path planning
//! - Function approximation
//!
//! # References
//!
//! - Numerical Recipes, 3rd Edition, Section 3.3
//! - De Boor, "A Practical Guide to Splines"

use crate::error::{BarracudaError, Result};

const WGSL_CUBIC_SPLINE_EVAL_F64: &str = include_str!("../shaders/math/cubic_spline_eval_f64.wgsl");

/// Cubic spline interpolator
#[derive(Debug, Clone)]
pub struct CubicSpline {
    /// x coordinates of data points (sorted)
    x: Vec<f64>,
    /// y coordinates of data points
    y: Vec<f64>,
    /// Second derivatives at each point
    y2: Vec<f64>,
}

/// Boundary condition for cubic spline
#[derive(Debug, Clone, Copy, Default)]
pub enum SplineBoundary {
    /// Natural spline: S''(x) = 0 at endpoints
    #[default]
    Natural,
    /// Clamped spline: S'(x) = value at endpoints
    Clamped { left: f64, right: f64 },
    /// Not-a-knot: third derivative continuous at second and second-to-last points
    NotAKnot,
}

impl CubicSpline {
    /// Create a natural cubic spline (S'' = 0 at endpoints)
    ///
    /// # Arguments
    ///
    /// * `x` - x coordinates (must be strictly increasing)
    /// * `y` - y coordinates
    ///
    /// # Example
    ///
    /// ```
    /// use barracuda::interpolate::CubicSpline;
    ///
    /// let x = vec![0.0, 1.0, 2.0, 3.0];
    /// let y = vec![0.0, 1.0, 0.0, 1.0];
    ///
    /// let spline = CubicSpline::natural(&x, &y)?;
    /// let y_mid = spline.eval(1.5)?;
    /// # Ok::<(), barracuda::error::BarracudaError>(())
    /// ```
    pub fn natural(x: &[f64], y: &[f64]) -> Result<Self> {
        Self::new(x, y, SplineBoundary::Natural)
    }

    /// Create a clamped cubic spline (specified first derivatives at endpoints)
    ///
    /// # Arguments
    ///
    /// * `x` - x coordinates (must be strictly increasing)
    /// * `y` - y coordinates
    /// * `dy_left` - First derivative at left endpoint
    /// * `dy_right` - First derivative at right endpoint
    pub fn clamped(x: &[f64], y: &[f64], dy_left: f64, dy_right: f64) -> Result<Self> {
        Self::new(
            x,
            y,
            SplineBoundary::Clamped {
                left: dy_left,
                right: dy_right,
            },
        )
    }

    /// Create a cubic spline with specified boundary conditions
    ///
    /// # Arguments
    ///
    /// * `x` - x coordinates (must be strictly increasing)
    /// * `y` - y coordinates
    /// * `boundary` - Boundary condition type
    pub fn new(x: &[f64], y: &[f64], boundary: SplineBoundary) -> Result<Self> {
        let n = x.len();

        if n < 2 {
            return Err(BarracudaError::InvalidInput {
                message: "CubicSpline requires at least 2 data points".to_string(),
            });
        }

        if n != y.len() {
            return Err(BarracudaError::InvalidInput {
                message: format!("x and y must have same length: {} vs {}", n, y.len()),
            });
        }

        // Check that x is strictly increasing
        for i in 1..n {
            if x[i] <= x[i - 1] {
                return Err(BarracudaError::InvalidInput {
                    message: format!(
                        "x must be strictly increasing: x[{}]={} >= x[{}]={}",
                        i,
                        x[i],
                        i - 1,
                        x[i - 1]
                    ),
                });
            }
        }

        let x = x.to_vec();
        let y = y.to_vec();

        // Compute second derivatives by solving tridiagonal system
        let y2 = compute_second_derivatives(&x, &y, boundary)?;

        Ok(Self { x, y, y2 })
    }

    /// Evaluate the spline at a single point
    ///
    /// # Arguments
    ///
    /// * `x_eval` - Point at which to evaluate
    ///
    /// # Returns
    ///
    /// Interpolated value
    pub fn eval(&self, x_eval: f64) -> Result<f64> {
        // Find the interval containing x_eval
        let i = self.find_interval(x_eval)?;

        // Compute the cubic spline value
        let h = self.x[i + 1] - self.x[i];
        let a = (self.x[i + 1] - x_eval) / h;
        let b = (x_eval - self.x[i]) / h;

        let result = a * self.y[i]
            + b * self.y[i + 1]
            + ((a * a * a - a) * self.y2[i] + (b * b * b - b) * self.y2[i + 1]) * h * h / 6.0;

        Ok(result)
    }

    /// Evaluate the spline at multiple points (CPU path).
    pub fn eval_many(&self, x_eval: &[f64]) -> Result<Vec<f64>> {
        x_eval.iter().map(|&x| self.eval(x)).collect()
    }

    /// Evaluate the spline at multiple points on GPU.
    ///
    /// Converts `(y, y2)` representation to `[a, b, c, d]` monomial coefficients
    /// per segment, then dispatches `cubic_spline_eval_f64.wgsl`.
    pub fn eval_many_gpu(
        &self,
        x_eval: &[f64],
        device: &crate::device::WgpuDevice,
    ) -> Result<Vec<f64>> {
        use bytemuck::{Pod, Zeroable};
        use wgpu::util::DeviceExt;

        #[repr(C)]
        #[derive(Copy, Clone, Pod, Zeroable)]
        struct SplineParams {
            n_query: u32,
            n_segments: u32,
        }

        let n_seg = self.x.len() - 1;
        let n_query = x_eval.len();

        // Convert (y, y2) to monomial [a, b, c, d] per segment
        let mut coefs = Vec::with_capacity(n_seg * 4);
        for i in 0..n_seg {
            let h = self.x[i + 1] - self.x[i];
            let a = self.y[i];
            let b = (self.y[i + 1] - self.y[i]) / h - (2.0 * self.y2[i] + self.y2[i + 1]) * h / 6.0;
            let c = self.y2[i] / 2.0;
            let d_coef = (self.y2[i + 1] - self.y2[i]) / (6.0 * h);
            coefs.extend_from_slice(&[a, b, c, d_coef]);
        }

        let module = device.compile_shader_f64(WGSL_CUBIC_SPLINE_EVAL_F64, Some("spline_eval_f64"));
        let d = &device.device;
        let q = &device.queue;

        let query_buf = d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Spline:query"),
            contents: bytemuck::cast_slice(x_eval),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let knots_buf = d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Spline:knots"),
            contents: bytemuck::cast_slice(&self.x),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let coefs_buf = d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Spline:coefs"),
            contents: bytemuck::cast_slice(&coefs),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let out_size = (n_query * 8) as u64;
        let result_buf = d.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Spline:result"),
            size: out_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let params = SplineParams {
            n_query: n_query as u32,
            n_segments: n_seg as u32,
        };
        let params_buf = d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Spline:params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let bgl = d.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Spline:bgl"),
            entries: &[
                bgl_storage(0, true),
                bgl_storage(1, true),
                bgl_storage(2, true),
                bgl_storage(3, false),
                bgl_uniform(4),
            ],
        });
        let bg = d.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Spline:bg"),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: query_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: knots_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: coefs_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: result_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: params_buf.as_entire_binding(),
                },
            ],
        });

        let pl = d.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Spline:pl"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });
        let pipeline = d.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Spline:pipeline"),
            layout: Some(&pl),
            module: &module,
            entry_point: "main",
            compilation_options: Default::default(),
            cache: None,
        });

        let mut enc = d.create_command_encoder(&Default::default());
        {
            let mut pass = enc.begin_compute_pass(&Default::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bg, &[]);
            pass.dispatch_workgroups((n_query as u32).div_ceil(256), 1, 1);
        }

        let rb = d.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Spline:rb"),
            size: out_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        enc.copy_buffer_to_buffer(&result_buf, 0, &rb, 0, out_size);
        q.submit(Some(enc.finish()));

        let slice = rb.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            tx.send(r).ok();
        });
        d.poll(wgpu::Maintain::Wait);
        rx.recv()
            .map_err(|_| BarracudaError::Gpu("spline readback".into()))?
            .map_err(|e| BarracudaError::Gpu(format!("spline map: {e}")))?;

        let data = slice.get_mapped_range();
        let result: Vec<f64> = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        rb.unmap();

        Ok(result)
    }

    /// Evaluate the first derivative at a point
    pub fn derivative(&self, x_eval: f64) -> Result<f64> {
        let i = self.find_interval(x_eval)?;

        let h = self.x[i + 1] - self.x[i];
        let a = (self.x[i + 1] - x_eval) / h;
        let b = (x_eval - self.x[i]) / h;

        let dy = (self.y[i + 1] - self.y[i]) / h - (3.0 * a * a - 1.0) * h * self.y2[i] / 6.0
            + (3.0 * b * b - 1.0) * h * self.y2[i + 1] / 6.0;

        Ok(dy)
    }

    /// Evaluate the second derivative at a point
    pub fn second_derivative(&self, x_eval: f64) -> Result<f64> {
        let i = self.find_interval(x_eval)?;

        let h = self.x[i + 1] - self.x[i];
        let a = (self.x[i + 1] - x_eval) / h;
        let b = (x_eval - self.x[i]) / h;

        let d2y = a * self.y2[i] + b * self.y2[i + 1];

        Ok(d2y)
    }

    /// Compute the definite integral of the spline from a to b
    pub fn integrate(&self, a: f64, b: f64) -> Result<f64> {
        if a > b {
            return Ok(-self.integrate(b, a)?);
        }

        let i_a = self.find_interval(a)?;
        let i_b = self.find_interval(b)?;

        let mut total = 0.0;

        for i in i_a..=i_b {
            let x0 = if i == i_a { a } else { self.x[i] };
            let x1 = if i == i_b { b } else { self.x[i + 1] };

            total += integrate_segment(&self.x, &self.y, &self.y2, i, x0, x1);
        }

        Ok(total)
    }

    /// Get the x coordinates of the data points
    pub fn x_data(&self) -> &[f64] {
        &self.x
    }

    /// Get the y coordinates of the data points
    pub fn y_data(&self) -> &[f64] {
        &self.y
    }

    /// Get the second derivatives at the data points
    pub fn second_derivatives(&self) -> &[f64] {
        &self.y2
    }

    /// Find the interval [x[i], x[i+1]] containing x_eval
    fn find_interval(&self, x_eval: f64) -> Result<usize> {
        let n = self.x.len();

        // Handle boundary cases with extrapolation
        if x_eval < self.x[0] {
            return Ok(0);
        }
        if x_eval >= self.x[n - 1] {
            return Ok(n - 2);
        }

        // Binary search for the interval
        let mut lo = 0;
        let mut hi = n - 1;

        while hi - lo > 1 {
            let mid = (lo + hi) / 2;
            if self.x[mid] > x_eval {
                hi = mid;
            } else {
                lo = mid;
            }
        }

        Ok(lo)
    }
}

/// Compute second derivatives for natural or clamped spline
fn compute_second_derivatives(x: &[f64], y: &[f64], boundary: SplineBoundary) -> Result<Vec<f64>> {
    let n = x.len();

    if n == 2 {
        // For 2 points, second derivatives are 0 (linear interpolation)
        return Ok(vec![0.0; 2]);
    }

    // Set up tridiagonal system
    let mut lower = vec![0.0; n];
    let mut diag = vec![0.0; n];
    let mut upper = vec![0.0; n];
    let mut rhs = vec![0.0; n];

    // Interior points
    for i in 1..n - 1 {
        let h_prev = x[i] - x[i - 1];
        let h_next = x[i + 1] - x[i];

        lower[i] = h_prev;
        diag[i] = 2.0 * (h_prev + h_next);
        upper[i] = h_next;
        rhs[i] = 6.0 * ((y[i + 1] - y[i]) / h_next - (y[i] - y[i - 1]) / h_prev);
    }

    // Boundary conditions
    match boundary {
        SplineBoundary::Natural => {
            // S''(x_0) = S''(x_n) = 0
            diag[0] = 1.0;
            upper[0] = 0.0;
            rhs[0] = 0.0;

            diag[n - 1] = 1.0;
            lower[n - 1] = 0.0;
            rhs[n - 1] = 0.0;
        }
        SplineBoundary::Clamped { left, right } => {
            let h0 = x[1] - x[0];
            let hn = x[n - 1] - x[n - 2];

            diag[0] = 2.0 * h0;
            upper[0] = h0;
            rhs[0] = 6.0 * ((y[1] - y[0]) / h0 - left);

            diag[n - 1] = 2.0 * hn;
            lower[n - 1] = hn;
            rhs[n - 1] = 6.0 * (right - (y[n - 1] - y[n - 2]) / hn);
        }
        SplineBoundary::NotAKnot => {
            // Third derivative continuous at x[1] and x[n-2]
            if n < 4 {
                // Fall back to natural for small n
                return compute_second_derivatives(x, y, SplineBoundary::Natural);
            }

            let h0 = x[1] - x[0];
            let h1 = x[2] - x[1];
            diag[0] = h1;
            upper[0] = -(h0 + h1);
            rhs[0] = 0.0;
            // Need to handle the first row specially
            // S'''_0(x_1) = S'''_1(x_1) means:
            // (y2[1] - y2[0])/h0 = (y2[2] - y2[1])/h1
            // h1*y2[0] - (h0+h1)*y2[1] + h0*y2[2] = 0

            let hm1 = x[n - 2] - x[n - 3];
            let hm0 = x[n - 1] - x[n - 2];
            diag[n - 1] = hm1;
            lower[n - 1] = -(hm0 + hm1);
            rhs[n - 1] = 0.0;
        }
    }

    // Solve the tridiagonal system
    // Note: tridiagonal_solve expects the system in a specific format
    // Let's use our own Thomas algorithm for this
    solve_tridiagonal(&lower, &diag, &upper, &rhs)
}

/// Thomas algorithm for tridiagonal system
fn solve_tridiagonal(lower: &[f64], diag: &[f64], upper: &[f64], rhs: &[f64]) -> Result<Vec<f64>> {
    let n = diag.len();
    let mut c_prime = vec![0.0; n];
    let mut d_prime = vec![0.0; n];

    // Forward sweep
    c_prime[0] = upper[0] / diag[0];
    d_prime[0] = rhs[0] / diag[0];

    for i in 1..n {
        let denom = diag[i] - lower[i] * c_prime[i - 1];
        if denom.abs() < 1e-14 {
            return Err(BarracudaError::ExecutionError {
                message: "Singular tridiagonal system in spline computation".to_string(),
            });
        }
        c_prime[i] = upper[i] / denom;
        d_prime[i] = (rhs[i] - lower[i] * d_prime[i - 1]) / denom;
    }

    // Back substitution
    let mut x = vec![0.0; n];
    x[n - 1] = d_prime[n - 1];

    for i in (0..n - 1).rev() {
        x[i] = d_prime[i] - c_prime[i] * x[i + 1];
    }

    Ok(x)
}

/// Integrate a single spline segment from x0 to x1
fn integrate_segment(x: &[f64], y: &[f64], y2: &[f64], i: usize, x0: f64, x1: f64) -> f64 {
    let h = x[i + 1] - x[i];

    // Transform to [0, 1] interval for cleaner integration
    let t0 = (x0 - x[i]) / h;
    let t1 = (x1 - x[i]) / h;

    // Cubic spline in terms of t = (x - x[i]) / h:
    // S(t) = (1-t)*y[i] + t*y[i+1] + h²/6 * ((1-t)³ - (1-t))*y2[i] + h²/6 * (t³ - t)*y2[i+1]
    //
    // Integrate from t0 to t1:
    let linear_part = y[i] * (t1 - t0) + 0.5 * (y[i + 1] - y[i]) * (t1 * t1 - t0 * t0);

    // Integral of (1-t)³ - (1-t) = -(1-t)⁴/4 + (1-t)²/2
    let term1 = |t: f64| -(1.0 - t).powi(4) / 4.0 + (1.0 - t).powi(2) / 2.0;
    let cubic_left = (term1(t1) - term1(t0)) * h * h * y2[i] / 6.0;

    // Integral of t³ - t = t⁴/4 - t²/2
    let term2 = |t: f64| t.powi(4) / 4.0 - t.powi(2) / 2.0;
    let cubic_right = (term2(t1) - term2(t0)) * h * h * y2[i + 1] / 6.0;

    h * (linear_part + cubic_left + cubic_right)
}

fn bgl_storage(binding: u32, read_only: bool) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn bgl_uniform(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    #[test]
    fn test_natural_spline_passes_through_points() {
        let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
        let y = vec![0.0, 1.0, 0.0, 1.0, 0.0];

        let spline = CubicSpline::natural(&x, &y).unwrap();

        for (xi, yi) in x.iter().zip(y.iter()) {
            let y_eval = spline.eval(*xi).unwrap();
            assert!(
                (y_eval - yi).abs() < 1e-10,
                "Spline should pass through data points"
            );
        }
    }

    #[test]
    fn test_linear_data_gives_linear_spline() {
        let x = vec![0.0, 1.0, 2.0, 3.0];
        let y = vec![0.0, 1.0, 2.0, 3.0];

        let spline = CubicSpline::natural(&x, &y).unwrap();

        // For linear data, natural spline should be linear
        for &xi in &[0.5, 1.5, 2.5] {
            let y_eval = spline.eval(xi).unwrap();
            assert!(
                (y_eval - xi).abs() < 1e-10,
                "Linear data should give linear interpolation"
            );
        }

        // Second derivatives should be zero
        for y2i in spline.second_derivatives() {
            assert!(
                y2i.abs() < 1e-10,
                "Second derivatives should be zero for linear data"
            );
        }
    }

    #[test]
    fn test_spline_continuity() {
        let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
        let y = vec![0.0, 1.0, 0.5, 1.5, 1.0];

        let spline = CubicSpline::natural(&x, &y).unwrap();

        // Check C² continuity at interior knots
        for i in 1..x.len() - 1 {
            let xi = x[i];
            let eps = 1e-6;

            let y_left = spline.eval(xi - eps).unwrap();
            let y_right = spline.eval(xi + eps).unwrap();
            let y_mid = spline.eval(xi).unwrap();

            // Value continuity
            assert!(
                (y_left - y_mid).abs() < 1e-4 && (y_right - y_mid).abs() < 1e-4,
                "Spline should be continuous"
            );

            // First derivative continuity
            let dy_left = spline.derivative(xi - eps).unwrap();
            let dy_right = spline.derivative(xi + eps).unwrap();
            assert!(
                (dy_left - dy_right).abs() < 1e-3,
                "First derivative should be continuous"
            );
        }
    }

    #[test]
    fn test_clamped_spline_derivatives() {
        let x = vec![0.0, 1.0, 2.0];
        let y = vec![0.0, 1.0, 0.0];

        let dy_left = 2.0;
        let dy_right = -2.0;

        let spline = CubicSpline::clamped(&x, &y, dy_left, dy_right).unwrap();

        // Check derivatives at endpoints
        let dy0 = spline.derivative(x[0]).unwrap();
        let dy_n = spline.derivative(x[x.len() - 1]).unwrap();

        assert!(
            (dy0 - dy_left).abs() < 1e-6,
            "Left derivative should match: {} vs {}",
            dy0,
            dy_left
        );
        assert!(
            (dy_n - dy_right).abs() < 1e-6,
            "Right derivative should match: {} vs {}",
            dy_n,
            dy_right
        );
    }

    #[test]
    fn test_sine_interpolation() {
        // Interpolate sin(x) on [0, 2π]
        let n = 10;
        let x: Vec<f64> = (0..=n).map(|i| 2.0 * PI * i as f64 / n as f64).collect();
        let y: Vec<f64> = x.iter().map(|&xi| xi.sin()).collect();

        let spline = CubicSpline::natural(&x, &y).unwrap();

        // Test at intermediate points
        for i in 0..100 {
            let xi = 2.0 * PI * i as f64 / 100.0;
            let y_spline = spline.eval(xi).unwrap();
            let y_exact = xi.sin();

            assert!(
                (y_spline - y_exact).abs() < 0.01,
                "Spline should approximate sin(x) well: {} vs {} at x={}",
                y_spline,
                y_exact,
                xi
            );
        }
    }

    #[test]
    fn test_integration() {
        // Integrate linear function y = x from 0 to 2
        let x = vec![0.0, 1.0, 2.0];
        let y = vec![0.0, 1.0, 2.0];

        let spline = CubicSpline::natural(&x, &y).unwrap();

        let integral = spline.integrate(0.0, 2.0).unwrap();
        let expected = 2.0; // ∫₀² x dx = x²/2 |₀² = 2

        assert!(
            (integral - expected).abs() < 1e-10,
            "Integral should be 2: got {}",
            integral
        );
    }

    #[test]
    fn test_invalid_input_too_few_points() {
        let x = vec![0.0];
        let y = vec![0.0];

        assert!(CubicSpline::natural(&x, &y).is_err());
    }

    #[test]
    fn test_invalid_input_non_increasing_x() {
        let x = vec![0.0, 2.0, 1.0, 3.0];
        let y = vec![0.0, 1.0, 0.0, 1.0];

        assert!(CubicSpline::natural(&x, &y).is_err());
    }

    #[test]
    fn test_invalid_input_length_mismatch() {
        let x = vec![0.0, 1.0, 2.0];
        let y = vec![0.0, 1.0];

        assert!(CubicSpline::natural(&x, &y).is_err());
    }

    #[test]
    fn test_extrapolation() {
        let x = vec![0.0, 1.0, 2.0];
        let y = vec![0.0, 1.0, 0.0];

        let spline = CubicSpline::natural(&x, &y).unwrap();

        // Extrapolation should work (uses end segment)
        let y_left = spline.eval(-0.5);
        let y_right = spline.eval(2.5);

        assert!(y_left.is_ok());
        assert!(y_right.is_ok());
    }

    #[test]
    fn test_two_points() {
        // Edge case: exactly 2 points (linear interpolation)
        let x = vec![0.0, 1.0];
        let y = vec![0.0, 2.0];

        let spline = CubicSpline::natural(&x, &y).unwrap();

        assert!((spline.eval(0.5).unwrap() - 1.0).abs() < 1e-10);
        assert!((spline.derivative(0.5).unwrap() - 2.0).abs() < 1e-10);
    }
}
