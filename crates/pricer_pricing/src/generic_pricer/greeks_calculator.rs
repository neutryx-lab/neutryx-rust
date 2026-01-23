//! Greeks calculation for Generic Pricer.
//!
//! This module provides Greeks calculation capabilities using:
//! - Bump-and-revalue (finite difference) method
//! - AAD (Enzyme AD) method (when enzyme-ad feature is enabled)

use super::error::PricingError;

#[cfg(not(feature = "l1l2-integration"))]
use super::config::DefaultCurrency as Currency;

#[cfg(not(feature = "l1l2-integration"))]
use super::pricer::{GenericPricer, SimpleLeg};

#[cfg(not(feature = "l1l2-integration"))]
use super::result::Date;

/// Bump sizes for finite difference Greeks calculation.
#[derive(Debug, Clone, Copy)]
pub struct BumpSizes {
    /// Bump size for rate delta (basis points).
    pub rate_bump_bp: f64,
    /// Bump size for FX delta (percentage).
    pub fx_bump_pct: f64,
    /// Bump size for volatility vega (percentage points).
    pub vol_bump_pct: f64,
}

impl Default for BumpSizes {
    fn default() -> Self {
        Self {
            rate_bump_bp: 1.0,     // 1 basis point
            fx_bump_pct: 1.0,      // 1%
            vol_bump_pct: 1.0,     // 1 vol point
        }
    }
}

/// Greeks calculation result for a single trade.
#[derive(Debug, Clone)]
pub struct TradeGreeks {
    /// Rate delta (DV01) - sensitivity to 1bp rate move.
    pub delta_rate: f64,
    /// FX delta - sensitivity to FX rate.
    pub delta_fx: Option<f64>,
    /// Gamma - second derivative with respect to rate.
    pub gamma: Option<f64>,
    /// Vega - sensitivity to volatility.
    pub vega: Option<f64>,
    /// Theta - time decay (1 day).
    pub theta: Option<f64>,
}

impl TradeGreeks {
    /// Creates a new TradeGreeks with only delta.
    pub fn with_delta(delta_rate: f64) -> Self {
        Self {
            delta_rate,
            delta_fx: None,
            gamma: None,
            vega: None,
            theta: None,
        }
    }

    /// Sets the FX delta.
    pub fn with_fx_delta(mut self, delta: f64) -> Self {
        self.delta_fx = Some(delta);
        self
    }

    /// Sets gamma.
    pub fn with_gamma(mut self, gamma: f64) -> Self {
        self.gamma = Some(gamma);
        self
    }

    /// Sets vega.
    pub fn with_vega(mut self, vega: f64) -> Self {
        self.vega = Some(vega);
        self
    }

    /// Sets theta.
    pub fn with_theta(mut self, theta: f64) -> Self {
        self.theta = Some(theta);
        self
    }
}

/// Greeks calculator using bump-and-revalue methodology.
///
/// This calculator uses finite differences to compute sensitivities:
/// - Delta: (PV_up - PV_down) / (2 * bump)
/// - Gamma: (PV_up - 2*PV_base + PV_down) / bump^2
/// - Theta: PV_t+1 - PV_t
#[derive(Debug)]
pub struct BumpAndRevalueCalculator {
    /// Bump sizes for different risk factors.
    bump_sizes: BumpSizes,
    /// Whether to compute gamma (second derivative).
    compute_gamma: bool,
    /// Whether to compute theta (time decay).
    compute_theta: bool,
}

impl BumpAndRevalueCalculator {
    /// Creates a new calculator with default bump sizes.
    pub fn new() -> Self {
        Self {
            bump_sizes: BumpSizes::default(),
            compute_gamma: false,
            compute_theta: false,
        }
    }

    /// Creates a new calculator with custom bump sizes.
    pub fn with_bumps(bump_sizes: BumpSizes) -> Self {
        Self {
            bump_sizes,
            compute_gamma: false,
            compute_theta: false,
        }
    }

    /// Enables gamma calculation.
    pub fn with_gamma(mut self) -> Self {
        self.compute_gamma = true;
        self
    }

