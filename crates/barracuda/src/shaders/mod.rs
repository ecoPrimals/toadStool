//! WGSL Shader Infrastructure
//!
//! This module provides:
//! - **Precision-generic shader templates**: ONE source generates f16/f32/f64 shaders
//! - **CPU implementations**: Same algorithms via `num-traits` for CPU fallback
//!
//! # Design Philosophy
//!
//! Same math runs on CPU and GPU:
//! - GPU: WGSL shaders (generated from templates)
//! - CPU: Rust implementations (via num-traits)
//!
//! # Usage
//!
//! ```rust,ignore
//! use barracuda::shaders::precision::{Precision, ShaderTemplate};
//!
//! // Generate f64 shader at runtime
//! let f64_add = ShaderTemplate::elementwise_add(Precision::F64);
//!
//! // CPU equivalent (same algorithm)
//! use barracuda::shaders::precision::cpu;
//! let mut out = vec![0.0f64; 3];
//! cpu::elementwise_add(&[1.0, 2.0, 3.0], &[4.0, 5.0, 6.0], &mut out);
//! ```

pub mod precision;

pub use precision::{Precision, ShaderTemplate};
