// SPDX-License-Identifier: AGPL-3.0-or-later
//! Hydrological primitives for environmental and agricultural computing.
//!
//! | Function | Reference | Use case |
//! |----------|-----------|----------|
//! | [`hargreaves_et0`] | Hargreaves & Samani (1985) | Temperature-only ET₀ |
//! | [`crop_coefficient`] | FAO-56 Ch. 6 | Adjust ET₀ for crop stage |
//! | [`soil_water_balance`] | FAO-56 Ch. 8 | Daily soil moisture bookkeeping |
//!
//! # Provenance
//!
//! Absorbed from airSpring `metalForge/forge/src/hydrology.rs` (V009).
//! Validated against FAO-56 (Allen et al. 1998), 918 station-days,
//! cross-validated with Python `ETo` library within 1e-5 tolerance.

/// Hargreaves empirical coefficient (dimensionless).
const HARGREAVES_COEFF: f64 = 0.0023;

/// Temperature offset for Hargreaves formula (°C).
const HARGREAVES_TEMP_OFFSET: f64 = 17.8;

/// Daily reference evapotranspiration via Hargreaves & Samani (1985).
///
/// `ET₀ = 0.0023 · Ra · (t_mean + 17.8) · √(t_max − t_min)`
///
/// Returns `None` if inputs are physically impossible (`t_max` < `t_min` or `ra` < 0).
#[must_use]
pub fn hargreaves_et0(ra: f64, t_max: f64, t_min: f64) -> Option<f64> {
    let delta = t_max - t_min;
    if delta < 0.0 || ra < 0.0 {
        return None;
    }
    let t_mean = (t_max + t_min) * 0.5;
    Some(HARGREAVES_COEFF * ra * (t_mean + HARGREAVES_TEMP_OFFSET) * delta.sqrt())
}

/// Batched Hargreaves ET₀ over multiple days.
///
/// Returns `None` if slice lengths differ.
#[must_use]
pub fn hargreaves_et0_batch(ra: &[f64], t_max: &[f64], t_min: &[f64]) -> Option<Vec<f64>> {
    if ra.len() != t_max.len() || ra.len() != t_min.len() {
        return None;
    }
    ra.iter()
        .zip(t_max)
        .zip(t_min)
        .map(|((&r, &tx), &tn)| hargreaves_et0(r, tx, tn))
        .collect()
}

/// Crop coefficient (Kc) interpolation for a development stage.
///
/// Linearly interpolates between `kc_prev` and `kc_next` based on
/// `day_in_stage / stage_length`. Returns `kc_prev` if `stage_length` is 0.
#[must_use]
pub fn crop_coefficient(kc_prev: f64, kc_next: f64, day_in_stage: u32, stage_length: u32) -> f64 {
    if stage_length == 0 {
        return kc_prev;
    }
    let frac = f64::from(day_in_stage) / f64::from(stage_length);
    (kc_next - kc_prev).mul_add(frac, kc_prev)
}

/// Daily soil water balance update (FAO-56 Chapter 8).
///
/// `θ_new = θ_old + precip + irrigation − et_c`
///
/// Result clamped to `[0, field_capacity]`.
#[must_use]
pub fn soil_water_balance(
    theta: f64,
    precip: f64,
    irrigation: f64,
    et_c: f64,
    field_capacity: f64,
) -> f64 {
    let raw = theta + precip + irrigation - et_c;
    raw.clamp(0.0, field_capacity)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hargreaves_typical() {
        let et0 = hargreaves_et0(35.0, 32.0, 18.0).unwrap();
        assert!(et0 > 0.0 && et0 < 15.0, "ET₀={et0} out of range");
    }

    #[test]
    fn test_hargreaves_zero_delta() {
        let et0 = hargreaves_et0(35.0, 25.0, 25.0).unwrap();
        assert!(et0.abs() < 1e-12, "zero ΔT → zero ET₀");
    }

    #[test]
    fn test_hargreaves_invalid() {
        assert!(hargreaves_et0(35.0, 18.0, 32.0).is_none());
        assert!(hargreaves_et0(-1.0, 30.0, 20.0).is_none());
    }

    #[test]
    fn test_hargreaves_batch() {
        let ra = vec![30.0, 35.0, 40.0];
        let tmax = vec![28.0, 32.0, 35.0];
        let tmin = vec![15.0, 18.0, 20.0];
        let et0 = hargreaves_et0_batch(&ra, &tmax, &tmin).unwrap();
        assert_eq!(et0.len(), 3);
        for &e in &et0 {
            assert!(e > 0.0);
        }
    }

    #[test]
    fn test_hargreaves_batch_mismatched() {
        assert!(hargreaves_et0_batch(&[1.0], &[2.0, 3.0], &[1.0]).is_none());
    }

    #[test]
    fn test_crop_coefficient_start() {
        assert!((crop_coefficient(0.3, 1.2, 0, 30) - 0.3).abs() < 1e-12);
    }

    #[test]
    fn test_crop_coefficient_end() {
        assert!((crop_coefficient(0.3, 1.2, 30, 30) - 1.2).abs() < 1e-12);
    }

    #[test]
    fn test_crop_coefficient_mid() {
        let kc = crop_coefficient(0.3, 1.2, 15, 30);
        assert!((kc - 0.75).abs() < 1e-12);
    }

    #[test]
    fn test_crop_coefficient_zero_stage() {
        assert!((crop_coefficient(0.3, 1.2, 0, 0) - 0.3).abs() < 1e-12);
    }

    #[test]
    fn test_soil_water_balance_basic() {
        let theta = soil_water_balance(100.0, 10.0, 5.0, 8.0, 200.0);
        assert!((theta - 107.0).abs() < 1e-12);
    }

    #[test]
    fn test_soil_water_balance_saturated() {
        let theta = soil_water_balance(190.0, 50.0, 0.0, 3.0, 200.0);
        assert!((theta - 200.0).abs() < 1e-12);
    }

    #[test]
    fn test_soil_water_balance_dry() {
        let theta = soil_water_balance(5.0, 0.0, 0.0, 20.0, 200.0);
        assert!(theta.abs() < 1e-12);
    }

    #[test]
    fn test_hargreaves_fao56_example() {
        let et0 = hargreaves_et0(40.6, 26.6, 14.8).unwrap();
        assert!(
            (et0 - 12.35).abs() < 0.1,
            "Hargreaves: expected ~12.35, got {et0}"
        );
    }
}
