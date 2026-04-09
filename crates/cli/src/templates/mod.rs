// SPDX-License-Identifier: AGPL-3.0-or-later
//! Biome Templates - Universal Compute Manifest Generation
//!
//! Templates for creating biome.yaml manifests for different scientific computing workflows.
//! Each template embodies the principles of SOVEREIGN SCIENCE and universal compute.
//!
//! ## Module Structure
//!
//! - `types_mod`: Template type definitions (BiomeTemplate, CustomTemplateSpec, etc.)
//! - `generator_impl`: TemplateGenerator implementation (orchestration layer)
//! - `basic_templates`: Basic and development template implementations
//! - `specialized_templates`: Science, AI, Quantum, and other specialized templates
//! - `rendering`: YAML rendering and template information display

// Type definitions
pub mod types_mod;
pub use types_mod::{BiomeTemplate, CustomServiceSpec, CustomTemplateSpec};

// Zero-copy constants
pub mod capability_constants;
pub mod constants; // Capability ids + legacy template aliases (see `constants::service_names` docs)

// Template implementations
pub mod basic_templates;
pub mod capability_helpers;
mod rendering;
pub mod specialized_templates;

// Core imports (minimal, only for struct definition)
use std::path::PathBuf;

/// Template generator for biome manifests
pub struct TemplateGenerator {
    output_dir: PathBuf,
    force_overwrite: bool,
}

// TemplateGenerator implementation
mod generator_impl;
