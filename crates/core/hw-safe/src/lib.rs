// SPDX-License-Identifier: AGPL-3.0-only
#![warn(missing_docs)]

//! # toadstool-hw-safe — Safe Wrappers for Hardware Primitives
//!
//! This crate is toadStool's **single unsafe containment zone**. Every other
//! crate in the workspace uses `#![forbid(unsafe_code)]` and depends on
//! this crate for hardware-level operations.
//!
//! ## What lives here
//!
//! - [`SafeMmapRegion`] — RAII memory-mapped file region (mmap/munmap)
//! - [`VolatileMmio`] — bounds-checked volatile MMIO register access
//! - [`AlignedAlloc`] — heap allocation with arbitrary alignment
//! - (future) `LockedMemory` — mlock/munlock for DMA-safe and secure memory
//!
//! ## Design principle
//!
//! Each type encapsulates the minimum unsafe needed for its operation.
//! The public API is entirely safe. All `unsafe` blocks have `// SAFETY:`
//! comments documenting invariants.
//!
//! The goal is to reduce this crate's unsafe surface to the irreducible
//! minimum (~26 operations), then iterate each one toward pure Rust
//! alternatives (e.g. `memmap2` for mmap, `aligned-vec` for allocation).

pub mod aligned_alloc;
pub mod safe_mmap;
pub mod volatile_mmio;

pub use aligned_alloc::AlignedAlloc;
pub use safe_mmap::SafeMmapRegion;
pub use volatile_mmio::VolatileMmio;
