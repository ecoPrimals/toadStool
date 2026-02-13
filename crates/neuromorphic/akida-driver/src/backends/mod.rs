//! NPU backend implementations
//!
//! Three backends available:
//! - **Kernel**: Uses `/dev/akida*` (requires C kernel module, best performance)
//! - **VFIO**: Pure Rust with DMA via IOMMU (no C module, good performance)
//! - **Userspace**: Memory-mapped PCIe BARs (pure Rust, no DMA, development)
//!
//! Deep Debt Compliance:
//! - Runtime capability discovery (no hardcoding)
//! - Comprehensive error handling
//! - Graceful fallbacks

pub mod kernel;
pub mod mmap;
pub mod userspace;
pub mod vfio;

pub use kernel::KernelBackend;
pub use userspace::UserspaceBackend;
pub use vfio::VfioBackend;
