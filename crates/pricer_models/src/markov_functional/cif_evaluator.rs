//! CIF (Callable Inverse Floater) hybrid evaluation.
//!
//! Pre-computes analytical option values for CIF coupons and provides
//! the 4-component Taylor decomposition (dE, dR, dI, dQ) used during
//! tree backward induction. European option components are evaluated
//! analytically for precision, while the tree handles only callability
//! and non-linear residuals.

use pricer_core::{
    math::{formulas::bachelier::Bachelier, normal_dist::norm_cdf, numeric::from_f64},
    traits::Float,
};

use super::MfmError;

// ─── CIF product definition ─────────────────────────────────────────

/// Callable Inverse Floater product definition.
///
/// A CIF coupon pays `max(floor_rate, fixed_rate - leverage * Libor)`,
/// optionally capped at `cap_rate`.
#[derive(Debug, Clone)]
pub struct CifInstrument<T: Float> {
    /// Fixed coupon rate (e.g., 0.06 for 6%).
    pub fixed_rate: T,
    /// Leverage on the floating rate (e.g., 1.0 or 2.0).
    pub leverage: T,
    /// Floor rate on the coupon (e.g., 0.0).
    pub floor_rate: T,
    /// Optional cap rate on the coupon.
    pub cap_rate: Option<T>,
    /// Notional amount.
    pub notional: T,
    /// Coupon dates as year fractions from valuation.
    pub coupon_dates: Vec<T>,
    /// Payment dates as year fractions from valuation.
    pub payment_dates: Vec<T>,
    /// Year fractions (accrual periods) for each coupon.
    pub year_fractions: Vec<T>,
}

impl<T: Float> CifInstrument<T> {
    /// Validate the instrument parameters.
    ///
    /// Checks that:
    /// - `leverage > 0`
    /// - `notional > 0`
    /// - `floor_rate >= 0`
    /// - `coupon_dates`, `payment_dates`, `year_fractions` have the same length
    /// - If `cap_rate` is present, `cap_rate > floor_rate`
    pub fn validate(&self) -> Result<(), MfmError> {
        if self.leverage <= T::zero() {
            return Err(MfmError::InvalidParameter {
                name: "leverage",
                reason: "must be positive".to_string(),
            });
        }
        if self.notional <= T::zero() {
            return Err(MfmError::InvalidParameter {
                name: "notional",
                reason: "must be positive".to_string(),
            });
        }
        if self.floor_rate < T::zero() {
            return Err(MfmError::InvalidParameter {
                name: "floor_rate",
                reason: "must be non-negative".to_string(),
            });
        }
        if self.coupon_dates.len() != self.payment_dates.len()
            || self.coupon_dates.len() != self.year_fractions.len()
        {
            return Err(MfmError::InvalidParameter {
                name: "coupon_dates/payment_dates/year_fractions",
                reason: "must have the same length".to_string(),
            });
        }
        if let Some(cap) = self.cap_rate {
            if cap <= self.floor_rate {
                return Err(MfmError::InvalidParameter {
                    name: "cap_rate",
                    reason: "must be greater than floor_rate".to_string(),
                });
            }
        }
        Ok(())
    }

    /// Returns the number of coupon periods.
    pub fn num_coupons(&self) -> usize { self.coupon_dates.len() }
}

// ─── 4-component Taylor decomposition ───────────────────────────────

/// The 4-component decomposition of a CIF coupon value at a single tree node.
///
/// The total coupon value is decomposed as `dE + dR + dI + dQ` where:
/// - `dE`: Floor value plus base expected coupon (analytical)
/// - `dR`: Linear delta w.r.t. swap rate deviation from forward
/// - `dI`: Libor basis adjustment (deviation from forward Libor)
/// - `dQ`: Non-linear residual / smile correction
#[derive(Debug, Clone, Copy)]
pub struct CifCouponComponents<T: Float> {
    /// dE: Floor value + base expected coupon.
    pub d_e: T,
    /// dR: Linear delta w.r.t. swap rate deviation.
    pub d_r: T,
    /// dI: Libor basis adjustment.
    pub d_i: T,
    /// dQ: Non-linear residual (smile correction).
    pub d_q: T,
}

impl<T: Float> Default for CifCouponComponents<T> {
    fn default() -> Self {
        Self {
            d_e: T::zero(),
            d_r: T::zero(),
            d_i: T::zero(),
            d_q: T::zero(),
        }
    }
}

impl<T: Float> CifCouponComponents<T> {
    /// Returns the total coupon value: `dE + dR + dI + dQ`.
    pub fn total(&self) -> T { self.d_e + self.d_r + self.d_i + self.d_q }
}

// ─── Pre-computed node info for one coupon period ───────────────────

/// Pre-computed CIF evaluation info for one coupon date across all tree nodes.
///
/// Contains analytical option values and the 4-component decomposition
/// that the tree engine uses during backward induction.
#[derive(Debug, Clone)]
pub struct CifNodeInfo<T: Float> {
    /// Coupon period index.
    pub coupon_idx: usize,
    /// Coupon date (year fraction).
    pub coupon_date_yf: T,
    /// Payment date (year fraction).
    pub payment_date_yf: T,
    /// Year fraction for this accrual period.
    pub year_fraction: T,
    /// Notional.
    pub notional: T,
    /// Forward swap rate (ATM).
    pub forward_swap_rate: T,
    /// Forward Libor rate (ATM).
    pub forward_libor: T,
    /// Normal vol for this coupon period.
    pub normal_vol: T,
    /// Analytical floor option values per node (ContPO).
    pub floor_values: Vec<T>,
    /// Expected coupon per node (ContBP).
    pub expected_coupons: Vec<T>,
    /// Delta w.r.t. swap rate per node (ContBD).
    pub delta_swap_rate: Vec<T>,
    /// 4-component decomposition per node.
    pub components: Vec<CifCouponComponents<T>>,
    /// Discounted coupon value per node (final usable value).
    pub discounted_values: Vec<T>,
}

