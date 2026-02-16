//! Yield curve traits, implementations, and calibration instruments.

use enum_dispatch::enum_dispatch;
use num_traits::Float;
use pricer_core::math::numeric::from_f64;

use super::MarketDataError;

/// Trait for yield curves providing discount factors and rates.
#[enum_dispatch]
pub trait YieldCurve<T: Float> {
    /// Returns the discount factor for time `t` (in years).
    fn discount_factor(&self, t: T) -> Result<T, MarketDataError>;

    /// Returns the continuously compounded zero rate for time `t`.
    fn zero_rate(&self, t: T) -> Result<T, MarketDataError> {
        let df = self.discount_factor(t)?;
        if t <= T::zero() {
            return Ok(T::zero());
        }
        Ok(-df.ln() / t)
    }

    /// Returns the forward rate between times `t1` and `t2`.
    fn forward_rate(&self, t1: T, t2: T) -> Result<T, MarketDataError> {
        let df1 = if t1 <= T::zero() {
            T::one()
        } else {
            self.discount_factor(t1)?
        };
        let df2 = self.discount_factor(t2)?;
        let tau = t2 - t1;

        if tau <= T::zero() {
            return Err(MarketDataError::InvalidInput {
                message: "t2 must be greater than t1".to_string(),
            });
        }

        Ok((df1 / df2 - T::one()) / tau)
    }
}

/// A flat yield curve with constant continuously compounded rate.
#[derive(Debug, Clone, Copy)]
pub struct FlatCurve<T: Float> {
    rate: T,
}

impl<T: Float> FlatCurve<T> {
    /// Creates a new flat curve with the given rate.
    pub fn new(rate: T) -> Self { Self { rate } }

    /// Returns the constant rate.
    pub fn rate(&self) -> T { self.rate }
}

impl<T: Float> YieldCurve<T> for FlatCurve<T> {
    fn discount_factor(&self, t: T) -> Result<T, MarketDataError> {
        if t < T::zero() {
            return Err(MarketDataError::InvalidInput {
                message: "time cannot be negative".to_string(),
            });
        }
        Ok((-self.rate * t).exp())
    }

    fn zero_rate(&self, _t: T) -> Result<T, MarketDataError> { Ok(self.rate) }

    fn forward_rate(&self, t1: T, t2: T) -> Result<T, MarketDataError> {
        if t2 <= t1 {
            return Err(MarketDataError::InvalidInput {
                message: "t2 must be greater than t1".to_string(),
            });
        }
        Ok(self.rate)
    }
}

/// Payment frequency for financial instruments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Frequency {
    /// Daily payments
    Daily,
    /// Weekly payments
    Weekly,
    /// Monthly payments
    #[default]
    Monthly,
    /// Quarterly payments (4 per year)
    Quarterly,
    /// Semi-annual payments (2 per year)
    SemiAnnual,
    /// Annual payments (1 per year)
    Annual,
}

impl Frequency {
    /// Returns the period length in years.
    pub fn period_years<T: Float>(&self) -> T {
        match self {
            Frequency::Daily => from_f64(1.0 / 365.0),
            Frequency::Weekly => from_f64(1.0 / 52.0),
            Frequency::Monthly => from_f64(1.0 / 12.0),
            Frequency::Quarterly => from_f64(0.25),
            Frequency::SemiAnnual => from_f64(0.5),
            Frequency::Annual => from_f64(1.0),
        }
    }
}

/// Interpolation method for bootstrapped curves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BootstrapInterpolation {
    /// Linear interpolation on discount factors.
    #[serde(rename = "linear_df", alias = "linear")]
    Linear,
    /// Log-linear interpolation (linear on log of discount factors).
    #[default]
    #[serde(rename = "log_linear_df", alias = "log_linear")]
    LogLinear,
    /// Flat forward interpolation (constant simple forward rate between pillars).
    FlatForward,
    /// Cubic spline on forward rates (not yet implemented, falls back to LogLinear).
    CubicSplineFwd,
    /// Monotone convex (Hagan-West) (not yet implemented, falls back to LogLinear).
    MonotoneConvex,
    /// Log-cubic on discount factors (not yet implemented, falls back to LogLinear).
    #[serde(rename = "log_cubic_df")]
    LogCubicDF,
    /// Tension spline on forward rates (not yet implemented, falls back to LogLinear).
    TensionSpline,
}

impl BootstrapInterpolation {
    /// Returns true if the interpolation method requires precomputed spline coefficients.
    pub fn requires_spline_coefficients(self) -> bool {
        matches!(
            self,
            Self::CubicSplineFwd
                | Self::MonotoneConvex
                | Self::LogCubicDF
                | Self::TensionSpline
        )
    }
}

