//! Userspace NPU backend via memory-mapped PCIe BARs
//!
//! Deep Debt Compliance:
//! - Zero unsafe code (uses safe mmap wrapper)
//! - Runtime capability discovery (no hardcoding)
//! - Comprehensive error handling
//! - Graceful fallbacks

pub mod kernel;
pub mod mmap;
pub mod userspace;

pub use kernel::KernelBackend;
pub use userspace::UserspaceBackend;