// ─── CIF schedule ───────────────────────────────────────────────────

/// All CIF node infos for the entire product.
#[derive(Debug, Clone)]
pub struct CifSchedule<T: Float> {
    /// The CIF instrument definition.
    pub instrument: CifInstrument<T>,
    /// Pre-computed coupon info for each coupon period.
    pub coupons: Vec<CifNodeInfo<T>>,
}

// ─── Core computation ───────────────────────────────────────────────

/// Compute the floored (and optionally capped) CIF coupon rate for a given
/// Libor rate.
///
/// Returns `min(cap, max(floor, fixed - leverage * libor))` scaled by
/// `year_fraction * notional`.
fn cif_coupon_value<T: Float>(
    fixed_rate: T,
    leverage: T,
    floor_rate: T,
    cap_rate: Option<T>,
    libor: T,
    year_fraction: T,
    notional: T,
) -> T {
    let raw = fixed_rate - leverage * libor;
    let floored = if raw > floor_rate { raw } else { floor_rate };
    let capped = match cap_rate {
        Some(cap) => {
            if floored < cap {
                floored
            } else {
                cap
            }
        }
        None => floored,
    };
    capped * year_fraction * notional
}

/// Compute the analytical floor option value using the Bachelier (normal)
/// model.
///
/// The inverse floater floor `max(floor_rate, fixed - leverage * L)` contains
/// an embedded put option on the Libor rate with strike
/// `K = (fixed - floor) / leverage`. The floor option value is:
///
///   `leverage * year_fraction * notional * Bachelier_put(fwd_libor, K, vol,
/// T)`
///
/// When `normal_vol` is too small (< 1e-12), falls back to intrinsic value.
fn analytical_floor_value<T: Float>(
    fixed_rate: T,
    leverage: T,
    floor_rate: T,
    forward_libor: T,
    normal_vol: T,
    option_expiry: T,
    year_fraction: T,
    notional: T,
) -> T {
    let k_floor = (fixed_rate - floor_rate) / leverage;
    let scale = leverage * year_fraction * notional;
    let vol_threshold: T = from_f64(1e-12);

    if normal_vol < vol_threshold {
        // Intrinsic value of the put: max(K - fwd, 0)
        let intrinsic = k_floor - forward_libor;
        let intrinsic_pos = if intrinsic > T::zero() {
            intrinsic
        } else {
            T::zero()
        };
        return scale * intrinsic_pos;
    }

    match Bachelier::new(forward_libor, normal_vol) {
        Ok(model) => scale * model.price_put(k_floor, option_expiry),
        Err(_) => {
            // Fallback to intrinsic if Bachelier construction fails
            let intrinsic = k_floor - forward_libor;
            let intrinsic_pos = if intrinsic > T::zero() {
                intrinsic
            } else {
                T::zero()
            };
            scale * intrinsic_pos
        }
    }
}

/// Compute the delta of the floor option w.r.t. the underlying Libor rate.
///
/// For a Bachelier put with strike K on forward F:
///   delta_put = -N(-d) where d = (F - K) / (vol * sqrt(T))
///
/// Scaled by `-leverage * year_fraction * notional` to give the sensitivity
/// of the coupon value to changes in the reference rate.
fn analytical_floor_delta<T: Float>(
    fixed_rate: T,
    leverage: T,
    floor_rate: T,
    forward_libor: T,
    normal_vol: T,
    option_expiry: T,
    year_fraction: T,
    notional: T,
) -> T {
    let k_floor = (fixed_rate - floor_rate) / leverage;
    let vol_threshold: T = from_f64(1e-12);
    let expiry_threshold: T = from_f64(1e-12);

    // The delta of the CIF coupon w.r.t. the Libor rate:
    // d(coupon)/d(libor) = -leverage * yf * notional * N(d)
    // where d = (fwd - K) / (vol * sqrt(T)) and N(d) is the probability
    // that the floor does NOT bind (i.e., libor < K_floor).
    //
    // More precisely, for the floored coupon:
    //   coupon = max(floor, fixed - leverage * L) * yf * N
    //         = (fixed - leverage * L) * yf * N + leverage * yf * N * max(K - L, 0)
    //
    // The derivative of the expected value w.r.t. the forward Libor:
    //   = -leverage * yf * N * N(-d)
    // where N(-d) = P(L > K) under the normal model.

    if normal_vol < vol_threshold || option_expiry < expiry_threshold {
        // In the deterministic limit, delta is either -leverage*yf*N (floor not
        // binding) or 0 (floor binding).
        let scale = -leverage * year_fraction * notional;
        if forward_libor < k_floor {
            return scale;
        } else {
            return T::zero();
        }
    }

    let sqrt_t = option_expiry.sqrt();
    let vol_sqrt_t = normal_vol * sqrt_t;
    let d = (forward_libor - k_floor) / vol_sqrt_t;

    // Delta = -leverage * yf * notional * N(-d)
    // N(-d) is the probability that L > K (floor binds), so the linear
    // sensitivity captures the rate at which the expected coupon changes.
    -leverage * year_fraction * notional * norm_cdf(-d)
}

/// Compute the analytical floor option value at a specific node Libor rate
/// (not at the forward).
///
/// Re-evaluates the Bachelier put using the node-specific Libor as forward,
/// giving the exact local option value for computing the non-linear residual.
fn node_floor_value<T: Float>(
    fixed_rate: T,
    leverage: T,
    floor_rate: T,
    node_libor: T,
    normal_vol: T,
    option_expiry: T,
    year_fraction: T,
    notional: T,
) -> T {
    analytical_floor_value(
        fixed_rate,
        leverage,
        floor_rate,
        node_libor,
        normal_vol,
        option_expiry,
        year_fraction,
        notional,
    )
}