/// Market instrument for yield curve calibration.
#[derive(Debug, Clone)]
pub enum MarketInstrument<T: Float> {
    /// Overnight Index Swap
    Ois {
        /// Maturity in years
        maturity: T,
        /// Market-quoted rate
        rate: T,
        /// Payment frequency
        payment_frequency: Frequency,
    },
    /// Interest Rate Swap
    Irs {
        /// Maturity in years
        maturity: T,
        /// Market-quoted rate
        rate: T,
        /// Fixed leg payment frequency
        fixed_frequency: Frequency,
    },
    /// Forward Rate Agreement
    Fra {
        /// Start time in years
        start: T,
        /// End time in years
        end: T,
        /// Market-quoted rate
        rate: T,
    },
    /// Interest Rate Future
    Future {
        /// Maturity in years
        maturity: T,
        /// Market-quoted rate (derived from price)
        rate: T,
        /// Convexity adjustment
        convexity_adjustment: T,
    },
    /// Central bank meeting or scheduled event (rate jump)
    Event {
        /// Time to event date in years
        maturity: T,
        /// Expected rate jump (absolute rate, not bps)
        expected_jump: T,
    },
    /// Fixed-coupon bond (government or corporate)
    Bond {
        /// Maturity in years
        maturity: T,
        /// Market-quoted yield-to-maturity
        rate: T,
        /// Fixed coupon rate (annual equivalent)
        coupon_rate: T,
        /// Coupon payment frequency
        payment_frequency: Frequency,
    },
    /// Credit Default Swap for credit curve calibration.
    ///
    /// The "rate" is the CDS par spread; the curve being bootstrapped is
    /// a survival probability curve (discount_factor = survival_probability).
    Cds {
        /// Maturity in years
        maturity: T,
        /// CDS par spread (market-quoted, as decimal e.g. 0.01 = 100bp)
        spread: T,
        /// Recovery rate (e.g. 0.40)
        recovery_rate: T,
        /// Pre-sampled risk-free discount factors at quarterly intervals.
        /// Stored as (time, df) pairs to avoid runtime curve dependency.
        risk_free_dfs: Vec<(T, T)>,
    },
}

impl<T: Float> MarketInstrument<T> {
    /// Creates an OIS instrument.
    pub fn ois(maturity: T, rate: T) -> Self {
        Self::Ois {
            maturity,
            rate,
            payment_frequency: Frequency::Annual,
        }
    }

    /// Creates an IRS instrument.
    pub fn irs(maturity: T, rate: T) -> Self {
        Self::Irs {
            maturity,
            rate,
            fixed_frequency: Frequency::SemiAnnual,
        }
    }

    /// Creates a FRA instrument.
    pub fn fra(start: T, end: T, rate: T) -> Self { Self::Fra { start, end, rate } }

    /// Creates a Future instrument.
    pub fn future(maturity: T, rate: T) -> Self {
        Self::Future {
            maturity,
            rate,
            convexity_adjustment: T::zero(),
        }
    }

    /// Creates an Event instrument (rate jump in basis points).
    pub fn event(maturity: T, expected_jump_bps: T) -> Self {
        // Convert basis points to absolute rate
        let expected_jump = expected_jump_bps * from_f64::<T>(0.0001);
        Self::Event {
            maturity,
            expected_jump,
        }
    }

    /// Creates an Event instrument with absolute rate jump.
    pub fn event_with_rate(maturity: T, expected_jump: T) -> Self {
        Self::Event {
            maturity,
            expected_jump,
        }
    }

    /// Creates a Bond instrument with semi-annual coupons (default for
    /// government bonds).
    pub fn bond(maturity: T, ytm: T, coupon_rate: T) -> Self {
        Self::Bond {
            maturity,
            rate: ytm,
            coupon_rate,
            payment_frequency: Frequency::SemiAnnual,
        }
    }

    /// Creates a Bond instrument with explicit coupon frequency.
    pub fn bond_with_frequency(
        maturity: T,
        ytm: T,
        coupon_rate: T,
        payment_frequency: Frequency,
    ) -> Self {
        Self::Bond {
            maturity,
            rate: ytm,
            coupon_rate,
            payment_frequency,
        }
    }

    /// Creates a CDS instrument with pre-sampled risk-free discount factors.
    pub fn cds(maturity: T, spread: T, recovery_rate: T, risk_free_dfs: Vec<(T, T)>) -> Self {
        Self::Cds {
            maturity,
            spread,
            recovery_rate,
            risk_free_dfs,
        }
    }

    /// Returns the market-quoted rate (or expected jump for Event).
    pub fn rate(&self) -> T {
        match self {
            Self::Ois { rate, .. } => *rate,
            Self::Irs { rate, .. } => *rate,
            Self::Fra { rate, .. } => *rate,
            Self::Future { rate, .. } => *rate,
            Self::Event { expected_jump, .. } => *expected_jump,
            Self::Bond { rate, .. } => *rate,
            Self::Cds { spread, .. } => *spread,
        }
    }

    /// Returns the instrument's maturity.
    pub fn maturity(&self) -> T {
        match self {
            Self::Ois { maturity, .. } => *maturity,
            Self::Irs { maturity, .. } => *maturity,
            Self::Fra { end, .. } => *end,
            Self::Future { maturity, .. } => *maturity,
            Self::Event { maturity, .. } => *maturity,
            Self::Bond { maturity, .. } => *maturity,
            Self::Cds { maturity, .. } => *maturity,
        }
    }

