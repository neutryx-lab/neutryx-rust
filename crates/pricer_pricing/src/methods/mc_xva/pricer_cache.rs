//! Trade-level pricing cache for XVA simulations.
//!
//! Provides [`XvaTradePricer`], a lightweight enum-dispatched pricer for
//! repricing trades along simulated scenarios. Supports analytic
//! Black-Scholes, forward, and interest rate swap pricing.

use pricer_core::math::normal_dist::norm_cdf;

/// Lightweight trade pricer for XVA scenario repricing.
///
/// Uses static dispatch via an enum to avoid dynamic dispatch overhead
/// in hot simulation loops.
#[derive(Clone, Debug)]
pub enum XvaTradePricer {
    /// Analytic Black-Scholes call option pricer.
    AnalyticBlackScholes {
        /// Risk-free rate (continuously compounded).
        rate: f64,
        /// Implied volatility.
        vol: f64,
    },

    /// Simple forward contract pricer.
    Forward {
        /// Risk-free rate (continuously compounded).
        rate: f64,
    },

    /// Interest rate swap (fixed-for-floating) NPV pricer.
    Swap {
        /// Fixed rate of the swap.
        fixed_rate: f64,
        /// Payment frequency in year fractions (e.g., 0.5 for semi-annual).
        payment_freq: f64,
    },
}

impl XvaTradePricer {
    /// Prices the instrument given the current spot, strike, time to maturity
    /// and notional.
    ///
    /// # Arguments
    ///
    /// * `spot` - Current spot price / rate.
    /// * `strike` - Strike price / rate.
    /// * `time_to_maturity` - Time to maturity in years.
    /// * `notional` - Notional amount.
    ///
    /// # Returns
    ///
    /// The present value of the instrument.
    pub fn price(&self, spot: f64, strike: f64, time_to_maturity: f64, notional: f64) -> f64 {
        match self {
            Self::AnalyticBlackScholes { rate, vol } => {
                bs_call_price(spot, strike, time_to_maturity, *rate, *vol, notional)
            }
            Self::Forward { rate } => {
                forward_price(spot, strike, time_to_maturity, *rate, notional)
            }
            Self::Swap {
                fixed_rate,
                payment_freq,
            } => swap_npv(spot, *fixed_rate, time_to_maturity, *payment_freq, notional),
        }
    }
}

/// Black-Scholes European call option price.
///
/// ```text
/// C = N * [S * N(d1) - K * exp(-r*T) * N(d2)]
/// d1 = [ln(S/K) + (r + 0.5*v^2)*T] / (v * sqrt(T))
/// d2 = d1 - v * sqrt(T)
/// ```
fn bs_call_price(
    spot: f64,
    strike: f64,
    time_to_maturity: f64,
    rate: f64,
    vol: f64,
    notional: f64,
) -> f64 {
    if time_to_maturity <= 0.0 {
        // At or past expiry, return intrinsic value.
        return notional * (spot - strike).max(0.0);
    }
    if vol <= 0.0 {
        // Zero vol: deterministic forward.
        let forward = spot * (rate * time_to_maturity).exp();
        return notional * (forward - strike).max(0.0) * (-rate * time_to_maturity).exp();
    }

    let sqrt_t = time_to_maturity.sqrt();
    let d1 = ((spot / strike).ln() + (rate + 0.5 * vol * vol) * time_to_maturity) / (vol * sqrt_t);
    let d2 = d1 - vol * sqrt_t;

    let discount = (-rate * time_to_maturity).exp();

    notional * (spot * norm_cdf(d1) - strike * discount * norm_cdf(d2))
}

/// Forward contract PV: `N * [S - K * exp(-r*T)]`.
fn forward_price(spot: f64, strike: f64, time_to_maturity: f64, rate: f64, notional: f64) -> f64 {
    if time_to_maturity <= 0.0 {
        return notional * (spot - strike);
    }
    let discount = (-rate * time_to_maturity).exp();
    notional * (spot - strike * discount)
}

