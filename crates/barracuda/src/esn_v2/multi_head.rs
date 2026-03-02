//! Multi-head ESN: shared reservoir with per-head readouts (hotSpring 36-head concept).
//!
//! `HeadKind` provides a domain-agnostic head classification system:
//! - Pre-defined kinds for known physics/bio/steering domains
//! - `Custom(String)` for spring-specific or experimental heads
//!
//! Provenance: hotSpring V0615 (physics), wetSpring V86 (BioHeadKind) → generalized

use crate::error::{BarracudaError, Result as BarracudaResult};
use crate::linalg::solve_f64_cpu;
use crate::tensor::Tensor;

use super::config::{validate_config, ESNConfig};
use super::model::{ExportedWeights, ESN};

/// Domain-agnostic head classification for multi-head ESN.
///
/// Pre-defined kinds cover common domains across all springs.
/// Use `Custom(String)` for spring-specific or experimental heads.
///
/// # Evolution from `HeadGroup`
/// S79 had 6 fixed physics groups. S83 generalizes to support:
/// - hotSpring physics (Anderson, Qcd, Potts)
/// - wetSpring biology (Diversity, Taxonomy, Amr, Bloom)
/// - airSpring hydrology (Et0, SoilMoisture, Irrigation)
/// - Any custom domain via `Custom(String)`
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum HeadKind {
    // Physics (hotSpring)
    Anderson,
    Qcd,
    Potts,
    Steering,
    Brain,
    Meta,
    // Biology (wetSpring BioHeadKind absorption)
    Diversity,
    Taxonomy,
    Amr,
    Bloom,
    Disorder,
    // Hydrology (airSpring)
    Et0,
    SoilMoisture,
    Irrigation,
    // Extensible
    Custom(String),
}

/// Backward-compatible alias.
pub type HeadGroup = HeadKind;

/// Configuration for a head within a group.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HeadConfig {
    pub group: HeadKind,
    pub label: String,
    pub output_size: usize,
}

struct HeadReadout {
    w_out: Option<Tensor>,
    trained: bool,
}

/// Multi-head ESN wrapping a shared reservoir with per-head readout.
pub struct MultiHeadEsn {
    reservoir: ESN,
    heads: Vec<HeadReadout>,
    head_configs: Vec<HeadConfig>,
}

impl MultiHeadEsn {
    /// Create a shared reservoir with per-head readout slots (no readout weights yet).
    pub async fn new(base_config: ESNConfig, heads: Vec<HeadConfig>) -> BarracudaResult<Self> {
        validate_config(&base_config)?;
        if heads.is_empty() {
            return Err(BarracudaError::InvalidInput {
                message: "At least one head required".to_string(),
            });
        }
        for h in &heads {
            if h.output_size == 0 {
                return Err(BarracudaError::InvalidInput {
                    message: format!("Head '{}' has zero output_size", h.label),
                });
            }
        }

        let reservoir_config = ESNConfig {
            output_size: 1,
            ..base_config.clone()
        };
        let reservoir = ESN::new(reservoir_config).await?;

        let head_readouts = heads
            .iter()
            .map(|_| HeadReadout {
                w_out: None,
                trained: false,
            })
            .collect();

        Ok(Self {
            reservoir,
            heads: head_readouts,
            head_configs: heads,
        })
    }

    /// Forward through shared reservoir; returns new state.
    pub async fn update(&mut self, input: &Tensor) -> BarracudaResult<Tensor> {
        self.reservoir.update(input).await
    }