    /// Returns a descriptive name for the instrument type.
    pub fn instrument_type(&self) -> &'static str {
        match self {
            Self::Ois { .. } => "OIS",
            Self::Irs { .. } => "IRS",
            Self::Fra { .. } => "FRA",
            Self::Future { .. } => "Future",
            Self::Event { .. } => "Event",
            Self::Bond { .. } => "Bond",
            Self::Cds { .. } => "CDS",
        }
    }

    /// Returns true if this is an Event instrument.
    pub fn is_event(&self) -> bool { matches!(self, Self::Event { .. }) }

    /// Returns the expected jump for Event instruments, None otherwise.
    pub fn expected_jump(&self) -> Option<T> {
        match self {
            Self::Event { expected_jump, .. } => Some(*expected_jump),
            _ => None,
        }
    }

    /// Creates a copy of this instrument with the market rate bumped by
    /// `delta`.
    ///
    /// Used for finite-difference Jacobian computation in the sequential
    /// bootstrapper.
    pub fn with_bumped_rate(&self, delta: T) -> Self {
        match self {
            Self::Ois {
                maturity,
                rate,
                payment_frequency,
            } => Self::Ois {
                maturity: *maturity,
                rate: *rate + delta,
                payment_frequency: *payment_frequency,
            },
            Self::Irs {
                maturity,
                rate,
                fixed_frequency,
            } => Self::Irs {
                maturity: *maturity,
                rate: *rate + delta,
                fixed_frequency: *fixed_frequency,
            },
            Self::Fra { start, end, rate } => Self::Fra {
                start: *start,
                end: *end,
                rate: *rate + delta,
            },
            Self::Future {
                maturity,
                rate,
                convexity_adjustment,
            } => Self::Future {
                maturity: *maturity,
                rate: *rate + delta,
                convexity_adjustment: *convexity_adjustment,
            },
            Self::Event {
                maturity,
                expected_jump,
            } => Self::Event {
                maturity: *maturity,
                expected_jump: *expected_jump + delta,
            },
            Self::Bond {
                maturity,
                rate,
                coupon_rate,
                payment_frequency,
            } => Self::Bond {
                maturity: *maturity,
                rate: *rate + delta,
                coupon_rate: *coupon_rate,
                payment_frequency: *payment_frequency,
            },
            Self::Cds {
                maturity,
                spread,
                recovery_rate,
                ref risk_free_dfs,
            } => Self::Cds {
                maturity: *maturity,
                spread: *spread + delta,
                recovery_rate: *recovery_rate,
                risk_free_dfs: risk_free_dfs.clone(),
            },
        }
    }
}

/// A bootstrapped yield curve with pillar discount factors.
///
/// Supports jump-aware interpolation for modelling rate discontinuities
/// at central bank meeting dates.
#[derive(Debug, Clone)]
pub struct BootstrappedCurve<T: Float> {
    /// Pillar maturities in years.
    pillars: Vec<T>,
    /// Discount factors at each pillar.
    discount_factors: Vec<T>,
    /// Interpolation method.
    interpolation: BootstrapInterpolation,
    /// Whether to allow extrapolation.
    allow_extrapolation: bool,
    /// Jump events: (time, cumulative_offset) pairs.
    /// The offset is in log-space: adjusted_df = df *
    /// exp(cumulative_offset)
    jumps: Vec<(T, T)>,
    /// Precomputed spline coefficients for cubic/monotone/tension methods.
    /// Each entry holds [a, b, c, d] for the polynomial on that segment.
    #[allow(dead_code)]
    spline_coefficients: Vec<[T; 4]>,
    /// Tension parameter for TensionSpline interpolation.
    #[allow(dead_code)]
    tension: Option<T>,
}

/// Decomposition of a forward rate into continuous and jump components.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ForwardRateDecomposition<T> {
    /// Continuous component of the forward rate.
    pub continuous: T,
    /// Jump component of the forward rate.
    pub jump: T,
    /// Total forward rate (continuous + jump).
    pub total: T,
}

impl<T: Float> BootstrappedCurve<T> {
    /// Creates a new bootstrapped curve.
    pub fn new(
        pillars: Vec<T>,
        discount_factors: Vec<T>,
        interpolation: BootstrapInterpolation,
        allow_extrapolation: bool,
    ) -> Result<Self, String> {
        if pillars.len() != discount_factors.len() {
            return Err("pillars and discount_factors must have same length".to_string());
        }
        if pillars.is_empty() {
            return Err("curve must have at least one pillar".to_string());
        }
        let mut curve = Self {
            pillars,
            discount_factors,
            interpolation,
            allow_extrapolation,
            jumps: Vec::new(),
            spline_coefficients: Vec::new(),
            tension: None,
        };
        if interpolation.requires_spline_coefficients() {
            curve.recompute_spline_coefficients();
        }
        Ok(curve)
    }

    /// Adds jump data `(time, cumulative_log_offset)` to the curve.
    pub fn with_jumps(mut self, jumps: Vec<(T, T)>) -> Self {
        self.jumps = jumps;
        self
    }