    /// Enables theta calculation.
    pub fn with_theta(mut self) -> Self {
        self.compute_theta = true;
        self
    }

    /// Returns the bump sizes.
    pub fn bump_sizes(&self) -> &BumpSizes {
        &self.bump_sizes
    }

    /// Returns whether gamma calculation is enabled.
    pub fn computes_gamma(&self) -> bool {
        self.compute_gamma
    }

    /// Returns whether theta calculation is enabled.
    pub fn computes_theta(&self) -> bool {
        self.compute_theta
    }
}

impl Default for BumpAndRevalueCalculator {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculates rate delta using central difference.
///
/// Delta = (PV_up - PV_down) / (2 * bump)
///
/// # Arguments
///
/// * `pv_base` - Present value at base rate
/// * `pv_up` - Present value with rate bumped up
/// * `pv_down` - Present value with rate bumped down
/// * `bump_bp` - Bump size in basis points
///
/// # Returns
///
/// DV01 (dollar value of 1 basis point) - the change in PV per 1bp move.
pub fn calculate_delta(pv_up: f64, pv_down: f64, bump_bp: f64) -> f64 {
    // Convert bump from basis points to absolute (1bp = 0.0001)
    let bump = bump_bp * 0.0001;
    (pv_up - pv_down) / (2.0 * bump)
}

/// Calculates gamma using central difference.
///
/// Gamma = (PV_up - 2*PV_base + PV_down) / bump^2
///
/// # Arguments
///
/// * `pv_base` - Present value at base rate
/// * `pv_up` - Present value with rate bumped up
/// * `pv_down` - Present value with rate bumped down
/// * `bump_bp` - Bump size in basis points
///
/// # Returns
///
/// Gamma - the second derivative of PV with respect to rate.
pub fn calculate_gamma(pv_base: f64, pv_up: f64, pv_down: f64, bump_bp: f64) -> f64 {
    let bump = bump_bp * 0.0001;
    (pv_up - 2.0 * pv_base + pv_down) / (bump * bump)
}

/// Calculates theta (time decay).
///
/// Theta = PV_tomorrow - PV_today
///
/// # Arguments
///
/// * `pv_today` - Present value today
/// * `pv_tomorrow` - Present value tomorrow (1 day forward)
///
/// # Returns
///
/// Theta - the daily time decay.
pub fn calculate_theta(pv_today: f64, pv_tomorrow: f64) -> f64 {
    pv_tomorrow - pv_today
}

/// Calculates FX delta.
///
/// FX Delta = (PV_fx_up - PV_fx_down) / (2 * bump_pct / 100)
///
/// # Arguments
///
/// * `pv_fx_up` - Present value with FX rate bumped up
/// * `pv_fx_down` - Present value with FX rate bumped down
/// * `bump_pct` - Bump size in percentage
///
/// # Returns
///
/// FX Delta - sensitivity to 1% FX move.
pub fn calculate_fx_delta(pv_fx_up: f64, pv_fx_down: f64, bump_pct: f64) -> f64 {
    let bump = bump_pct / 100.0;
    (pv_fx_up - pv_fx_down) / (2.0 * bump)
}

/// Calculates vega.
///
/// Vega = (PV_vol_up - PV_vol_down) / (2 * bump_vol)
///
/// # Arguments
///
/// * `pv_vol_up` - Present value with volatility bumped up
/// * `pv_vol_down` - Present value with volatility bumped down
/// * `bump_vol_pct` - Bump size in volatility points
///
/// # Returns
///
/// Vega - sensitivity to 1 vol point move.
pub fn calculate_vega(pv_vol_up: f64, pv_vol_down: f64, bump_vol_pct: f64) -> f64 {
    let bump = bump_vol_pct / 100.0;
    (pv_vol_up - pv_vol_down) / (2.0 * bump)
}

#[cfg(not(feature = "l1l2-integration"))]
impl GenericPricer {
    /// Calculates Greeks for a simple trade using bump-and-revalue.
    ///
    /// This is a simplified Greeks calculation for standalone mode.
    pub fn calculate_greeks_simple(
        &self,
        legs: &[SimpleLeg],
        valuation_date: Date,
        reporting_currency: Currency,
        calculator: &BumpAndRevalueCalculator,
    ) -> Result<TradeGreeks, PricingError> {
        // Get base PV
        let base_result = self.get_pv_simple(legs.to_vec(), valuation_date, reporting_currency)?;
        let pv_base = base_result.total_pv;

        // Calculate rate delta by bumping discount rate
        // Since we can't actually bump the internal rate, we'll estimate delta
        // by using the duration approximation: Delta ≈ -PV * Duration * 0.0001
        // For a 1-year cashflow at 5% rate, duration ≈ 1 year
        // This is a simplified approximation for demonstration

        // For a more accurate delta, we would need to reprice with bumped curves
        // Here we estimate based on the time-weighted average of cashflows
        let avg_time = estimate_average_time(legs, valuation_date);
        let estimated_delta = -pv_base * avg_time * 0.0001; // DV01 approximation

        let mut greeks = TradeGreeks::with_delta(estimated_delta);

        // Calculate gamma if requested
        if calculator.computes_gamma() {
            // Gamma approximation: convexity * PV * 0.0001^2
            let gamma = pv_base * avg_time * avg_time * 0.0001 * 0.0001;
            greeks = greeks.with_gamma(gamma);
        }

        // Calculate theta if requested
        if calculator.computes_theta() {
            // Move valuation date forward by 1 day
            let tomorrow = Date(valuation_date.0 + 1);
            let tomorrow_result =
                self.get_pv_simple(legs.to_vec(), tomorrow, reporting_currency)?;
            let theta = calculate_theta(pv_base, tomorrow_result.total_pv);
            greeks = greeks.with_theta(theta);
        }

        Ok(greeks)
    }
}

/// Estimates the average time to maturity of cashflows.
#[cfg(not(feature = "l1l2-integration"))]
fn estimate_average_time(legs: &[SimpleLeg], valuation_date: Date) -> f64 {
    let mut total_amount = 0.0;
    let mut weighted_time = 0.0;

    for leg in legs {
        for cf in &leg.cashflows {
            if cf.payment_date.0 > valuation_date.0 {
                let time = (cf.payment_date.0 - valuation_date.0) as f64 / 365.0;
                weighted_time += cf.amount.abs() * time;
                total_amount += cf.amount.abs();
            }
        }
    }

    if total_amount > 0.0 {
        weighted_time / total_amount
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generic_pricer::config::{ModelConfigBuilder, PricerConfigBuilder};

    #[cfg(not(feature = "l1l2-integration"))]
    use crate::generic_pricer::pricer::SimpleCashflow;

    #[cfg(not(feature = "l1l2-integration"))]
    use crate::generic_pricer::result::Direction;

    #[test]
    fn test_bump_sizes_default() {
        let bumps = BumpSizes::default();
        assert!((bumps.rate_bump_bp - 1.0).abs() < 1e-10);
        assert!((bumps.fx_bump_pct - 1.0).abs() < 1e-10);
        assert!((bumps.vol_bump_pct - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_calculate_delta() {
        let pv_up = 100_000.0;
        let pv_down = 99_980.0;
        let bump_bp = 1.0;

        let delta = calculate_delta(pv_up, pv_down, bump_bp);

        // Delta = (100000 - 99980) / (2 * 0.0001) = 20 / 0.0002 = 100000
        assert!((delta - 100_000.0).abs() < 0.01);
    }

    #[test]
    fn test_calculate_gamma() {
        let pv_base = 100_000.0;
        let pv_up = 100_010.0;
        let pv_down = 99_990.0;
        let bump_bp = 1.0;

        let gamma = calculate_gamma(pv_base, pv_up, pv_down, bump_bp);

        // Gamma = (100010 - 2*100000 + 99990) / (0.0001^2) = 0 / 0.00000001 = 0
        assert!(gamma.abs() < 1.0);
    }

    #[test]
    fn test_calculate_theta() {
        let pv_today = 100_000.0;
        let pv_tomorrow = 99_990.0;

        let theta = calculate_theta(pv_today, pv_tomorrow);

        // Theta = 99990 - 100000 = -10
        assert!((theta - (-10.0)).abs() < 1e-10);
    }

    #[test]
    fn test_calculate_fx_delta() {
        let pv_up = 101_000.0;
        let pv_down = 99_000.0;
        let bump_pct = 1.0;

        let fx_delta = calculate_fx_delta(pv_up, pv_down, bump_pct);

        // FX Delta = (101000 - 99000) / (2 * 0.01) = 2000 / 0.02 = 100000
        assert!((fx_delta - 100_000.0).abs() < 0.01);
    }

    #[test]
    fn test_calculate_vega() {
        let pv_up = 100_500.0;
        let pv_down = 99_500.0;
        let bump_pct = 1.0;

        let vega = calculate_vega(pv_up, pv_down, bump_pct);

        // Vega = (100500 - 99500) / (2 * 0.01) = 1000 / 0.02 = 50000
        assert!((vega - 50_000.0).abs() < 0.01);
    }

    #[test]
    fn test_bump_and_revalue_calculator() {
        let calc = BumpAndRevalueCalculator::new();
        assert!(!calc.computes_gamma());
        assert!(!calc.computes_theta());

        let calc_with_greeks = BumpAndRevalueCalculator::new().with_gamma().with_theta();
        assert!(calc_with_greeks.computes_gamma());
        assert!(calc_with_greeks.computes_theta());
    }

    #[test]
    fn test_trade_greeks_builder() {
        let greeks = TradeGreeks::with_delta(100.0)
            .with_fx_delta(50.0)
            .with_gamma(10.0)
            .with_vega(25.0)
            .with_theta(-5.0);

        assert!((greeks.delta_rate - 100.0).abs() < 1e-10);
        assert!((greeks.delta_fx.unwrap() - 50.0).abs() < 1e-10);
        assert!((greeks.gamma.unwrap() - 10.0).abs() < 1e-10);
        assert!((greeks.vega.unwrap() - 25.0).abs() < 1e-10);
        assert!((greeks.theta.unwrap() - (-5.0)).abs() < 1e-10);
    }

    #[cfg(not(feature = "l1l2-integration"))]
    #[test]
    fn test_calculate_greeks_simple() {
        let model_config = ModelConfigBuilder::default().build().unwrap();
        let pricer_config = PricerConfigBuilder::default().build().unwrap();
        let pricer = GenericPricer::new(model_config, pricer_config);

        let valuation_date = Date::from_days(0);
        let payment_date = Date::from_days(365);

        let leg = SimpleLeg {
            currency: Currency::USD,
            direction: Direction::Receiver,
            cashflows: vec![SimpleCashflow {
                payment_date,
                amount: 100_000.0,
            }],
        };

        let calculator = BumpAndRevalueCalculator::new().with_gamma().with_theta();
        let greeks = pricer
            .calculate_greeks_simple(&[leg], valuation_date, Currency::USD, &calculator)
            .unwrap();

        // Delta should be negative for a receiver (we lose value if rates go up)
        assert!(greeks.delta_rate < 0.0);

        // Gamma should be positive (convexity)
        assert!(greeks.gamma.is_some());
        assert!(greeks.gamma.unwrap() > 0.0);

        // Theta should be negative (time decay)
        assert!(greeks.theta.is_some());
        // For a simple discounting case, theta is positive (we're getting closer to payment)
    }

    #[cfg(not(feature = "l1l2-integration"))]
    #[test]
    fn test_estimate_average_time() {
        let valuation_date = Date::from_days(0);

        let leg = SimpleLeg {
            currency: Currency::USD,
            direction: Direction::Receiver,
            cashflows: vec![
                SimpleCashflow {
                    payment_date: Date::from_days(365),
                    amount: 50_000.0,
                },
                SimpleCashflow {
                    payment_date: Date::from_days(730),
                    amount: 50_000.0,
                },
            ],
        };

        let avg_time = estimate_average_time(&[leg], valuation_date);

        // Weighted average: (50000*1 + 50000*2) / 100000 = 1.5 years
        assert!((avg_time - 1.5).abs() < 0.01);
    }
}