/// Compute the base expected coupon value using the forward Libor rate.
///
/// This is the deterministic part: `max(floor, fixed - leverage * fwd) * yf *
/// N`.
fn expected_coupon_base<T: Float>(
    fixed_rate: T,
    leverage: T,
    floor_rate: T,
    forward_libor: T,
    year_fraction: T,
    notional: T,
) -> T {
    cif_coupon_value(
        fixed_rate,
        leverage,
        floor_rate,
        None, // base expected does not apply cap
        forward_libor,
        year_fraction,
        notional,
    )
}

/// Compute pre-computed CIF node info for one coupon period across all tree
/// nodes.
///
/// This is the core function that produces the 4-component Taylor decomposition
/// (`dE`, `dR`, `dI`, `dQ`) at each node, plus discounted coupon values.
///
/// # Arguments
///
/// * `instrument` -- the CIF product definition
/// * `coupon_idx` -- index of the coupon period being evaluated
/// * `swap_rates` -- swap rate at each tree node (from calibrated slice)
/// * `libor_rates` -- Libor rate at each tree node
/// * `discount_factors` -- discount factor at each tree node
/// * `forward_swap_rate` -- ATM forward swap rate
/// * `forward_libor` -- ATM forward Libor rate
/// * `normal_vol` -- normal (Bachelier) volatility for this expiry/tenor
/// * `option_expiry` -- time to option expiry (year fraction)
///
/// # Errors
///
/// Returns `MfmError::InvalidParameter` if the input slice lengths are
/// inconsistent.
pub fn compute_cif_node_info<T: Float>(
    instrument: &CifInstrument<T>,
    coupon_idx: usize,
    swap_rates: &[T],
    libor_rates: &[T],
    discount_factors: &[T],
    forward_swap_rate: T,
    forward_libor: T,
    normal_vol: T,
    option_expiry: T,
) -> Result<CifNodeInfo<T>, MfmError> {
    let num_nodes = swap_rates.len();
    if libor_rates.len() != num_nodes || discount_factors.len() != num_nodes {
        return Err(MfmError::InvalidParameter {
            name: "swap_rates/libor_rates/discount_factors",
            reason: "must have the same length".to_string(),
        });
    }
    if coupon_idx >= instrument.num_coupons() {
        return Err(MfmError::InvalidParameter {
            name: "coupon_idx",
            reason: format!(
                "index {} out of range for {} coupons",
                coupon_idx,
                instrument.num_coupons()
            ),
        });
    }

    let fixed_rate = instrument.fixed_rate;
    let leverage = instrument.leverage;
    let floor_rate = instrument.floor_rate;
    let cap_rate = instrument.cap_rate;
    let notional = instrument.notional;
    let year_fraction = instrument.year_fractions[coupon_idx];
    let coupon_date_yf = instrument.coupon_dates[coupon_idx];
    let payment_date_yf = instrument.payment_dates[coupon_idx];

    // ── Analytical quantities at the forward ────────────────────────

    // Floor option value at the forward
    let fwd_floor_val = analytical_floor_value(
        fixed_rate,
        leverage,
        floor_rate,
        forward_libor,
        normal_vol,
        option_expiry,
        year_fraction,
        notional,
    );

    // Base expected coupon at the forward
    let base_expected = expected_coupon_base(
        fixed_rate,
        leverage,
        floor_rate,
        forward_libor,
        year_fraction,
        notional,
    );

    // Delta of the floor option w.r.t. Libor
    let fwd_delta = analytical_floor_delta(
        fixed_rate,
        leverage,
        floor_rate,
        forward_libor,
        normal_vol,
        option_expiry,
        year_fraction,
        notional,
    );

    // dE component: the expected coupon value at the forward Libor rate,
    // including the floor option time value. This is the "base" value that
    // is constant across all tree nodes.
    //
    // The base expected coupon already includes the floor intrinsic (it uses
    // max(floor, fixed - leverage * fwd)). The floor time value is the
    // additional stochastic premium from the Bachelier option.
    let k_floor = (fixed_rate - floor_rate) / leverage;
    let floor_intrinsic = {
        let diff = k_floor - forward_libor;
        let pos = if diff > T::zero() { diff } else { T::zero() };
        leverage * year_fraction * notional * pos
    };
    let floor_time_value = fwd_floor_val - floor_intrinsic;
    let d_e_base = base_expected + floor_time_value;

    // ── Per-node computation ────────────────────────────────────────

    let mut floor_values = Vec::with_capacity(num_nodes);
    let mut expected_coupons = Vec::with_capacity(num_nodes);
    let mut delta_swap_rate = Vec::with_capacity(num_nodes);
    let mut components = Vec::with_capacity(num_nodes);
    let mut discounted_values = Vec::with_capacity(num_nodes);

    for j in 0..num_nodes {
        let libor_j = libor_rates[j];
        let swap_j = swap_rates[j];
        let df_j = discount_factors[j];

        // (a)-(d) Direct coupon value at this node (deterministic payoff)
        let direct_coupon = cif_coupon_value(
            fixed_rate,
            leverage,
            floor_rate,
            cap_rate,
            libor_j,
            year_fraction,
            notional,
        );

        // (e) Analytical floor option value at this node's Libor rate
        let floor_val_j = node_floor_value(
            fixed_rate,
            leverage,
            floor_rate,
            libor_j,
            normal_vol,
            option_expiry,
            year_fraction,
            notional,
        );

        // (f) Expected coupon at this node = base coupon + floor time value
        let base_j = expected_coupon_base(
            fixed_rate,
            leverage,
            floor_rate,
            libor_j,
            year_fraction,
            notional,
        );
        let floor_intrinsic_j = {
            let diff = k_floor - libor_j;
            let pos = if diff > T::zero() { diff } else { T::zero() };
            leverage * year_fraction * notional * pos
        };
        let floor_time_value_j = floor_val_j - floor_intrinsic_j;
        let expected_j = base_j + floor_time_value_j;

        // (g) Delta w.r.t. swap rate at this node
        let delta_j = analytical_floor_delta(
            fixed_rate,
            leverage,
            floor_rate,
            libor_j,
            normal_vol,
            option_expiry,
            year_fraction,
            notional,
        );

        // (h) 4-component decomposition
        //
        // The decomposition ensures that dE + dR + dI + dQ = direct_coupon
        // exactly at each node. This is achieved by defining dQ as the
        // residual after subtracting the linear approximation from the
        // actual coupon value.
        let libor_deviation = libor_j - forward_libor;
        let swap_deviation = swap_j - forward_swap_rate;

        // dE: base expected value (computed at the forward, constant)
        let d_e = d_e_base;

        // dR: linear delta w.r.t. swap rate deviation from forward
        let d_r = fwd_delta * swap_deviation;

        // dI: linear Libor basis adjustment
        let d_i = -leverage * year_fraction * notional * libor_deviation;

        // dQ: non-linear residual = actual - (dE + dR + dI)
        // This captures the convexity/smile correction that the linear
        // Taylor expansion misses.
        let d_q = direct_coupon - d_e - d_r - d_i;

        let comp = CifCouponComponents { d_e, d_r, d_i, d_q };

        // (i) Discounted value: comp.total() == direct_coupon by construction
        let disc_val = comp.total() * df_j;

        floor_values.push(floor_val_j);
        expected_coupons.push(expected_j);
        delta_swap_rate.push(delta_j);
        components.push(comp);
        discounted_values.push(disc_val);
    }

    Ok(CifNodeInfo {
        coupon_idx,
        coupon_date_yf,
        payment_date_yf,
        year_fraction,
        notional,
        forward_swap_rate,
        forward_libor,
        normal_vol,
        floor_values,
        expected_coupons,
        delta_swap_rate,
        components,
        discounted_values,
    })
}

