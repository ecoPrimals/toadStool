//! Universal substrate for heterogeneous computing
//!
//! This module provides a unified interface for discovering and managing diverse computing
//! platforms, from traditional CPUs to quantum computers and experimental biological systems.
//!
//! # Architecture
//!
//! The universal substrate is organized into several key components:
//!
//! - **Types** (`types/`): Platform type definitions organized by category
//! - **Detection** (`detection.rs`): Capability detection across all platform types
//! - **Capabilities** (`UniversalSubstrateCapabilities`): Top-level API for platform management
//!
//! # Example
//!
//! ```no_run
//! use toadstool_distributed::universal::UniversalSubstrateCapabilities;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Detect all available computing platforms
//!     let capabilities = UniversalSubstrateCapabilities::detect_all().await?;
//!     
//!     // Query detected capabilities
//!     println!("Total platforms: {}", capabilities.total_platforms());
//!     println!("Has AI accelerators: {}", capabilities.has_ai_accelerators());
//!     println!("Has quantum: {}", capabilities.has_quantum_platforms());
//!     
//!     // Access specific platform types
//!     for platform in &capabilities.traditional_platforms {
//!         println!("CPU: {}", platform.architecture_name());
//!     }
//!     
//!     for runtime in &capabilities.language_runtimes {
//!         println!("Language: {}", runtime.language_name());
//!     }
//!     
//!     Ok(())
//! }
//! ```
//!
//! # Platform Categories
//!
//! ## Traditional Platforms
//! - x86_64, ARM64, RISC-V, PowerPC, SPARC, MIPS
//! - Standard CPU architectures with multi-core support
//!
//! ## Biological Computing
//! - DNA computing, protein folding, cellular computing
//! - Enzymatic and bacterial computing systems
//! - Neural organoids and bioelectronic interfaces
//!
//! ## Neuromorphic Computing
//! - Spiking neural networks
//! - Memristive computing
//! - Neuromorphic chips (TrueNorth, Loihi, etc.)
//!
//! ## Quantum Computing
//! - Gate-based quantum computers
//! - Quantum annealing systems
//! - Photonic quantum computers
//! - Trapped ion and superconducting systems
//!
//! ## Edge/IoT
//! - Microcontrollers (ESP32, STM32, etc.)
//! - Single-board computers (Raspberry Pi, etc.)
//! - IoT sensors and smart devices
//! - FPGAs and edge NPUs
//!
//! ## Container Platforms
//! - Traditional containers (Docker, Podman)
//! - WebAssembly runtimes (Wasmtime, Wasmer, WasmEdge)
//! - Serverless platforms (Lambda, Cloud Run, Azure Functions)
//! - Orchestration (Kubernetes, Docker Swarm, Nomad)
//!
//! ## Language Runtimes
//! - Systems languages (Rust, C, C++, Go, Zig)
//! - Memory-managed languages (Java, C#, Python, JavaScript)
//! - Functional languages (Haskell, OCaml, Erlang, Elixir)
//! - Domain-specific languages (R, MATLAB, Julia)
//!
//! ## Operating Systems
//! - Unix-like (Linux, BSD, macOS)
//! - Windows
//! - Mobile (Android, iOS)
//! - Embedded and real-time systems (FreeRTOS, Zephyr, VxWorks, QNX)
//! - Hypervisors (Xen, VMware, Hyper-V, KVM)
//!
//! ## Specialized Architectures
//! - AI/ML accelerators (TPU, NPU, IPU)
//! - GPU compute (CUDA, ROCm, OpenCL, Vulkan, Metal)
//! - Signal processors (DSP)
//! - Network processors (DPU)
//! - Custom silicon (ASIC)
//!
//! ## Experimental Platforms
//! - Molecular computing
//! - Cyborg systems (bio-electronic hybrids)
//! - Metamaterial processors
//! - Spintronics
//! - Superconducting classical computers
//! - Reversible computing
//!
//! # Detection Strategy
//!
//! Platform detection follows a multi-phase approach:
//!
//! 1. **System Introspection**: Query OS APIs for hardware information
//! 2. **Command Detection**: Check for installed tools and runtimes
//! 3. **Hardware Probing**: Detect specialized accelerators (GPU, TPU, etc.)
//! 4. **Network Discovery**: Scan for edge/IoT devices (optional)
//! 5. **Cloud API Integration**: Check for remote quantum/specialized resources
//!
//! # Design Principles
//!
//! - **Comprehensive**: Support for traditional to experimental platforms
//! - **Extensible**: Easy to add new platform types
//! - **Non-intrusive**: Detection doesn't affect system performance
//! - **Type-safe**: Strong typing for all platform capabilities
//! - **Async-first**: All detection operations are asynchronous
//! - **Graceful Degradation**: Missing platforms don't cause failures

// Re-export all platform types
pub use super::types::*;

// Re-export capabilities with detection
pub use super::types::UniversalSubstrateCapabilities;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_full_detection() {
        let caps = UniversalSubstrateCapabilities::detect_all()
            .await
            .expect("Universal substrate detection should succeed");

        // Should detect at least the current system
        assert!(!caps.is_empty());
        assert!(caps.total_platforms() > 0);
    }

    #[test]
    fn test_type_exports() {
        // Verify all types are accessible
        let _traditional = TraditionalPlatform::X86_64 {
            cpu_model: "Test".to_string(),
            cores: 4,
            threads: 8,
            cache_mb: 8,
            memory_gb: 16,
            features: vec![],
        };

        let _container = ContainerPlatform::Docker {
            version: "24.0.0".to_string(),
            features: vec![],
        };

        let _language = LanguageRuntime::Rust {
            version: "1.75.0".to_string(),
            target_triple: "x86_64-unknown-linux-gnu".to_string(),
            features: vec![],
        };
    }

    #[test]
    fn test_capabilities_structure() {
        let caps = UniversalSubstrateCapabilities::new();

        assert_eq!(caps.total_platforms(), 0);
        assert!(caps.is_empty());
        assert!(!caps.has_traditional_platforms());
        assert!(!caps.has_ai_accelerators());
        assert!(!caps.has_quantum_platforms());
    }
}
