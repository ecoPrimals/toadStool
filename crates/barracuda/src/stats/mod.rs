//! Statistical functions
//!
//! Provides core statistical distributions and measures for scientific computing.
//!
//! # Distributions
//!
//! - **Normal (Gaussian)**: CDF, PDF, inverse CDF (probit/quantile)
//! - Uses erf-based implementations for high accuracy
//!
//! # Correlation & Covariance
//!
//! - **Pearson correlation**: Linear correlation coefficient
//! - **Covariance**: Sample and population covariance
//! - **Correlation matrix**: Pairwise correlations
//!
//! # References
//!
//! - Abramowitz & Stegun §26 (Normal distribution)
//! - Moro (1995) inverse normal approximation

pub mod normal;
pub mod correlation;

pub use normal::{norm_cdf, norm_pdf, norm_ppf, norm_cdf_batch, norm_pdf_batch};
pub use correlation::{pearson_correlation, covariance, correlation_matrix, covariance_matrix};
