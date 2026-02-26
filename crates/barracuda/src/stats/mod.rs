//! Statistical functions
//!
//! Provides core statistical distributions, measures, and agreement metrics
//! for scientific computing across all springs.
//!
//! # Agreement Metrics (S64 absorption)
//!
//! - **RMSE, MAE, MBE, NSE, R², IA, hit_rate**: Model validation
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
//! # Regression (S66 absorption from airSpring)
//!
//! - **Linear, Quadratic, Exponential, Logarithmic**: Closed-form least-squares
//!
//! # Hydrology (S66 absorption from airSpring)
//!
//! - **Hargreaves ET₀, Crop coefficient, Soil water balance**: FAO-56 reference
//!
//! # Moving Window f64 (S66 absorption from airSpring)
//!
//! - **CPU f64 sliding window**: mean, variance, min, max
//!
//! # References
//!
//! - Abramowitz & Stegun §26, Moro (1995), Efron & Tibshirani (1993)
//! - QIIME2/skbio for diversity metrics, Willmott (1981) for IA
//! - Hargreaves & Samani (1985), FAO-56 (Allen et al. 1998)
//! - Dong et al. (2020) *Agriculture* 10(12):598

/// WGSL kernel for GPU-parallel bootstrap mean estimation (f64).
pub const WGSL_BOOTSTRAP_MEAN_F64: &str = include_str!("../shaders/special/bootstrap_mean_f64.wgsl");

/// WGSL kernel for parallel histogram via atomic binning.
pub static WGSL_HISTOGRAM: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
    crate::shaders::precision::downcast_f64_to_f32_with_transcendentals(include_str!(
        "../shaders/stats/histogram_f64.wgsl"
    ))
});

pub mod bootstrap;
pub mod chi2;
pub mod correlation;
pub mod diversity;
pub mod hydrology;
pub mod metrics;
pub mod moving_window_f64;
pub mod normal;
pub mod regression;
pub mod spectral_density;

pub use bootstrap::{
    bootstrap_ci, bootstrap_mean, bootstrap_median, bootstrap_std, rawr_mean, BootstrapCI,
};
pub use chi2::{chi2_decomposed, chi2_decomposed_weighted, Chi2Decomposed};
pub use correlation::{
    correlation_matrix, covariance, covariance_matrix, pearson_correlation, spearman_correlation,
};
pub use metrics::{
    dot, hill, hit_rate, index_of_agreement, l2_norm, mae, mbe, mean, monod, nash_sutcliffe,
    percentile, r_squared, rmse,
};
pub use normal::{norm_cdf, norm_cdf_batch, norm_pdf, norm_pdf_batch, norm_ppf};
pub use diversity::{
    alpha_diversity, bray_curtis, bray_curtis_condensed, bray_curtis_matrix, chao1,
    condensed_index, observed_features, pielou_evenness, rarefaction_curve, shannon,
    shannon_from_frequencies, simpson, AlphaDiversity,
};
pub use spectral_density::{empirical_spectral_density, marchenko_pastur_bounds};
pub use regression::{fit_all, fit_exponential, fit_linear, fit_logarithmic, fit_quadratic, FitResult};
pub use hydrology::{
    crop_coefficient, hargreaves_et0, hargreaves_et0_batch, soil_water_balance,
};
pub use moving_window_f64::{moving_window_stats_f64, MovingWindowResultF64};