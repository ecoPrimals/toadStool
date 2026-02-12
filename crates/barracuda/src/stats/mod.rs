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

pub mod correlation;
pub mod normal;

pub use correlation::{correlation_matrix, covariance, covariance_matrix, pearson_correlation};
pub use normal::{norm_cdf, norm_cdf_batch, norm_pdf, norm_pdf_batch, norm_ppf};
