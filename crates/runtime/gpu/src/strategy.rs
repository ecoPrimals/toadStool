// SPDX-License-Identifier: AGPL-3.0-only
//! Backend Selection Strategy - Pragmatic now, Sovereign tomorrow
//!
//! This module implements intelligent GPU backend selection that prioritizes
//! sovereignty (`WebGPU`) while pragmatically supporting vendor backends (CUDA)
//! when the ecosystem requires them.
//!
//! ## Philosophy
//! - **Default**: Pure Rust `WebGPU` (vendor-agnostic, sovereign)
//! - **Pragmatic**: CUDA when Python AI needs it (2025)
//! - **Evolution**: Track ecosystem maturity, migrate to `WebGPU` when ready
//!
//! ## Selection Priority
//! 1. `WebGPU` (pure Rust, universal) ✅ Always prefer
//! 2. CUDA (vendor-specific) ⚠️ Python AI compatibility (interim; migrate when `WebGPU` covers stacks)
//! 3. `OpenCL` (legacy) ⚠️ Fallback only
//! 4. CPU Compute (always available) ✅ Safe fallback

use tracing::{info, warn};

use crate::types::GpuFramework;
use toadstool::WorkloadType;

/// Backend selection strategy for GPU compute
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum BackendSelectionStrategy {
    /// Automatic: Intelligent selection based on workload and availability
    /// Prioritizes sovereign backends (`WebGPU`), uses vendor when needed
    #[default]
    Automatic,

    /// Sovereign only: Pure Rust `WebGPU` only (no vendor backends)
    /// Fails if `WebGPU` is not available
    SovereignOnly,

    /// Pragmatic: Prefer vendor backends for maximum performance
    /// Uses CUDA/`OpenCL` when available, falls back to `WebGPU`
    Pragmatic,

    /// Specific: Use a specific framework (for testing/debugging)
    Specific(GpuFramework),
}

impl BackendSelectionStrategy {
    /// Select best framework based on strategy, workload, and availability
    pub fn select_framework(
        &self,
        workload: Option<&WorkloadType>,
        available: &[GpuFramework],
    ) -> Option<GpuFramework> {
        match self {
            Self::Automatic => Self::select_automatic(workload, available),
            Self::SovereignOnly => Self::select_sovereign(available),
            Self::Pragmatic => Self::select_pragmatic(workload, available),
            Self::Specific(framework) => {
                if available.contains(framework) {
                    Some(framework.clone())
                } else {
                    warn!(
                        "Requested framework {:?} not available, falling back to automatic selection",
                        framework
                    );
                    Self::select_automatic(workload, available)
                }
            }
        }
    }

    /// Automatic selection: Intelligent based on workload
    fn select_automatic(
        workload: Option<&WorkloadType>,
        available: &[GpuFramework],
    ) -> Option<GpuFramework> {
        // PRIORITY 1: WebGPU (pure Rust, sovereign)
        if available.contains(&GpuFramework::WebGpu) {
            info!("✅ Selected WebGPU (pure Rust, vendor-agnostic)");
            info!("   Evolution status: Sovereign backend active! 🍄");
            return Some(GpuFramework::WebGpu);
        }

        // PRIORITY 2: Check if workload needs vendor-specific backend
        if let Some(workload_type) = workload
            && Self::workload_needs_cuda(workload_type)
            && available.contains(&GpuFramework::Cuda)
        {
            info!("⚠️  Selected CUDA (vendor-specific, interim choice for Python AI stacks)");
            info!("   Evolution status: Using CUDA for Python AI compatibility");
            info!("   Future: Will migrate to WebGPU when ecosystem ready");
            return Some(GpuFramework::Cuda);
        }

        // PRIORITY 3: CUDA as general fallback (if available)
        if available.contains(&GpuFramework::Cuda) {
            info!("⚠️  Selected CUDA (vendor-specific fallback)");
            info!("   Evolution status: Waiting for WebGPU availability");
            return Some(GpuFramework::Cuda);
        }

        // PRIORITY 4: OpenCL (legacy fallback)
        if available.contains(&GpuFramework::OpenCl) {
            info!("⚠️  Selected OpenCL (legacy fallback)");
            info!("   Evolution status: Using OpenCL for compatibility");
            return Some(GpuFramework::OpenCl);
        }

        // PRIORITY 5: Metal (Apple platforms)
        if available.contains(&GpuFramework::Metal) {
            info!("✅ Selected Metal (Apple platform, native)");
            return Some(GpuFramework::Metal);
        }

        // PRIORITY 6: Vulkan
        if available.contains(&GpuFramework::Vulkan) {
            info!("✅ Selected Vulkan (cross-platform compute)");
            return Some(GpuFramework::Vulkan);
        }

        warn!("⚠️  No GPU backend available, will use CPU compute");
        None
    }

    /// Sovereign selection: `WebGPU` only
    fn select_sovereign(available: &[GpuFramework]) -> Option<GpuFramework> {
        if available.contains(&GpuFramework::WebGpu) {
            info!("✅ Selected WebGPU (sovereign mode)");
            Some(GpuFramework::WebGpu)
        } else {
            warn!("❌ WebGPU not available (sovereign mode requires WebGPU)");
            None
        }
    }