    /// Train a single head's readout via ridge regression.
    pub fn train_head(
        &mut self,
        head_idx: usize,
        states: &[f64],
        targets: &[f64],
        lambda: f64,
    ) -> BarracudaResult<()> {
        let Some(cfg) = self.head_configs.get(head_idx) else {
            return Err(BarracudaError::InvalidInput {
                message: format!("Head index {} out of range", head_idx),
            });
        };
        let n = self.reservoir.config().reservoir_size;
        let m = cfg.output_size;

        if states.is_empty() {
            return Err(BarracudaError::InvalidInput {
                message: "States cannot be empty".to_string(),
            });
        }
        if !states.len().is_multiple_of(n) {
            return Err(BarracudaError::InvalidInput {
                message: format!(
                    "States length {} must be divisible by reservoir_size {}",
                    states.len(),
                    n
                ),
            });
        }
        let n_samples = states.len() / n;
        if targets.len() != m * n_samples {
            return Err(BarracudaError::InvalidInput {
                message: format!(
                    "Targets length {} expected {}",
                    targets.len(),
                    m * n_samples
                ),
            });
        }
        if lambda <= 0.0 {
            return Err(BarracudaError::InvalidInput {
                message: "Lambda must be positive".to_string(),
            });
        }

        let x = states;
        let y = targets;

        let mut m_mat = vec![0.0; n * n];
        for i in 0..n {
            for j in 0..n {
                let mut sum = 0.0;
                for k in 0..n_samples {
                    sum += x[i * n_samples + k] * x[j * n_samples + k];
                }
                m_mat[i * n + j] = sum;
            }
            m_mat[i * n + i] += lambda;
        }

        let mut b_mat = vec![0.0; n * m];
        for i in 0..n {
            for j in 0..m {
                let mut sum = 0.0;
                for k in 0..n_samples {
                    sum += x[i * n_samples + k] * y[j * n_samples + k];
                }
                b_mat[i * m + j] = sum;
            }
        }

        let mut w_out_t = vec![0.0; n * m];
        for j in 0..m {
            let b_col: Vec<f64> = (0..n).map(|i| b_mat[i * m + j]).collect();
            let w_col = solve_f64_cpu(&m_mat, &b_col, n)?;
            for (i, &w) in w_col.iter().enumerate() {
                w_out_t[i * m + j] = w;
            }
        }

        let device = self.reservoir.state().device().clone();
        let w_out_f32: Vec<f32> = w_out_t.iter().map(|&x| x as f32).collect();
        let w_out = Tensor::from_data(&w_out_f32, vec![n, m], device)?;

        let head = &mut self.heads[head_idx];
        head.w_out = Some(w_out);
        head.trained = true;

        Ok(())
    }

    /// Predict from a single head given reservoir state.
    pub fn predict_head(&self, head_idx: usize, state: &Tensor) -> BarracudaResult<Tensor> {
        let head = self
            .heads
            .get(head_idx)
            .ok_or_else(|| BarracudaError::InvalidInput {
                message: format!("Head index {} out of range", head_idx),
            })?;
        let w_out = head
            .w_out
            .as_ref()
            .ok_or_else(|| BarracudaError::InvalidOperation {
                op: "MultiHeadEsn::predict_head".to_string(),
                reason: format!("Head {} not trained", head_idx),
            })?;
        w_out.transpose()?.matmul(state)
    }

    /// Predict from all trained heads.
    pub fn predict_all(&self, state: &Tensor) -> BarracudaResult<Vec<Tensor>> {
        let mut out = Vec::with_capacity(self.heads.len());
        for i in 0..self.heads.len() {
            if self.heads[i].trained {
                out.push(self.predict_head(i, state)?);
            }
        }
        Ok(out)
    }

    /// Mean pairwise L2 distance between head predictions (uncertainty signal).
    pub fn head_disagreement(&self, state: &Tensor) -> BarracudaResult<f64> {
        let preds: Vec<Vec<f32>> = (0..self.heads.len())
            .filter(|&i| self.heads[i].trained)
            .map(|i| self.predict_head(i, state).and_then(|t| t.to_vec()))
            .collect::<BarracudaResult<Vec<_>>>()?;

        if preds.len() < 2 {
            return Ok(0.0);
        }

        let mut total = 0.0;
        let mut count = 0usize;
        for i in 0..preds.len() {
            for j in (i + 1)..preds.len() {
                let a = &preds[i];
                let b = &preds[j];
                let len = a.len().min(b.len());
                let d2: f64 = (0..len)
                    .map(|k| {
                        let diff = a[k] as f64 - b[k] as f64;
                        diff * diff
                    })
                    .sum();
                total += d2.sqrt();
                count += 1;
            }
        }
        Ok(if count > 0 { total / count as f64 } else { 0.0 })
    }

    /// Export weights with head_labels populated.
    pub fn export_weights(&self) -> BarracudaResult<ExportedWeights> {
        let mut base = self.reservoir.export_weights()?;

        let mut all_w_out = Vec::new();
        let mut head_labels = Vec::new();
        let mut total_output_size = 0usize;

        for (i, head) in self.heads.iter().enumerate() {
            if let Some(ref w) = head.w_out {
                let data = w.to_vec()?;
                all_w_out.extend(data);
                total_output_size += self.head_configs[i].output_size;
                head_labels.push(self.head_configs[i].label.clone());
            }
        }

        base.w_out = if all_w_out.is_empty() {
            None
        } else {
            Some(all_w_out)
        };
        base.output_size = total_output_size;
        base.head_labels = head_labels;

        Ok(base)
    }
}
