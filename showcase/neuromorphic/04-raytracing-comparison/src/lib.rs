//! NPU vs GPU Raytracing Comparison
//!
//! Deep Debt: Uses ToadStool for hardware discovery

pub mod benchmark;
pub mod gpu_raytracer;
pub mod npu_raytracer;
pub mod scene;

pub use benchmark::Benchmark;
pub use gpu_raytracer::GpuRaytracer;
pub use npu_raytracer::NpuRaytracer;
pub use scene::{Ray, Scene, Sphere};