    /// Pragmatic selection: Prefer vendor backends for performance
    fn select_pragmatic(
        workload: Option<&WorkloadType>,
        available: &[GpuFramework],
    ) -> Option<GpuFramework> {
        // PRIORITY 1: CUDA for best performance (if suitable)
        if let Some(workload_type) = workload
            && Self::workload_prefers_cuda(workload_type)
            && available.contains(&GpuFramework::Cuda)
        {
            info!("🚀 Selected CUDA (pragmatic mode, maximum performance)");
            return Some(GpuFramework::Cuda);
        }

        // PRIORITY 2: Metal on Apple platforms
        if available.contains(&GpuFramework::Metal) {
            info!("🚀 Selected Metal (pragmatic mode, Apple optimized)");
            return Some(GpuFramework::Metal);
        }

        // PRIORITY 3: CUDA as general high-performance option
        if available.contains(&GpuFramework::Cuda) {
            info!("🚀 Selected CUDA (pragmatic mode, high performance)");
            return Some(GpuFramework::Cuda);
        }

        // PRIORITY 4: Vulkan
        if available.contains(&GpuFramework::Vulkan) {
            info!("🚀 Selected Vulkan (pragmatic mode, cross-platform)");
            return Some(GpuFramework::Vulkan);
        }

        // PRIORITY 5: OpenCL
        if available.contains(&GpuFramework::OpenCl) {
            info!("⚠️  Selected OpenCL (pragmatic mode, legacy)");
            return Some(GpuFramework::OpenCl);
        }

        // PRIORITY 6: WebGPU as fallback
        if available.contains(&GpuFramework::WebGpu) {
            info!("✅ Selected WebGPU (pragmatic mode fallback)");
            return Some(GpuFramework::WebGpu);
        }

        None
    }

    /// Check if workload REQUIRES CUDA (Python AI in 2025)
    const fn workload_needs_cuda(workload: &WorkloadType) -> bool {
        matches!(
            workload,
            WorkloadType::Python | WorkloadType::AiMl | WorkloadType::Cuda
        )
    }

    /// Check if workload PREFERS CUDA (but can work without it)
    const fn workload_prefers_cuda(workload: &WorkloadType) -> bool {
        matches!(
            workload,
            WorkloadType::Python | WorkloadType::AiMl | WorkloadType::Cuda | WorkloadType::Gpu
        )
    }
}

/// Evolution metrics for tracking ecosystem maturity
#[derive(Debug, Clone)]
pub struct EvolutionMetrics {
    /// `WebGPU` AI library coverage (0.0 - 1.0)
    pub webgpu_ai_coverage: f32,
    /// `WebGPU` performance vs CUDA (0.0 - 1.0+)
    pub webgpu_performance_ratio: f32,
    /// `PyTorch` `WebGPU` backend ready
    pub pytorch_webgpu_ready: bool,
    /// `TensorFlow` `WebGPU` backend ready
    pub tensorflow_webgpu_ready: bool,
    /// Burn (Rust) adoption rate
    pub burn_adoption_rate: f32,
    /// CUDA usage percentage (0.0 - 1.0)
    pub cuda_usage_percentage: f32,
    /// `WebGPU` usage percentage (0.0 - 1.0)
    pub webgpu_usage_percentage: f32,
}

impl Default for EvolutionMetrics {
    fn default() -> Self {
        Self {
            // Current state (2025)
            webgpu_ai_coverage: 0.3,        // 30% coverage (experimental)
            webgpu_performance_ratio: 0.7,  // 70% of CUDA performance
            pytorch_webgpu_ready: false,    // Experimental
            tensorflow_webgpu_ready: false, // In development
            burn_adoption_rate: 0.05,       // 5% (growing)
            cuda_usage_percentage: 0.85,    // 85% for Python AI
            webgpu_usage_percentage: 0.15,  // 15% for Rust/general
        }
    }
}

impl EvolutionMetrics {
    /// Check if ecosystem is ready to drop CUDA support
    pub fn ready_to_drop_cuda(&self) -> bool {
        self.webgpu_ai_coverage > 0.8
            && self.webgpu_performance_ratio > 0.95
            && (self.pytorch_webgpu_ready || self.tensorflow_webgpu_ready)
            && self.webgpu_usage_percentage > 0.7
    }