    /// Returns the pillar maturities.
    pub fn pillars(&self) -> &[T] { &self.pillars }

    /// Returns the discount factors.
    pub fn discount_factors(&self) -> &[T] { &self.discount_factors }

    /// Returns true if the curve has jump data.
    pub fn has_jumps(&self) -> bool { !self.jumps.is_empty() }

    /// Returns the jump data.
    pub fn jumps(&self) -> &[(T, T)] { &self.jumps }


    /// Recomputes spline coefficients from the current pillars and discount factors.
    ///
    /// Called automatically during construction when the interpolation method
    /// requires spline coefficients. Call this again if the discount factors
    /// are mutated externally.
    pub fn recompute_spline_coefficients(&mut self) {
        let n = self.pillars.len();
        if n < 2 {
            self.spline_coefficients = Vec::new();
            return;
        }
        let fwd = self.derive_instantaneous_forwards();
        let segments = n - 1;
        let mut coeffs = Vec::with_capacity(segments);

        for k in 0..segments {
            let dt = self.pillars[k + 1] - self.pillars[k];
            if dt <= T::zero() {
                coeffs.push([T::zero(); 4]);
                continue;
            }
            let f0 = fwd[k];
            let f1 = fwd[k + 1];
            // Hermite basis: a + b*u + c*u^2 + d*u^3  where u = (t - t_k) / dt
            let a = f0;
            let b = T::zero();
            let c = from_f64::<T>(3.0) * (f1 - f0);
            let d = from_f64::<T>(-2.0) * (f1 - f0);
            coeffs.push([a, b, c, d]);
        }
        self.spline_coefficients = coeffs;
    }

    /// Derives instantaneous forward rates at each pillar from the discount
    /// factors using finite differences on the log-DF curve.
    fn derive_instantaneous_forwards(&self) -> Vec<T> {
        let n = self.pillars.len();
        let mut fwd = vec![T::zero(); n];
        if n < 2 {
            return fwd;
        }
        for k in 0..n {
            if k == 0 {
                let dt = self.pillars[1] - self.pillars[0];
                if dt > T::zero() {
                    fwd[0] = -(self.discount_factors[1].ln()
                        - self.discount_factors[0].ln())
                        / dt;
                }
            } else if k == n - 1 {
                let dt = self.pillars[k] - self.pillars[k - 1];
                if dt > T::zero() {
                    fwd[k] = -(self.discount_factors[k].ln()
                        - self.discount_factors[k - 1].ln())
                        / dt;
                }
            } else {
                let dt = self.pillars[k + 1] - self.pillars[k - 1];
                if dt > T::zero() {
                    fwd[k] = -(self.discount_factors[k + 1].ln()
                        - self.discount_factors[k - 1].ln())
                        / dt;
                }
            }
        }
        fwd
    }

    /// Sets the tension parameter (used by TensionSpline interpolation) and
    /// returns self for builder-style chaining.
    pub fn with_tension(mut self, tension: T) -> Self {
        self.tension = Some(tension);
        self
    }

    /// Returns the cumulative jump offset at time `t`.
    ///
    /// Uses binary search for O(log n) lookup.
    fn cumulative_offset_at(&self, t: T) -> T {
        if self.jumps.is_empty() {
            return T::zero();
        }

        let idx = self.jumps.partition_point(|(jump_t, _)| *jump_t <= t);
        if idx == 0 {
            T::zero()
        } else {
            self.jumps[idx - 1].1
        }
    }

    /// Returns the cumulative jump offset just before time `t` (left
    /// limit).
    fn cumulative_offset_before(&self, t: T) -> T {
        if self.jumps.is_empty() {
            return T::zero();
        }

        let idx = self.jumps.partition_point(|(jump_t, _)| *jump_t < t);
        if idx == 0 {
            T::zero()
        } else {
            self.jumps[idx - 1].1
        }
    }

    /// Returns the discount factor with limit specification (Left, Right,
    /// or Continuous).
    pub fn discount_factor_with_limit(
        &self,
        t: T,
        limit: pricer_core::types::Limit,
    ) -> Result<T, MarketDataError> {
        use pricer_core::types::Limit;

        // Get base discount factor (without jump adjustment)
        let base_df = self.discount_factor_base(t)?;

        if self.jumps.is_empty() {
            return Ok(base_df);
        }

        // Get the appropriate offset based on limit
        let offset = match limit {
            Limit::Left => self.cumulative_offset_before(t),
            Limit::Right | Limit::Continuous => self.cumulative_offset_at(t),
        };

        // Apply offset: adjusted_df = base_df * exp(offset)
        Ok(base_df * offset.exp())
    }

