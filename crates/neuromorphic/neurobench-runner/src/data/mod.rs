// SPDX-License-Identifier: AGPL-3.0-only
//! Data loading utilities for `NeuroBench`
//!
//! Handles loading benchmark datasets from standard `NeuroBench` formats.
//!
//! ## Supported Formats
//!
//! - **DVS Gesture**: NPY files with shape [samples, time, x, y, polarity]
//! - **Keyword FSCIL**: MFCC features from Google Speech Commands
//! - **Chaotic Function**: CSV time series (Lorenz, Mackey-Glass)
//! - **NHP Motor**: Neural spike trains in NPY format

mod benchmarks;
mod csv;
mod dataset;
mod npy;
mod sample;

pub use dataset::{Dataset, DatasetSplit};
pub use sample::Sample;

#[cfg(test)]
mod tests;