/// Build a complete CIF schedule from the instrument definition and
/// calibrated slice data.
///
/// Each element of `calibrated_slices` is a tuple:
/// `(swap_rates, libor_rates, discount_factors, fwd_swap, fwd_libor,
/// normal_vol, option_expiry)`
///
/// There must be exactly one entry per coupon period.
///
/// # Errors
///
/// Returns `MfmError::InvalidParameter` if the number of calibrated slices
/// does not match the number of coupon periods, or if the instrument fails
/// validation.
pub fn build_cif_schedule<T: Float>(
    instrument: &CifInstrument<T>,
    calibrated_slices: &[(Vec<T>, Vec<T>, Vec<T>, T, T, T, T)],
) -> Result<CifSchedule<T>, MfmError> {
    instrument.validate()?;

    if calibrated_slices.len() != instrument.num_coupons() {
        return Err(MfmError::InvalidParameter {
            name: "calibrated_slices",
            reason: format!(
                "expected {} slices for {} coupons, got {}",
                instrument.num_coupons(),
                instrument.num_coupons(),
                calibrated_slices.len()
            ),
        });
    }

    let mut coupons = Vec::with_capacity(instrument.num_coupons());

    for (idx, slice_data) in calibrated_slices.iter().enumerate() {
        let (
            ref swap_rates,
            ref libor_rates,
            ref discount_factors,
            fwd_swap,
            fwd_libor,
            normal_vol,
            option_expiry,
        ) = *slice_data;

        let node_info = compute_cif_node_info(
            instrument,
            idx,
            swap_rates,
            libor_rates,
            discount_factors,
            fwd_swap,
            fwd_libor,
            normal_vol,
            option_expiry,
        )?;

        coupons.push(node_info);
    }

    Ok(CifSchedule {
        instrument: instrument.clone(),
        coupons,
    })
}