    /// Base discount factor calculation (without jump adjustment).
    fn discount_factor_base(&self, t: T) -> Result<T, MarketDataError> {
        if t <= T::zero() {
            return Ok(T::one());
        }

        let n = self.pillars.len();
        let max_t = self.pillars[n - 1];

        if t > max_t && !self.allow_extrapolation {
            return Err(MarketDataError::MaturityOutOfRange {
                maturity: t.to_f64().unwrap_or(0.0),
                max_maturity: max_t.to_f64().unwrap_or(0.0),
            });
        }

        // Handle single-pillar curve
        if n == 1 {
            let t1 = self.pillars[0];
            let df1 = self.discount_factors[0];
            if t1 > T::zero() && df1 > T::zero() {
                let r = -df1.ln() / t1;
                return Ok((-r * t).exp());
            }
            return Ok(df1);
        }

        // Find interpolation interval
        let mut i = 0;
        while i < n - 1 && self.pillars[i + 1] < t {
            i += 1;
        }

        if i >= n - 1 {
            i = n - 2;
        }

        let t1 = self.pillars[i];
        let t2 = self.pillars[i + 1];
        let df1 = self.discount_factors[i];
        let df2 = self.discount_factors[i + 1];

        let w = if t2 > t1 {
            (t - t1) / (t2 - t1)
        } else {
            T::zero()
        };

        match self.interpolation {
            BootstrapInterpolation::Linear => Ok(df1 * (T::one() - w) + df2 * w),
            BootstrapInterpolation::LogLinear | BootstrapInterpolation::FlatForward => {
                let log_df = df1.ln() * (T::one() - w) + df2.ln() * w;
                Ok(log_df.exp())
            }
            BootstrapInterpolation::CubicSplineFwd
            | BootstrapInterpolation::MonotoneConvex
            | BootstrapInterpolation::LogCubicDF
            | BootstrapInterpolation::TensionSpline => {
                if i >= self.spline_coefficients.len() {
                    // Fallback to log-linear when coefficients are unavailable.
                    let log_df = df1.ln() * (T::one() - w) + df2.ln() * w;
                    return Ok(log_df.exp());
                }
                let [a, b, c, d] = self.spline_coefficients[i];
                let u = w; // normalised position in segment
                let fwd_interp = a + u * (b + u * (c + u * d));
                let dt = t2 - t1;
                // DF(t) = DF(t1) * exp(-fwd_interp * (t - t1))
                Ok(df1 * (-fwd_interp * dt * u).exp())
            }
        }
    }

    /// Returns the forward rate with limit specification.
    pub fn forward_rate_with_limit(
        &self,
        t1: T,
        t2: T,
        limit: pricer_core::types::Limit,
    ) -> Result<T, MarketDataError> {
        let df1 = if t1 <= T::zero() {
            T::one()
        } else {
            self.discount_factor_with_limit(t1, limit)?
        };
        let df2 = self.discount_factor_with_limit(t2, limit)?;
        let tau = t2 - t1;

        if tau <= T::zero() {
            return Err(MarketDataError::InvalidInput {
                message: "t2 must be greater than t1".to_string(),
            });
        }

        Ok((df1 / df2 - T::one()) / tau)
    }

    /// Decomposes the forward rate into continuous and jump components.
    pub fn decompose_forward_rate(
        &self,
        t1: T,
        t2: T,
    ) -> Result<ForwardRateDecomposition<T>, MarketDataError> {
        use pricer_core::types::Limit;

        let tau = t2 - t1;
        if tau <= T::zero() {
            return Err(MarketDataError::InvalidInput {
                message: "t2 must be greater than t1".to_string(),
            });
        }

        // Total rate (with jumps)
        let total = self.forward_rate_with_limit(t1, t2, Limit::Continuous)?;

        // Continuous rate (without jumps applied in forward calculation)
        // We compute this by getting the base forward rate
        let df1_base = if t1 <= T::zero() {
            T::one()
        } else {
            self.discount_factor_base(t1)?
        };
        let df2_base = self.discount_factor_base(t2)?;
        let continuous = (df1_base / df2_base - T::one()) / tau;

        // Jump component
        let jump = total - continuous;

        Ok(ForwardRateDecomposition {
            continuous,
            jump,
            total,
        })
    }

    /// Returns the discount factor and its gradient w.r.t. pillar DFs.
    pub fn discount_factor_with_gradient(&self, t: T) -> Result<(T, Vec<T>), MarketDataError> {
        self.discount_factor_gradient_impl(t, false)
    }

    /// Returns the discount factor and its gradient w.r.t. log(DF) values.
    pub fn discount_factor_with_log_gradient(&self, t: T) -> Result<(T, Vec<T>), MarketDataError> {
        self.discount_factor_gradient_impl(t, true)
    }

