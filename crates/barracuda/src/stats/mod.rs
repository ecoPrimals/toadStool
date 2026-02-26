//! Statistical functions
//!
//! Provides core statistical distributions, measures, and agreement metrics
//! for scientific computing across all springs.
//!
//! # Agreement Metrics (S64 absorption)
//!
//! - **RMSE, MBE, NSE, R², IA, hit_rate**: Model validation
//! - **mean, percentile**: Descriptive statistics
//! - **dot, l2_norm**: CPU vector operations
//!
//! # Ecological Diversity (S64 absorption)
//!
//! - **Shannon, Simpson, Chao1, Pielou**: Alpha diversity
//! - **Bray-Curtis**: Beta diversity (pairwise dissimilarity)
//! - **Rarefaction curves**: Expected species vs subsampling depth
//!
//! # Distributions
//!
//! - **Normal (Gaussian)**: CDF, PDF, inverse CDF (probit/quantile)
//!
//! # Correlation & Covariance
//!
//! - **Pearson/Spearman correlation**, **Covariance**, **Correlation matrix**
//!
//! # Chi-Squared / Bootstrap
//!
//! - **chi2_decomposed**: Per-datum residuals, pulls, contributions
//! - **bootstrap_ci**: Non-parametric confidence intervals
//!
//! # References
//!
//! - Abramowitz & Stegun §26, Moro (1995), Efron & Tibshirani (1993)
//! - QIIME2/skbio for diversity metrics, Willmott (1981) for IA

/// WGSL kernel for GPU-parallel bootstrap mean estimation (f64).
pub const WGSL_BOOTSTRAP_MEAN_F64: &str = include_str!("../shaders/special/bootstrap_mean_f64.wgsl");

/// WGSL kernel for parallel histogram via atomic binning.
pub const WGSL_HISTOGRAM: &str = include_str!("../shaders/stats/histogram.wgsl");

pub mod bootstrap;
pub mod chi2;
pub mod correlation;
pub mod diversity;
pub mod metrics;
pub mod normal;
pub mod spectral_density;

pub use bootstrap::{bootstrap_ci, bootstrap_mean, bootstrap_median, bootstrap_std, BootstrapCI};
pub use chi2::{chi2_decomposed, chi2_decomposed_weighted, Chi2Decomposed};
pub use correlation::{correlation_matrix, covariance, covariance_matrix, pearson_correlation};
pub use metrics::{
    dot, hit_rate, index_of_agreement, l2_norm, mbe, mean, nash_sutcliffe, percentile, r_squared,
    rmse,
};
pub use normal::{norm_cdf, norm_cdf_batch, norm_pdf, norm_pdf_batch, norm_ppf};
pub use diversity::{
    alpha_diversity, bray_curtis, bray_curtis_condensed, bray_curtis_matrix, chao1,
    condensed_index, observed_features, pielou_evenness, rarefaction_curve, shannon, simpson,
    AlphaDiversity,
};
pub use spectral_density::{empirical_spectral_density, marchenko_pastur_bounds};