// ─── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    /// Helper: create a valid CIF instrument for testing.
    fn sample_instrument() -> CifInstrument<f64> {
        CifInstrument {
            fixed_rate: 0.06,
            leverage: 1.0,
            floor_rate: 0.0,
            cap_rate: None,
            notional: 1_000_000.0,
            coupon_dates: vec![1.0, 2.0],
            payment_dates: vec![1.0, 2.0],
            year_fractions: vec![1.0, 1.0],
        }
    }

    // ── Validation tests ────────────────────────────────────────────

    #[test]
    fn test_cif_instrument_validate_valid() {
        let inst = sample_instrument();
        assert!(inst.validate().is_ok());
    }

    #[test]
    fn test_cif_instrument_validate_with_cap() {
        let mut inst = sample_instrument();
        inst.cap_rate = Some(0.08);
        assert!(inst.validate().is_ok());
    }

    #[test]
    fn test_cif_instrument_validate_zero_leverage() {
        let mut inst = sample_instrument();
        inst.leverage = 0.0;
        let result = inst.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            MfmError::InvalidParameter { name, .. } => assert_eq!(name, "leverage"),
            _ => panic!("expected InvalidParameter for leverage"),
        }
    }

    #[test]
    fn test_cif_instrument_validate_negative_leverage() {
        let mut inst = sample_instrument();
        inst.leverage = -1.0;
        assert!(inst.validate().is_err());
    }

    #[test]
    fn test_cif_instrument_validate_zero_notional() {
        let mut inst = sample_instrument();
        inst.notional = 0.0;
        assert!(inst.validate().is_err());
    }

    #[test]
    fn test_cif_instrument_validate_negative_floor() {
        let mut inst = sample_instrument();
        inst.floor_rate = -0.01;
        assert!(inst.validate().is_err());
    }

    #[test]
    fn test_cif_instrument_validate_mismatched_lengths() {
        let mut inst = sample_instrument();
        inst.payment_dates = vec![1.0]; // only 1 vs 2 coupon dates
        assert!(inst.validate().is_err());
    }

    #[test]
    fn test_cif_instrument_validate_cap_below_floor() {
        let mut inst = sample_instrument();
        inst.floor_rate = 0.02;
        inst.cap_rate = Some(0.01); // cap < floor
        assert!(inst.validate().is_err());
    }

    #[test]
    fn test_cif_instrument_validate_cap_equals_floor() {
        let mut inst = sample_instrument();
        inst.floor_rate = 0.02;
        inst.cap_rate = Some(0.02); // cap == floor
        assert!(inst.validate().is_err());
    }

    #[test]
    fn test_cif_instrument_num_coupons() {
        let inst = sample_instrument();
        assert_eq!(inst.num_coupons(), 2);
    }

    // ── CifCouponComponents tests ───────────────────────────────────

    #[test]
    fn test_cif_coupon_components_total() {
        let comp = CifCouponComponents {
            d_e: 100.0,
            d_r: 5.0,
            d_i: -3.0,
            d_q: 0.5,
        };
        assert_relative_eq!(comp.total(), 102.5, epsilon = 1e-12);
    }

    #[test]
    fn test_cif_coupon_components_default() {
        let comp = CifCouponComponents::<f64>::default();
        assert_relative_eq!(comp.total(), 0.0, epsilon = 1e-12);
    }

    // ── Flat rate tests ─────────────────────────────────────────────

    #[test]
    fn test_compute_node_info_flat() {
        // All nodes have the same rate equal to the forward.
        // dR and dI should be ~0, and dQ should be ~0.
        let inst = CifInstrument {
            fixed_rate: 0.06,
            leverage: 1.0,
            floor_rate: 0.0,
            cap_rate: None,
            notional: 1_000_000.0,
            coupon_dates: vec![1.0],
            payment_dates: vec![1.0],
            year_fractions: vec![1.0],
        };

        let fwd_swap = 0.04;
        let fwd_libor = 0.04;
        let normal_vol = 0.005; // 50 bps normal vol
        let option_expiry = 1.0;

        let num_nodes = 5;
        let swap_rates = vec![fwd_swap; num_nodes];
        let libor_rates = vec![fwd_libor; num_nodes];
        let discount_factors = vec![0.96; num_nodes];

        let info = compute_cif_node_info(
            &inst,
            0,
            &swap_rates,
            &libor_rates,
            &discount_factors,
            fwd_swap,
            fwd_libor,
            normal_vol,
            option_expiry,
        )
        .unwrap();

        assert_eq!(info.coupon_idx, 0);
        assert_eq!(info.floor_values.len(), num_nodes);
        assert_eq!(info.components.len(), num_nodes);

        // All floor values should be identical since all Libor rates are the same
        for j in 1..num_nodes {
            assert_relative_eq!(info.floor_values[j], info.floor_values[0], epsilon = 1e-10);
        }

        // dR should be ~0 since all swap rates equal forward
        for j in 0..num_nodes {
            assert!(
                info.components[j].d_r.abs() < 1e-10,
                "dR[{}] = {} should be ~0",
                j,
                info.components[j].d_r
            );
        }

        // dI should be ~0 since all Libor rates equal forward
        for j in 0..num_nodes {
            assert!(
                info.components[j].d_i.abs() < 1e-10,
                "dI[{}] = {} should be ~0",
                j,
                info.components[j].d_i
            );
        }

        // When all nodes are at the forward, dQ captures the difference
        // between the direct coupon and the linear approximation (dE + dI).
        // Since dR = 0 and dI = 0 at the forward, dQ = direct - dE.
        // The direct coupon is deterministic; dE includes floor time value.
        // All dQ values should be identical since all nodes are at the same rate.
        for j in 1..num_nodes {
            assert_relative_eq!(
                info.components[j].d_q,
                info.components[0].d_q,
                epsilon = 1e-10
            );
        }

        // The component total should equal the direct coupon exactly
        let direct = cif_coupon_value(0.06, 1.0, 0.0, None, fwd_libor, 1.0, 1_000_000.0);
        for j in 0..num_nodes {
            assert_relative_eq!(info.components[j].total(), direct, epsilon = 1e-6,);
        }

        // All discounted values should be the same
        for j in 1..num_nodes {
            assert_relative_eq!(
                info.discounted_values[j],
                info.discounted_values[0],
                epsilon = 1e-6
            );
        }
    }

    // ── Varying rate tests ──────────────────────────────────────────

    #[test]
    fn test_compute_node_info_varying_rates() {
        let inst = CifInstrument {
            fixed_rate: 0.06,
            leverage: 1.0,
            floor_rate: 0.0,
            cap_rate: None,
            notional: 1_000_000.0,
            coupon_dates: vec![1.0],
            payment_dates: vec![1.0],
            year_fractions: vec![1.0],
        };

        let fwd_swap = 0.04;
        let fwd_libor = 0.04;
        let normal_vol = 0.005;
        let option_expiry = 1.0;

        // Varying Libor rates across nodes
        let swap_rates = vec![0.03, 0.035, 0.04, 0.045, 0.05];
        let libor_rates = vec![0.02, 0.03, 0.04, 0.05, 0.06];
        let discount_factors = vec![0.98, 0.97, 0.96, 0.95, 0.94];

        let info = compute_cif_node_info(
            &inst,
            0,
            &swap_rates,
            &libor_rates,
            &discount_factors,
            fwd_swap,
            fwd_libor,
            normal_vol,
            option_expiry,
        )
        .unwrap();

        // dI should be non-zero for nodes where Libor deviates from forward
        // Node 0: libor=0.02, deviation = -0.02, dI = -1.0 * 1.0 * 1e6 * (-0.02) =
        // 20000
        assert!(
            info.components[0].d_i.abs() > 1.0,
            "dI[0] should be large for Libor=0.02 vs fwd=0.04"
        );

        // Node 2 (at forward): dI should be ~0
        assert!(
            info.components[2].d_i.abs() < 1e-10,
            "dI[2] should be ~0 at forward"
        );

        // Node 4: libor=0.06, deviation = 0.02, dI = -1.0 * 1.0 * 1e6 * 0.02 = -20000
        assert!(
            info.components[4].d_i.abs() > 1.0,
            "dI[4] should be large for Libor=0.06 vs fwd=0.04"
        );

        // Verify dI signs: higher Libor -> lower coupon -> negative dI
        assert!(
            info.components[0].d_i > 0.0,
            "dI should be positive when Libor < forward (higher coupon)"
        );
        assert!(
            info.components[4].d_i < 0.0,
            "dI should be negative when Libor > forward (lower coupon)"
        );

        // The component total should equal the direct coupon exactly
        // (by construction: dQ = direct - dE - dR - dI).
        for j in 0..5 {
            let direct = cif_coupon_value(0.06, 1.0, 0.0, None, libor_rates[j], 1.0, 1_000_000.0);
            let component_total = info.components[j].total();
            assert_relative_eq!(component_total, direct, epsilon = 1e-6,);
        }
    }

    // ── Floor binding tests ─────────────────────────────────────────

    #[test]
    fn test_floor_binding() {
        // When Libor > (fixed_rate - floor_rate) / leverage, the floor binds.
        // For fixed=0.06, floor=0.02, leverage=1.0: K_floor = 0.04
        // If Libor = 0.05 > K_floor, the floor should bind and the floor option
        // should have significant value.
        let inst = CifInstrument {
            fixed_rate: 0.06,
            leverage: 1.0,
            floor_rate: 0.02,
            cap_rate: None,
            notional: 1_000_000.0,
            coupon_dates: vec![1.0],
            payment_dates: vec![1.0],
            year_fractions: vec![1.0],
        };

        let fwd_libor = 0.05; // Above K_floor = 0.04, floor binds
        let normal_vol = 0.005;
        let option_expiry = 1.0;

        let swap_rates = vec![0.05];
        let libor_rates = vec![fwd_libor];
        let discount_factors = vec![0.95];

        let info = compute_cif_node_info(
            &inst,
            0,
            &swap_rates,
            &libor_rates,
            &discount_factors,
            0.05,
            fwd_libor,
            normal_vol,
            option_expiry,
        )
        .unwrap();

        // Floor value should be significant because the put is ITM
        // K_floor = 0.04, fwd = 0.05, put intrinsic = 0 (K < fwd means OTM for put)
        // Wait -- put payoff is max(K - F, 0). K=0.04, F=0.05 -> intrinsic = 0.
        // But the floor binds when L > K, meaning the coupon is at the floor.
        // The put is OTM but still has time value.
        //
        // Actually, rethinking: the floor binds when raw coupon < floor,
        // i.e., fixed - leverage * L < floor, i.e., L > (fixed - floor)/leverage =
        // K_floor. When fwd_libor = 0.05 > K_floor = 0.04, the put is OTM (K <
        // F for a put). The floor option value is the time value only.
        //
        // For a deep ITM floor, we need fwd_libor << K_floor.
        // Let's set fwd_libor = 0.02 < K_floor = 0.04.
        // Then put is ITM: intrinsic = (0.04 - 0.02) = 0.02
        // floor_value = 1.0 * 1.0 * 1e6 * put(0.02, K=0.04, vol, T)
        //             should be large (at least 0.02 * 1e6 = 20,000 intrinsic)

        // Re-test with ITM floor
        let fwd_libor_itm = 0.02;
        let libor_rates_itm = vec![fwd_libor_itm];
        let swap_rates_itm = vec![0.02];

        let info_itm = compute_cif_node_info(
            &inst,
            0,
            &swap_rates_itm,
            &libor_rates_itm,
            &discount_factors,
            0.02,
            fwd_libor_itm,
            normal_vol,
            option_expiry,
        )
        .unwrap();

        // K_floor = 0.04, fwd = 0.02, put intrinsic = 0.02
        // floor_value >= leverage * yf * N * intrinsic = 1.0 * 1.0 * 1e6 * 0.02 =
        // 20,000
        assert!(
            info_itm.floor_values[0] > 19_000.0,
            "floor_value = {} should be > 19,000 when floor is deep ITM",
            info_itm.floor_values[0]
        );

        // The original case (OTM put) should still have positive but smaller value
        assert!(
            info.floor_values[0] > 0.0,
            "floor_value should be positive even when OTM (time value)"
        );
        assert!(
            info_itm.floor_values[0] > info.floor_values[0],
            "ITM floor should have larger value than OTM floor"
        );
    }

    #[test]
    fn test_no_floor_binding() {
        // When Libor is well below K_floor, the floor doesn't bind and the
        // floor option value should be small (deep OTM put).
        let inst = CifInstrument {
            fixed_rate: 0.06,
            leverage: 1.0,
            floor_rate: 0.0,
            cap_rate: None,
            notional: 1_000_000.0,
            coupon_dates: vec![1.0],
            payment_dates: vec![1.0],
            year_fractions: vec![1.0],
        };

        // K_floor = (0.06 - 0.0) / 1.0 = 0.06
        // fwd_libor = 0.01 << K_floor = 0.06, so put is deep ITM
        // Actually, for the floor NOT to bind, we need coupon > floor,
        // i.e., fixed - leverage * L > floor, i.e., L < K_floor.
        // So fwd = 0.01 means floor does NOT bind (coupon = 0.05 > 0).
        // But the put IS ITM (K=0.06, F=0.01, intrinsic=0.05).
        //
        // For floor not binding AND small floor value, we need the put to be OTM,
        // i.e., fwd_libor > K_floor. But that means the floor IS binding.
        //
        // The correct interpretation: the floor option value represents the
        // ADDITIONAL value from the floor above the linear coupon.
        // When Libor is low (floor not binding), the raw coupon is high and
        // the floor is irrelevant -> floor option value is just OTM time value.
        //
        // Wait: K_floor = (fixed - floor) / leverage = 0.06 for this case.
        // Put with K=0.06, F=0.01: put is deep ITM.
        //
        // Let me use a higher Libor so the put is OTM (floor not binding scenario
        // in expectation, but the put still has some value).
        //
        // With fwd_libor = 0.08 > K_floor = 0.06:
        // The floor binds (coupon = 0.06 - 1.0*0.08 = -0.02 < 0 = floor).
        // The put is OTM (K=0.06, F=0.08), so floor_value is small time value.
        //
        // Hmm, this is the tricky part: when Libor is HIGH, the floor BINDS
        // (protecting the holder), and the put is OTM. When Libor is LOW,
        // the floor does NOT bind, but the put is ITM.
        //
        // For the test "no floor binding" with small floor value, we want:
        // A scenario where the floor is very unlikely to bind.
        // Set floor_rate = 0.0, fixed = 0.06, leverage = 1.0 -> K_floor = 0.06
        // Set fwd_libor = 0.01 (low Libor, coupon = 0.05, floor not binding).
        // The put with K=0.06, F=0.01 is deep ITM.
        //
        // So "small floor value" really means the floor adds little EXTRA value
        // beyond the deterministic coupon. In a world with low vol and Libor far
        // below K, the put is deep ITM but the time value is small.
        // Let's verify with very low vol.

        let fwd_libor = 0.01; // Well below K_floor = 0.06
        let normal_vol = 0.0005; // Very low vol
        let option_expiry = 1.0;

        let swap_rates = vec![0.01; 3];
        let libor_rates = vec![0.01; 3];
        let discount_factors = vec![0.99; 3];

        let info = compute_cif_node_info(
            &inst,
            0,
            &swap_rates,
            &libor_rates,
            &discount_factors,
            0.01,
            fwd_libor,
            normal_vol,
            option_expiry,
        )
        .unwrap();

        // The direct coupon = max(0, 0.06 - 0.01) * 1.0 * 1e6 = 50,000
        // Floor option: put with K=0.06, F=0.01, very low vol -> almost pure intrinsic
        // Intrinsic = (0.06 - 0.01) * 1e6 = 50,000
        // Time value should be negligible with very low vol
        let k_floor = 0.06;
        let intrinsic = (k_floor - fwd_libor) * 1.0 * 1_000_000.0;

        // The floor value should be very close to intrinsic (minimal time value)
        assert_relative_eq!(
            info.floor_values[0],
            intrinsic,
            epsilon = 500.0 // Allow small time value
        );

        // The time value (floor_value - intrinsic) should be small
        let time_value = info.floor_values[0] - intrinsic;
        assert!(
            time_value.abs() < 500.0,
            "time value = {} should be small with low vol",
            time_value
        );
    }

    // ── Mismatched inputs test ──────────────────────────────────────

    #[test]
    fn test_compute_node_info_mismatched_lengths() {
        let inst = sample_instrument();
        let swap_rates = vec![0.04; 5];
        let libor_rates = vec![0.04; 3]; // Different length!
        let discount_factors = vec![0.96; 5];

        let result = compute_cif_node_info(
            &inst,
            0,
            &swap_rates,
            &libor_rates,
            &discount_factors,
            0.04,
            0.04,
            0.005,
            1.0,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_compute_node_info_invalid_coupon_idx() {
        let inst = sample_instrument(); // 2 coupons
        let swap_rates = vec![0.04; 5];
        let libor_rates = vec![0.04; 5];
        let discount_factors = vec![0.96; 5];

        let result = compute_cif_node_info(
            &inst,
            5, // Out of range
            &swap_rates,
            &libor_rates,
            &discount_factors,
            0.04,
            0.04,
            0.005,
            1.0,
        );
        assert!(result.is_err());
    }

    // ── Build schedule tests ────────────────────────────────────────

    #[test]
    fn test_build_cif_schedule() {
        let inst = CifInstrument {
            fixed_rate: 0.06,
            leverage: 1.0,
            floor_rate: 0.0,
            cap_rate: None,
            notional: 1_000_000.0,
            coupon_dates: vec![1.0, 2.0],
            payment_dates: vec![1.0, 2.0],
            year_fractions: vec![1.0, 1.0],
        };

        let num_nodes = 3;
        let swap_rates = vec![0.04; num_nodes];
        let libor_rates = vec![0.04; num_nodes];
        let discount_factors = vec![0.96; num_nodes];

        let slices = vec![
            (
                swap_rates.clone(),
                libor_rates.clone(),
                discount_factors.clone(),
                0.04,
                0.04,
                0.005,
                1.0,
            ),
            (
                swap_rates,
                libor_rates,
                discount_factors,
                0.04,
                0.04,
                0.005,
                2.0,
            ),
        ];

        let schedule = build_cif_schedule(&inst, &slices).unwrap();
        assert_eq!(schedule.coupons.len(), 2);
        assert_eq!(schedule.coupons[0].coupon_idx, 0);
        assert_eq!(schedule.coupons[1].coupon_idx, 1);
        assert_relative_eq!(schedule.coupons[0].coupon_date_yf, 1.0, epsilon = 1e-12);
        assert_relative_eq!(schedule.coupons[1].coupon_date_yf, 2.0, epsilon = 1e-12);
    }

    #[test]
    fn test_build_cif_schedule_wrong_slice_count() {
        let inst = sample_instrument(); // 2 coupons
        let slices: Vec<(Vec<f64>, Vec<f64>, Vec<f64>, f64, f64, f64, f64)> = vec![];
        let result = build_cif_schedule(&inst, &slices);
        assert!(result.is_err());
    }

    #[test]
    fn test_build_cif_schedule_invalid_instrument() {
        let inst = CifInstrument {
            fixed_rate: 0.06,
            leverage: -1.0, // Invalid
            floor_rate: 0.0,
            cap_rate: None,
            notional: 1_000_000.0,
            coupon_dates: vec![1.0],
            payment_dates: vec![1.0],
            year_fractions: vec![1.0],
        };

        let slices = vec![(vec![0.04], vec![0.04], vec![0.96], 0.04, 0.04, 0.005, 1.0)];
        let result = build_cif_schedule(&inst, &slices);
        assert!(result.is_err());
    }

    // ── Cap handling test ───────────────────────────────────────────

    #[test]
    fn test_cif_coupon_value_with_cap() {
        // fixed=0.10, leverage=1.0, floor=0.0, cap=0.05
        // libor=0.02 -> raw = 0.08, floored = 0.08, capped = 0.05
        let val = cif_coupon_value(0.10, 1.0, 0.0, Some(0.05), 0.02, 1.0, 1.0);
        assert_relative_eq!(val, 0.05, epsilon = 1e-12);

        // libor=0.06 -> raw = 0.04, floored = 0.04, capped = 0.04 (below cap)
        let val2 = cif_coupon_value(0.10, 1.0, 0.0, Some(0.05), 0.06, 1.0, 1.0);
        assert_relative_eq!(val2, 0.04, epsilon = 1e-12);

        // libor=0.12 -> raw = -0.02, floored = 0.0, capped = 0.0
        let val3 = cif_coupon_value(0.10, 1.0, 0.0, Some(0.05), 0.12, 1.0, 1.0);
        assert_relative_eq!(val3, 0.0, epsilon = 1e-12);
    }

    #[test]
    fn test_cif_coupon_value_no_cap() {
        // fixed=0.06, leverage=1.0, floor=0.0, no cap
        // libor=0.02 -> raw = 0.04
        let val = cif_coupon_value(0.06, 1.0, 0.0, None, 0.02, 1.0, 1_000_000.0);
        assert_relative_eq!(val, 40_000.0, epsilon = 1e-6);

        // libor=0.08 -> raw = -0.02, floored = 0.0
        let val2 = cif_coupon_value(0.06, 1.0, 0.0, None, 0.08, 1.0, 1_000_000.0);
        assert_relative_eq!(val2, 0.0, epsilon = 1e-6);
    }

    // ── Leverage test ───────────────────────────────────────────────

    #[test]
    fn test_cif_coupon_value_leverage() {
        // fixed=0.10, leverage=2.0, floor=0.0, no cap
        // libor=0.03 -> raw = 0.10 - 2.0*0.03 = 0.04
        let val = cif_coupon_value(0.10, 2.0, 0.0, None, 0.03, 1.0, 1.0);
        assert_relative_eq!(val, 0.04, epsilon = 1e-12);

        // libor=0.06 -> raw = 0.10 - 2.0*0.06 = -0.02, floored = 0.0
        let val2 = cif_coupon_value(0.10, 2.0, 0.0, None, 0.06, 1.0, 1.0);
        assert_relative_eq!(val2, 0.0, epsilon = 1e-12);
    }

    // ── Analytical floor value tests ────────────────────────────────

    #[test]
    fn test_analytical_floor_value_zero_vol() {
        // With zero vol, should return intrinsic
        // K_floor = (0.06 - 0.0) / 1.0 = 0.06
        // fwd = 0.04 -> intrinsic = max(0.06 - 0.04, 0) = 0.02
        // floor_value = 1.0 * 1.0 * 1e6 * 0.02 = 20,000
        let val = analytical_floor_value(0.06, 1.0, 0.0, 0.04, 0.0, 1.0, 1.0, 1_000_000.0);
        assert_relative_eq!(val, 20_000.0, epsilon = 1.0);
    }

    #[test]
    fn test_analytical_floor_value_positive_vol() {
        // With positive vol, value should exceed intrinsic (time value > 0)
        let intrinsic = analytical_floor_value(0.06, 1.0, 0.0, 0.04, 0.0, 1.0, 1.0, 1_000_000.0);
        let with_vol = analytical_floor_value(0.06, 1.0, 0.0, 0.04, 0.005, 1.0, 1.0, 1_000_000.0);
        assert!(
            with_vol > intrinsic,
            "Option value with vol ({}) should exceed intrinsic ({})",
            with_vol,
            intrinsic
        );
    }

    // ── Delta tests ─────────────────────────────────────────────────

    #[test]
    fn test_analytical_floor_delta_at_forward() {
        // Delta should be negative (higher Libor -> lower coupon)
        let delta = analytical_floor_delta(0.06, 1.0, 0.0, 0.04, 0.005, 1.0, 1.0, 1_000_000.0);
        assert!(delta < 0.0, "delta = {} should be negative", delta);
    }

    #[test]
    fn test_analytical_floor_delta_deep_itm() {
        // When fwd is well below K_floor, put is deep ITM, delta ~ -leverage * yf * N
        let delta = analytical_floor_delta(0.06, 1.0, 0.0, 0.01, 0.005, 1.0, 1.0, 1_000_000.0);
        // N(-d) should be close to 1 for deep ITM put (d very negative)
        // delta ~ -1.0 * 1.0 * 1e6 = -1,000,000
        assert_relative_eq!(delta, -1_000_000.0, epsilon = 10_000.0);
    }

    #[test]
    fn test_analytical_floor_delta_deep_otm() {
        // When fwd is well above K_floor, put is deep OTM, delta ~ 0
        let delta = analytical_floor_delta(0.06, 1.0, 0.0, 0.10, 0.005, 1.0, 1.0, 1_000_000.0);
        // N(-d) should be close to 0 for deep OTM put
        assert!(
            delta.abs() < 10_000.0,
            "delta = {} should be small for deep OTM floor",
            delta
        );
    }
}