    /// Shared gradient implementation. When `log_mode` is true, computes
    /// dDF(t)/d_log(DF_i); otherwise dDF(t)/dDF_i.
    fn discount_factor_gradient_impl(
        &self,
        t: T,
        log_mode: bool,
    ) -> Result<(T, Vec<T>), MarketDataError> {
        let n = self.pillars.len();
        let mut gradient = vec![T::zero(); n];

        if t <= T::zero() {
            return Ok((T::one(), gradient));
        }

        let max_t = self.pillars[n - 1];
        if t > max_t && !self.allow_extrapolation {
            return Err(MarketDataError::MaturityOutOfRange {
                maturity: t.to_f64().unwrap_or(0.0),
                max_maturity: max_t.to_f64().unwrap_or(0.0),
            });
        }

        if n == 1 {
            let t1 = self.pillars[0];
            let df1 = self.discount_factors[0];
            if t1 > T::zero() && df1 > T::zero() {
                let r = -df1.ln() / t1;
                let df = (-r * t).exp();
                gradient[0] = if log_mode {
                    df * (t / t1)
                } else {
                    df * (t / (t1 * df1))
                };
                return Ok((df, gradient));
            }
            gradient[0] = if log_mode { df1 } else { T::one() };
            return Ok((df1, gradient));
        }

        let mut i = 0;
        while i < n - 1 && self.pillars[i + 1] < t {
            i += 1;
        }
        if i >= n - 1 {
            i = n - 2;
        }

        let t1 = self.pillars[i];
        let t2 = self.pillars[i + 1];
        let df1 = self.discount_factors[i];
        let df2 = self.discount_factors[i + 1];

        let w = if t2 > t1 {
            (t - t1) / (t2 - t1)
        } else {
            T::zero()
        };

        let df = match self.interpolation {
            BootstrapInterpolation::Linear => {
                let df = df1 * (T::one() - w) + df2 * w;
                if log_mode {
                    gradient[i] = df1 * (T::one() - w);
                    gradient[i + 1] = df2 * w;
                } else {
                    gradient[i] = T::one() - w;
                    gradient[i + 1] = w;
                }
                df
            }
            BootstrapInterpolation::LogLinear
            | BootstrapInterpolation::FlatForward
            | BootstrapInterpolation::CubicSplineFwd
            | BootstrapInterpolation::MonotoneConvex
            | BootstrapInterpolation::LogCubicDF
            | BootstrapInterpolation::TensionSpline => {
                let log_df = df1.ln() * (T::one() - w) + df2.ln() * w;
                let df = log_df.exp();
                if log_mode {
                    gradient[i] = df * (T::one() - w);
                    gradient[i + 1] = df * w;
                } else {
                    gradient[i] = df * (T::one() - w) / df1;
                    gradient[i + 1] = df * w / df2;
                }
                df
            }
        };

        Ok((df, gradient))
    }
}

impl<T: Float> YieldCurve<T> for BootstrappedCurve<T> {
    fn discount_factor(&self, t: T) -> Result<T, MarketDataError> {
        // Default behavior uses Continuous limit (includes jumps, right-limit)
        self.discount_factor_with_limit(t, pricer_core::types::Limit::Continuous)
    }
}

/// Enum wrapper for different curve types (static dispatch via
/// `enum_dispatch`).
#[derive(Debug, Clone)]
#[enum_dispatch(YieldCurve<T>)]
pub enum CurveEnum<T: Float> {
    /// Flat curve with constant rate.
    Flat(FlatCurve<T>),
    /// Bootstrapped curve from market instruments.
    Bootstrapped(BootstrappedCurve<T>),
}

impl<T: Float> CurveEnum<T> {
    /// Creates a flat curve with the given rate.
    pub fn flat(rate: T) -> Self { Self::Flat(FlatCurve::new(rate)) }

    /// Creates a bootstrapped curve.
    pub fn bootstrapped(curve: BootstrappedCurve<T>) -> Self { Self::Bootstrapped(curve) }
}

// ---------------------------------------------------------------------------
// Named curve identifiers and curve collections
// ---------------------------------------------------------------------------

/// Named curve identifiers for common rate indices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CurveName {
    /// SOFR (Secured Overnight Financing Rate)
    Sofr,
    /// EURIBOR (Euro Interbank Offered Rate)
    Euribor,
    /// ESTR (Euro Short-Term Rate)
    Estr,
    /// TONAR (Tokyo Overnight Average Rate) / TONA
    Tonar,
    /// SONIA (Sterling Overnight Index Average)
    Sonia,
    /// Generic discount curve
    Discount,
    /// Generic forward curve
    Forward,
    /// Custom curve with a name
    Custom(&'static str),
}

/// A collection of named yield curves.
#[derive(Debug, Clone, Default)]
pub struct CurveSet<T: Float> {
    curves: std::collections::HashMap<CurveName, CurveEnum<T>>,
}

