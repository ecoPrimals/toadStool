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

pub mod bootstrap;
pub mod chi2;
pub mod correlation;
pub mod normal;

pub use bootstrap::{bootstrap_ci, bootstrap_mean, bootstrap_median, bootstrap_std, BootstrapCI};
pub use chi2::{chi2_decomposed, chi2_decomposed_weighted, Chi2Decomposed};
pub use correlation::{correlation_matrix, covariance, covariance_matrix, pearson_correlation};
pub use normal::{norm_cdf, norm_cdf_batch, norm_pdf, norm_pdf_batch, norm_ppf};
