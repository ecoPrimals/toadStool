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
//! # Chi-Squared Analysis
//!
//! - **chi2_decomposed**: Per-datum residuals, pulls, and contributions
//! - **chi2_decomposed_weighted**: With known uncertainties
//!
//! # Bootstrap Inference
//!
//! - **bootstrap_ci**: Non-parametric confidence intervals for any statistic
//! - **bootstrap_mean/median/std**: Convenience functions
//!
//! # References
//!
//! - Abramowitz & Stegun §26 (Normal distribution)
//! - Moro (1995) inverse normal approximation
//! - Efron & Tibshirani (1993) Bootstrap methods
//! - hotSpring validation: `stats.rs`

#[allow(dead_code)]
const WGSL_BOOTSTRAP_MEAN_F64: &str = include_str!("../shaders/special/bootstrap_mean_f64.wgsl");

/// WGSL shader: parallel histogram via atomic binning
#[allow(dead_code)]
pub const WGSL_HISTOGRAM: &str = include_str!("../shaders/stats/histogram.wgsl");

pub mod bootstrap;
pub mod chi2;
pub mod correlation;
pub mod normal;
pub mod spectral_density;

pub use bootstrap::{bootstrap_ci, bootstrap_mean, bootstrap_median, bootstrap_std, BootstrapCI};
pub use chi2::{chi2_decomposed, chi2_decomposed_weighted, Chi2Decomposed};
pub use correlation::{correlation_matrix, covariance, covariance_matrix, pearson_correlation};
pub use normal::{norm_cdf, norm_cdf_batch, norm_pdf, norm_pdf_batch, norm_ppf};
pub use spectral_density::{empirical_spectral_density, marchenko_pastur_bounds};