impl<T: Float + 'static> CurveSet<T> {
    /// Creates a new empty curve set.
    pub fn new() -> Self {
        Self {
            curves: std::collections::HashMap::new(),
        }
    }

    /// Inserts a curve with the given name.
    pub fn insert(&mut self, name: CurveName, curve: CurveEnum<T>) {
        self.curves.insert(name, curve);
    }

    /// Gets a curve by name.
    pub fn get(&self, name: &CurveName) -> Option<&CurveEnum<T>> { self.curves.get(name) }

    /// Returns the number of curves.
    pub fn len(&self) -> usize { self.curves.len() }

    /// Returns true if the set is empty.
    pub fn is_empty(&self) -> bool { self.curves.is_empty() }

    /// Computes the forward rate for a rate index between two times.
    ///
    /// Maps the rate index to a curve name and looks up the forward rate.
    pub fn forward_rate_for_index(
        &self,
        rate_index: infra_domain::market::RateIndex,
        t1: T,
        t2: T,
    ) -> Result<T, MarketDataError> {
        use infra_domain::market::RateIndex;

        let curve_name = match rate_index {
            RateIndex::Sofr => CurveName::Sofr,
            RateIndex::Euribor3M | RateIndex::Euribor6M => CurveName::Euribor,
            RateIndex::Estr => CurveName::Estr,
            RateIndex::Tonar => CurveName::Tonar,
            RateIndex::Sonia => CurveName::Sonia,
            RateIndex::Saron => CurveName::Custom("SARON"),
            _ => CurveName::Sofr, // Default fallback for unknown indices
        };

        let curve = self
            .curves
            .get(&curve_name)
            .ok_or(MarketDataError::CurveNotFound {
                name: format!("{:?}", rate_index),
            })?;

        curve.forward_rate(t1, t2)
    }

    /// Returns an iterator over (name, curve) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&CurveName, &CurveEnum<T>)> { self.curves.iter() }

    /// Gets the discount curve if set.
    pub fn discount_curve(&self) -> Option<&CurveEnum<T>> { self.curves.get(&CurveName::Discount) }

    /// Sets the discount curve.
    pub fn set_discount_curve(&mut self, _name: CurveName) {
        // Placeholder - actual implementation would handle curve assignment
    }
}

#[cfg(test)]
mod tests {
    use pricer_core::types::Limit;

    use super::*;

    #[test]
    fn test_flat_curve_discount_factor() {
        let curve = FlatCurve::new(0.05_f64);
        let df = curve.discount_factor(1.0).unwrap();
        assert!((df - (-0.05_f64).exp()).abs() < 1e-10);
    }

    #[test]
    fn test_bootstrapped_curve() {
        let pillars = vec![1.0_f64, 2.0, 5.0];
        let dfs: Vec<f64> = pillars.iter().map(|&t| (-0.03 * t).exp()).collect();
        let curve =
            BootstrappedCurve::new(pillars, dfs, BootstrapInterpolation::LogLinear, true).unwrap();

        let df = curve.discount_factor(1.5).unwrap();
        assert!(df > 0.0 && df < 1.0);
    }

    #[test]
    fn test_bootstrapped_curve_no_jumps_same_as_base() {
        let pillars = vec![0.25_f64, 0.5, 1.0, 2.0];
        let dfs: Vec<f64> = pillars.iter().map(|&t| (-0.03 * t).exp()).collect();
        let curve = BootstrappedCurve::new(
            pillars.clone(),
            dfs.clone(),
            BootstrapInterpolation::LogLinear,
            true,
        )
        .unwrap();

        let df_cont = curve
            .discount_factor_with_limit(0.75, Limit::Continuous)
            .unwrap();
        let df_left = curve.discount_factor_with_limit(0.75, Limit::Left).unwrap();
        let df_right = curve
            .discount_factor_with_limit(0.75, Limit::Right)
            .unwrap();

        assert!((df_cont - df_left).abs() < 1e-12);
        assert!((df_cont - df_right).abs() < 1e-12);
        assert!(!curve.has_jumps());
    }

    #[test]
    fn test_bootstrapped_curve_single_jump_left_right_limit() {
        let pillars = vec![0.25_f64, 0.5, 1.0];
        let dfs: Vec<f64> = pillars.iter().map(|&t| (-0.03 * t).exp()).collect();

        let jump_offset = -0.0025_f64;
        let jumps = vec![(0.5_f64, jump_offset)];

        let curve = BootstrappedCurve::new(pillars, dfs, BootstrapInterpolation::LogLinear, true)
            .unwrap()
            .with_jumps(jumps);

        assert!(curve.has_jumps());
        assert_eq!(curve.jumps().len(), 1);

        let df_left = curve.discount_factor_with_limit(0.5, Limit::Left).unwrap();
        let df_right = curve.discount_factor_with_limit(0.5, Limit::Right).unwrap();

        assert!(df_right < df_left);

        let ratio = df_right / df_left;
        assert!((ratio - jump_offset.exp()).abs() < 1e-10);
    }

    #[test]
    fn test_bootstrapped_curve_multiple_jumps_cumulative() {
        let pillars = vec![0.25_f64, 0.5, 0.75, 1.0];
        let dfs: Vec<f64> = pillars.iter().map(|&t| (-0.03 * t).exp()).collect();

        let jumps = vec![(0.3_f64, -0.0025), (0.6_f64, -0.005)];

        let curve = BootstrappedCurve::new(pillars, dfs, BootstrapInterpolation::LogLinear, true)
            .unwrap()
            .with_jumps(jumps);

        let df_before = curve
            .discount_factor_with_limit(0.2, Limit::Continuous)
            .unwrap();
        let base_df_at_02 = (-0.03 * 0.2_f64).exp();
        assert!((df_before - base_df_at_02).abs() < 1e-6);

        let df_between_cont = curve
            .discount_factor_with_limit(0.4, Limit::Continuous)
            .unwrap();
        let df_between_left = curve.discount_factor_with_limit(0.4, Limit::Left).unwrap();
        assert!((df_between_cont - df_between_left).abs() < 1e-12);

        let df_after_left = curve.discount_factor_with_limit(0.8, Limit::Left).unwrap();
        let df_after_right = curve.discount_factor_with_limit(0.8, Limit::Right).unwrap();
        assert!((df_after_left - df_after_right).abs() < 1e-12);
    }