/// Simplified IRS NPV: receives floating (spot rate), pays fixed.
///
/// NPV = Notional * (spot - fixed_rate) * annuity factor
///
/// where the annuity factor = sum of discount factors at payment dates.
fn swap_npv(
    floating_rate: f64,
    fixed_rate: f64,
    time_to_maturity: f64,
    payment_freq: f64,
    notional: f64,
) -> f64 {
    if time_to_maturity <= 0.0 || payment_freq <= 0.0 {
        return 0.0;
    }

    // Calculate the number of remaining payment periods.
    let n_periods = (time_to_maturity / payment_freq).ceil() as usize;
    if n_periods == 0 {
        return 0.0;
    }

    // Sum discounted cash flows.
    let mut npv = 0.0;
    for i in 1..=n_periods {
        let t_i = (i as f64) * payment_freq;
        let t_i = t_i.min(time_to_maturity);
        let df = (-floating_rate * t_i).exp();
        let period_frac = if i == n_periods && (time_to_maturity % payment_freq) > 1e-10 {
            time_to_maturity - ((i - 1) as f64) * payment_freq
        } else {
            payment_freq
        };
        npv += (floating_rate - fixed_rate) * period_frac * df;
    }

    notional * npv
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    // --- Black-Scholes tests ---

    #[test]
    fn test_bs_call_atm() {
        let pricer = XvaTradePricer::AnalyticBlackScholes {
            rate: 0.05,
            vol: 0.2,
        };
        let price = pricer.price(100.0, 100.0, 1.0, 1.0);

        // ATM BS call with S=K=100, r=5%, vol=20%, T=1y should be ~$10.45.
        assert!(price > 8.0 && price < 15.0, "ATM BS call = {price}");
    }

    #[test]
    fn test_bs_call_deep_itm() {
        let pricer = XvaTradePricer::AnalyticBlackScholes {
            rate: 0.05,
            vol: 0.2,
        };
        let price = pricer.price(150.0, 100.0, 1.0, 1.0);

        // Deep ITM: intrinsic ~ 50, price should be close to
        // S - K*exp(-rT) ~ 150 - 95.12 = 54.88.
        assert!(price > 50.0, "deep ITM BS call = {price}");
    }

    #[test]
    fn test_bs_call_deep_otm() {
        let pricer = XvaTradePricer::AnalyticBlackScholes {
            rate: 0.05,
            vol: 0.2,
        };
        let price = pricer.price(50.0, 100.0, 1.0, 1.0);

        // Deep OTM: price should be near zero.
        assert!(price < 1.0, "deep OTM BS call = {price}");
        assert!(price >= 0.0, "BS call price must be non-negative");
    }

    #[test]
    fn test_bs_call_at_expiry() {
        let pricer = XvaTradePricer::AnalyticBlackScholes {
            rate: 0.05,
            vol: 0.2,
        };

        // ITM at expiry.
        let price = pricer.price(110.0, 100.0, 0.0, 1.0);
        assert_relative_eq!(price, 10.0, epsilon = 1e-10);

        // OTM at expiry.
        let price = pricer.price(90.0, 100.0, 0.0, 1.0);
        assert_relative_eq!(price, 0.0, epsilon = 1e-10);
    }

    #[test]
    fn test_bs_call_zero_vol() {
        let pricer = XvaTradePricer::AnalyticBlackScholes {
            rate: 0.05,
            vol: 0.0,
        };
        let price = pricer.price(100.0, 95.0, 1.0, 1.0);

        // Forward = 100 * exp(0.05) ~ 105.13, PV = (105.13 - 95) * exp(-0.05) ~ 9.63.
        let expected = (100.0 * 0.05_f64.exp() - 95.0).max(0.0) * (-0.05_f64).exp();
        assert_relative_eq!(price, expected, epsilon = 1e-8);
    }

    #[test]
    fn test_bs_call_notional() {
        let pricer = XvaTradePricer::AnalyticBlackScholes {
            rate: 0.05,
            vol: 0.2,
        };
        let price_1 = pricer.price(100.0, 100.0, 1.0, 1.0);
        let price_100 = pricer.price(100.0, 100.0, 1.0, 100.0);

        assert_relative_eq!(price_100, 100.0 * price_1, epsilon = 1e-10);
    }

    // --- Forward tests ---

    #[test]
    fn test_forward_basic() {
        let pricer = XvaTradePricer::Forward { rate: 0.05 };
        let price = pricer.price(100.0, 95.0, 1.0, 1.0);

        // PV = S - K * exp(-r*T) = 100 - 95 * exp(-0.05) ~ 100 - 90.37 = 9.63
        let expected = 100.0 - 95.0 * (-0.05_f64).exp();
        assert_relative_eq!(price, expected, epsilon = 1e-8);
    }

    #[test]
    fn test_forward_atm() {
        let pricer = XvaTradePricer::Forward { rate: 0.05 };
        // ATM forward: S = K * exp(-r*T).
        let strike = 100.0 * 0.05_f64.exp();
        let price = pricer.price(100.0, strike, 1.0, 1.0);

        // Should be approximately zero.
        assert_relative_eq!(price, 0.0, epsilon = 1e-8);
    }

    #[test]
    fn test_forward_at_expiry() {
        let pricer = XvaTradePricer::Forward { rate: 0.05 };
        let price = pricer.price(110.0, 100.0, 0.0, 1.0);
        assert_relative_eq!(price, 10.0, epsilon = 1e-10);
    }

    #[test]
    fn test_forward_notional() {
        let pricer = XvaTradePricer::Forward { rate: 0.05 };
        let price_1 = pricer.price(100.0, 95.0, 1.0, 1.0);
        let price_1000 = pricer.price(100.0, 95.0, 1.0, 1_000.0);

        assert_relative_eq!(price_1000, 1_000.0 * price_1, epsilon = 1e-8);
    }

    // --- Swap tests ---

    #[test]
    fn test_swap_atm() {
        let fixed_rate = 0.05;
        let pricer = XvaTradePricer::Swap {
            fixed_rate,
            payment_freq: 0.5,
        };

        // When floating = fixed, NPV should be approximately zero.
        let price = pricer.price(0.05, 0.0, 5.0, 1_000_000.0);
        assert_relative_eq!(price, 0.0, epsilon = 1.0);
    }

    #[test]
    fn test_swap_floating_above_fixed() {
        let pricer = XvaTradePricer::Swap {
            fixed_rate: 0.03,
            payment_freq: 0.5,
        };

        // Floating > fixed => positive NPV for receiver of floating.
        let price = pricer.price(0.05, 0.0, 5.0, 1_000_000.0);
        assert!(
            price > 0.0,
            "swap NPV should be positive when floating > fixed, got {price}"
        );
    }

    #[test]
    fn test_swap_floating_below_fixed() {
        let pricer = XvaTradePricer::Swap {
            fixed_rate: 0.05,
            payment_freq: 0.5,
        };

        // Floating < fixed => negative NPV for receiver of floating.
        let price = pricer.price(0.03, 0.0, 5.0, 1_000_000.0);
        assert!(
            price < 0.0,
            "swap NPV should be negative when floating < fixed, got {price}"
        );
    }

    #[test]
    fn test_swap_at_maturity() {
        let pricer = XvaTradePricer::Swap {
            fixed_rate: 0.05,
            payment_freq: 0.5,
        };
        let price = pricer.price(0.04, 0.0, 0.0, 1_000_000.0);
        assert_relative_eq!(price, 0.0, epsilon = 1e-10);
    }

    #[test]
    fn test_swap_notional_scaling() {
        let pricer = XvaTradePricer::Swap {
            fixed_rate: 0.03,
            payment_freq: 0.25,
        };

        let price_1 = pricer.price(0.05, 0.0, 2.0, 1.0);
        let price_1m = pricer.price(0.05, 0.0, 2.0, 1_000_000.0);
        assert_relative_eq!(price_1m, 1_000_000.0 * price_1, epsilon = 1e-6);
    }

    #[test]
    fn test_swap_zero_payment_freq() {
        let pricer = XvaTradePricer::Swap {
            fixed_rate: 0.05,
            payment_freq: 0.0,
        };
        let price = pricer.price(0.04, 0.0, 5.0, 1_000_000.0);
        assert_relative_eq!(price, 0.0, epsilon = 1e-10);
    }
}
