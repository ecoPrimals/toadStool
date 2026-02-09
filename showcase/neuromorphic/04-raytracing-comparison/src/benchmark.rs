//! Benchmark NPU vs GPU raytracing performance

use crate::gpu_raytracer::GpuRaytracer;
use crate::npu_raytracer::NpuRaytracer;
use crate::scene::Scene;
use anyhow::Result;
use std::time::Instant;

pub struct Benchmark {
    scene: Scene,
}

impl Benchmark {
    pub fn new(scene: Scene) -> Self {
        Self { scene }
    }

    /// Benchmark NPU rendering
    pub fn benchmark_npu(&self, device_id: &str) -> Result<BenchmarkResult> {
        let mut raytracer = NpuRaytracer::new(self.scene.clone(), device_id)?;

        let start = Instant::now();
        let pixels = raytracer.render()?;
        let duration = start.elapsed();

        Ok(BenchmarkResult {
            device: "NPU (Akida)".to_string(),
            duration_ms: duration.as_secs_f64() * 1000.0,
            pixels: pixels.len(),
            scene_type: format!("{} spheres", self.scene.spheres.len()),
        })
    }

    /// Benchmark GPU rendering
    pub fn benchmark_gpu(&self) -> Result<BenchmarkResult> {
        let raytracer = GpuRaytracer::new(self.scene.clone())?;

        let start = Instant::now();
        let pixels = raytracer.render()?;
        let duration = start.elapsed();

        Ok(BenchmarkResult {
            device: "GPU (WGPU)".to_string(),
            duration_ms: duration.as_secs_f64() * 1000.0,
            pixels: pixels.len(),
            scene_type: format!("{} spheres", self.scene.spheres.len()),
        })
    }
}

#[derive(Debug)]
pub struct BenchmarkResult {
    pub device: String,
    pub duration_ms: f64,
    pub pixels: usize,
    pub scene_type: String,
}

impl BenchmarkResult {
    pub fn fps(&self) -> f64 {
        1000.0 / self.duration_ms
    }

    pub fn rays_per_second(&self) -> f64 {
        (self.pixels as f64 / self.duration_ms) * 1000.0
    }
}