    #[test]
    fn test_bootstrapped_curve_forward_rate_with_limit() {
        let pillars = vec![0.25_f64, 0.5, 1.0];
        let dfs: Vec<f64> = pillars.iter().map(|&t| (-0.03 * t).exp()).collect();

        let jumps = vec![(0.5_f64, -0.0025)];

        let curve = BootstrappedCurve::new(pillars, dfs, BootstrapInterpolation::LogLinear, true)
            .unwrap()
            .with_jumps(jumps);

        let fwd_rate = curve
            .forward_rate_with_limit(0.25, 0.75, Limit::Continuous)
            .unwrap();

        assert!(fwd_rate.is_finite());

        let df1 = curve
            .discount_factor_with_limit(0.25, Limit::Continuous)
            .unwrap();
        let df2 = curve
            .discount_factor_with_limit(0.75, Limit::Continuous)
            .unwrap();
        let expected_fwd = (df1 / df2 - 1.0) / 0.5;

        assert!((fwd_rate - expected_fwd).abs() < 1e-10);
    }

    #[test]
    fn test_bootstrapped_curve_decompose_forward_rate() {
        let pillars = vec![0.25_f64, 0.5, 0.75, 1.0];
        let dfs: Vec<f64> = pillars.iter().map(|&t| (-0.03 * t).exp()).collect();

        let jump_offset = -0.0025_f64;
        let jumps = vec![(0.5_f64, jump_offset)];

        let curve = BootstrappedCurve::new(pillars, dfs, BootstrapInterpolation::LogLinear, true)
            .unwrap()
            .with_jumps(jumps);

        let decomp = curve.decompose_forward_rate(0.25, 0.75).unwrap();

        assert!((decomp.total - (decomp.continuous + decomp.jump)).abs() < 1e-10);
        assert!(decomp.jump.abs() > 1e-10);
    }

    #[test]
    fn test_bootstrapped_curve_decompose_forward_rate_no_jump_in_range() {
        let pillars = vec![0.25_f64, 0.5, 0.75, 1.0];
        let dfs: Vec<f64> = pillars.iter().map(|&t| (-0.03 * t).exp()).collect();

        let jumps = vec![(0.5_f64, -0.0025)];

        let curve = BootstrappedCurve::new(pillars, dfs, BootstrapInterpolation::LogLinear, true)
            .unwrap()
            .with_jumps(jumps);

        let decomp = curve.decompose_forward_rate(0.6, 0.8).unwrap();

        assert!(decomp.jump.abs() < 1e-10);
        assert!((decomp.total - decomp.continuous).abs() < 1e-10);
    }

    #[test]
    fn test_bootstrapped_curve_with_jumps_builder() {
        let pillars = vec![0.25_f64, 0.5, 1.0];
        let dfs: Vec<f64> = pillars.iter().map(|&t| (-0.03 * t).exp()).collect();

        let curve = BootstrappedCurve::new(pillars, dfs, BootstrapInterpolation::LogLinear, true)
            .unwrap()
            .with_jumps(vec![(0.5, -0.0025)]);

        assert!(curve.has_jumps());
        assert_eq!(curve.jumps().len(), 1);
        assert_eq!(curve.jumps()[0], (0.5, -0.0025));
    }

    #[test]
    fn test_bootstrapped_curve_default_discount_factor_uses_continuous() {
        let pillars = vec![0.25_f64, 0.5, 1.0];
        let dfs: Vec<f64> = pillars.iter().map(|&t| (-0.03 * t).exp()).collect();

        let jumps = vec![(0.5_f64, -0.0025)];

        let curve = BootstrappedCurve::new(pillars, dfs, BootstrapInterpolation::LogLinear, true)
            .unwrap()
            .with_jumps(jumps);

        let df_default = curve.discount_factor(0.75).unwrap();
        let df_continuous = curve
            .discount_factor_with_limit(0.75, Limit::Continuous)
            .unwrap();

        assert!((df_default - df_continuous).abs() < 1e-12);
    }

    #[test]
    fn test_bootstrapped_curve_forward_rate_consistency_with_zero_rate() {
        let pillars = vec![0.25_f64, 0.5, 1.0, 2.0];
        let rate = 0.03_f64;
        let dfs: Vec<f64> = pillars.iter().map(|&t| (-rate * t).exp()).collect();

        let curve =
            BootstrappedCurve::new(pillars, dfs, BootstrapInterpolation::LogLinear, true).unwrap();

        let fwd = curve
            .forward_rate_with_limit(0.5, 1.0, Limit::Continuous)
            .unwrap();

        let expected = ((rate * 0.5).exp() - 1.0) / 0.5;
        assert!((fwd - expected).abs() < 1e-10);
    }
}