    /// Log current evolution status
    pub fn log_status(&self) {
        info!("🔬 GPU Evolution Status:");
        info!(
            "   WebGPU AI Coverage: {:.0}%",
            self.webgpu_ai_coverage * 100.0
        );
        info!(
            "   WebGPU Performance: {:.0}% of CUDA",
            self.webgpu_performance_ratio * 100.0
        );
        info!(
            "   PyTorch WebGPU: {}",
            if self.pytorch_webgpu_ready {
                "✅"
            } else {
                "⏳"
            }
        );
        info!(
            "   TensorFlow WebGPU: {}",
            if self.tensorflow_webgpu_ready {
                "✅"
            } else {
                "⏳"
            }
        );
        info!("   Burn Adoption: {:.1}%", self.burn_adoption_rate * 100.0);

        if self.ready_to_drop_cuda() {
            info!("🎉 READY TO DROP CUDA! Ecosystem has matured!");
        } else {
            info!("⏳ Waiting for WebGPU AI ecosystem maturity...");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_automatic_prefers_webgpu() {
        let available = vec![GpuFramework::WebGpu, GpuFramework::Cuda];
        let strategy = BackendSelectionStrategy::Automatic;
        let selected = strategy.select_framework(None, &available);
        assert_eq!(selected, Some(GpuFramework::WebGpu));
    }

    #[test]
    fn test_sovereign_only_requires_webgpu() {
        let available = vec![GpuFramework::Cuda];
        let strategy = BackendSelectionStrategy::SovereignOnly;
        let selected = strategy.select_framework(None, &available);
        assert_eq!(selected, None);

        let available = vec![GpuFramework::WebGpu];
        let selected = strategy.select_framework(None, &available);
        assert_eq!(selected, Some(GpuFramework::WebGpu));
    }

    #[test]
    fn test_pragmatic_prefers_cuda() {
        let available = vec![GpuFramework::WebGpu, GpuFramework::Cuda];
        let strategy = BackendSelectionStrategy::Pragmatic;
        let workload = WorkloadType::Python;
        let selected = strategy.select_framework(Some(&workload), &available);
        assert_eq!(selected, Some(GpuFramework::Cuda));
    }

    #[test]
    fn test_evolution_metrics_default() {
        let metrics = EvolutionMetrics::default();
        assert!(!metrics.ready_to_drop_cuda());
        assert!(metrics.webgpu_ai_coverage < 0.8);
    }

    #[test]
    fn test_evolution_metrics_ready_to_drop() {
        let metrics = EvolutionMetrics {
            webgpu_ai_coverage: 0.9,
            webgpu_performance_ratio: 0.98,
            pytorch_webgpu_ready: true,
            tensorflow_webgpu_ready: false,
            burn_adoption_rate: 0.15,
            cuda_usage_percentage: 0.2,
            webgpu_usage_percentage: 0.8,
        };
        assert!(metrics.ready_to_drop_cuda());
    }

    #[test]
    fn test_specific_framework_available() {
        let strategy = BackendSelectionStrategy::Specific(GpuFramework::Cuda);
        let available = vec![GpuFramework::Cuda, GpuFramework::WebGpu];
        let selected = strategy.select_framework(None, &available);
        assert_eq!(selected, Some(GpuFramework::Cuda));
    }

    #[test]
    fn test_specific_framework_unavailable_falls_back() {
        let strategy = BackendSelectionStrategy::Specific(GpuFramework::Cuda);
        let available = vec![GpuFramework::WebGpu, GpuFramework::OpenCl];
        let selected = strategy.select_framework(None, &available);
        assert_eq!(selected, Some(GpuFramework::WebGpu));
    }

    #[test]
    fn test_automatic_workload_needs_cuda_when_no_webgpu() {
        let strategy = BackendSelectionStrategy::Automatic;
        let available = vec![GpuFramework::Cuda, GpuFramework::OpenCl];
        let workload = WorkloadType::Python;
        let selected = strategy.select_framework(Some(&workload), &available);
        assert_eq!(selected, Some(GpuFramework::Cuda));
    }

    #[test]
    fn test_automatic_prefers_webgpu_over_cuda_even_for_python() {
        let strategy = BackendSelectionStrategy::Automatic;
        let available = vec![GpuFramework::WebGpu, GpuFramework::Cuda];
        let workload = WorkloadType::Python;
        let selected = strategy.select_framework(Some(&workload), &available);
        assert_eq!(selected, Some(GpuFramework::WebGpu));
    }

    #[test]
    fn test_automatic_empty_available_returns_none() {
        let strategy = BackendSelectionStrategy::Automatic;
        let available: Vec<GpuFramework> = vec![];
        let selected = strategy.select_framework(None, &available);
        assert_eq!(selected, None);
    }

    #[test]
    fn test_pragmatic_empty_returns_none() {
        let strategy = BackendSelectionStrategy::Pragmatic;
        let available: Vec<GpuFramework> = vec![];
        let selected = strategy.select_framework(None, &available);
        assert_eq!(selected, None);
    }

    #[test]
    fn test_evolution_metrics_default_values() {
        let m = EvolutionMetrics::default();
        assert!(m.webgpu_ai_coverage > 0.0);
        assert!(m.webgpu_performance_ratio > 0.0);
        assert!(!m.pytorch_webgpu_ready);
        assert!(m.cuda_usage_percentage > 0.5);
    }

    #[test]
    fn test_backend_strategy_default() {
        let strategy = BackendSelectionStrategy::default();
        assert!(matches!(strategy, BackendSelectionStrategy::Automatic));
    }
}
