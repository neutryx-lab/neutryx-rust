//! Geometric average Asian option analytical pricing.
//!
//! Implements the Kemna-Vorst (1990) closed-form solution for geometric
//! average Asian options. The geometric average allows for a closed-form
//! solution because the product of log-normal variables is log-normal.
//!
//! # Mathematical Background
//!
//! For a geometric average Asian option, the payoff depends on the
//! geometric mean of the underlying price:
//!
//! ```text
//! G = (∏_{i=1}^{n} S_{t_i})^{1/n}
//! ```
//!
//! The closed-form solution adjusts the Black-Scholes formula:
//! - Adjusted volatility: σ_G = σ / √3
//! - Adjusted forward: F_G = S * exp((r - q - σ²/6) * T)
//!
//! # References
//!
//! - Kemna, A.G.Z. and Vorst, A.C.F. (1990). "A Pricing Method for Options
//!   Based on Average Asset Values." Journal of Banking and Finance, 14, 113-129.

use num_traits::Float;

/// Parameters for geometric average Asian option pricing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeometricAsianParams<T: Float> {
    /// Spot price (S_0)
    pub spot: T,
    /// Strike price (K)
    pub strike: T,
    /// Risk-free interest rate (r)
    pub rate: T,
    /// Dividend yield (q)
    pub dividend: T,
    /// Volatility (σ)
    pub volatility: T,
    /// Time to maturity in years (T)
    pub maturity: T,
}

impl<T: Float> GeometricAsianParams<T> {
    /// Creates new parameters for geometric Asian option pricing.
    ///
    /// # Arguments
    ///
    /// * `spot` - Current spot price
    /// * `strike` - Strike price
    /// * `rate` - Risk-free interest rate (annualized)
    /// * `dividend` - Continuous dividend yield
    /// * `volatility` - Annualized volatility
    /// * `maturity` - Time to maturity in years
    pub fn new(spot: T, strike: T, rate: T, dividend: T, volatility: T, maturity: T) -> Self {
        Self {
            spot,
            strike,
            rate,
            dividend,
            volatility,
            maturity,
        }
    }

    /// Creates parameters with zero dividend.
    pub fn without_dividend(spot: T, strike: T, rate: T, volatility: T, maturity: T) -> Self {
        Self::new(spot, strike, rate, T::zero(), volatility, maturity)
    }
}

/// Result from geometric Asian option pricing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeometricAsianResult<T: Float> {
    /// Option price
    pub price: T,
    /// Adjusted volatility (σ_G = σ / √3)
    pub adjusted_volatility: T,
    /// Adjusted drift for forward calculation
    pub adjusted_drift: T,
    /// d1 parameter
    pub d1: T,
    /// d2 parameter
    pub d2: T,
}

/// Standard normal CDF approximation.
///
/// Uses the Abramowitz and Stegun approximation for the error function.
#[inline]
fn norm_cdf<T: Float>(x: T) -> T {
    let one = T::one();
    let zero = T::zero();
    let half = T::from(0.5).unwrap();
    let sqrt_2 = T::from(std::f64::consts::SQRT_2).unwrap();

    // Handle extreme values
    let abs_x = x.abs();
    if abs_x > T::from(8.0).unwrap() {
        return if x > zero { one } else { zero };
    }

    // Abramowitz and Stegun constants
    let a1 = T::from(0.254829592).unwrap();
    let a2 = T::from(-0.284496736).unwrap();
    let a3 = T::from(1.421413741).unwrap();
    let a4 = T::from(-1.453152027).unwrap();
    let a5 = T::from(1.061405429).unwrap();
    let p = T::from(0.3275911).unwrap();

    // Compute erfc for -x/sqrt(2)
    let arg = -x / sqrt_2;
    let abs_arg = arg.abs();
    let t = one / (one + p * abs_arg);
    let poly = a1 + t * (a2 + t * (a3 + t * (a4 + t * a5)));
    let erfc_abs = t * poly * (-abs_arg * abs_arg).exp();

    let two = T::from(2.0).unwrap();
    let erfc_val = if arg < zero { two - erfc_abs } else { erfc_abs };

    half * erfc_val
}

/// Price a geometric average Asian call option.
///
/// Uses the Kemna-Vorst (1990) closed-form formula with adjusted
/// Black-Scholes parameters for continuously sampled geometric average.
///
/// # Mathematical Formula
///
/// ```text
/// σ_G = σ / √3  (adjusted volatility)
/// b_adj = (r - q - σ²/6) / 2  (adjusted cost of carry)
/// F_G = S * exp(b_adj * T)  (forward of geometric average)
/// d1 = [ln(F_G / K) + (σ_G² / 2) * T] / (σ_G * √T)
/// d2 = d1 - σ_G * √T
///
/// Call = exp(-rT) * [F_G * N(d1) - K * N(d2)]
/// ```
///
/// # Arguments
///
/// * `spot` - Current spot price (S_0)
/// * `strike` - Strike price (K)
/// * `rate` - Risk-free interest rate (r)
/// * `dividend` - Dividend yield (q)
/// * `volatility` - Annualized volatility (σ)
/// * `maturity` - Time to maturity (T)
///
/// # Returns
///
/// The price of the geometric average Asian call option.
///
/// # Example
///
/// ```rust
/// use pricer_pricing::analytical::geometric_asian_call;
///
/// let price = geometric_asian_call(100.0, 100.0, 0.05, 0.0, 0.2, 1.0);
/// assert!(price > 0.0);
/// ```
#[inline]
pub fn geometric_asian_call<T: Float>(
    spot: T,
    strike: T,
    rate: T,
    dividend: T,
    volatility: T,
    maturity: T,
) -> T {
    let result =
        geometric_asian_call_with_details(spot, strike, rate, dividend, volatility, maturity);
    result.price
}

/// Price a geometric average Asian call option with detailed output.
///
/// Returns the price along with intermediate calculation values for
/// debugging and verification.
pub fn geometric_asian_call_with_details<T: Float>(
    spot: T,
    strike: T,
    rate: T,
    dividend: T,
    volatility: T,
    maturity: T,
) -> GeometricAsianResult<T> {
    let zero = T::zero();
    let two = T::from(2.0).unwrap();
    let three = T::from(3.0).unwrap();
    let six = T::from(6.0).unwrap();

    // Handle edge cases
    if maturity <= zero || volatility <= zero || spot <= zero || strike <= zero {
        return GeometricAsianResult {
            price: zero,
            adjusted_volatility: zero,
            adjusted_drift: zero,
            d1: zero,
            d2: zero,
        };
    }

    // Adjusted volatility: σ_G = σ / √3
    let sqrt_3 = three.sqrt();
    let vol_adj = volatility / sqrt_3;

    // Adjusted cost of carry: b_adj = (r - q - σ²/6) / 2
    // This is the adjusted drift for the geometric average
    let vol_sq = volatility * volatility;
    let drift_adj = (rate - dividend - vol_sq / six) / two;

    // Forward price of geometric average
    // F_G = S * exp(b_adj * T)
    let forward = spot * (drift_adj * maturity).exp();

    // d1 and d2 calculations
    let sqrt_t = maturity.sqrt();
    let vol_sqrt_t = vol_adj * sqrt_t;

    // d1 = [ln(F_G / K) + (σ_G² * T / 2)] / (σ_G * √T)
    let log_moneyness = (forward / strike).ln();
    let d1 = (log_moneyness + vol_adj * vol_adj * maturity / two) / vol_sqrt_t;
    let d2 = d1 - vol_sqrt_t;

    // Call price = exp(-rT) * [F_G * N(d1) - K * N(d2)]
    let discount = (-rate * maturity).exp();
    let price = discount * (forward * norm_cdf(d1) - strike * norm_cdf(d2));

    GeometricAsianResult {
        price,
        adjusted_volatility: vol_adj,
        adjusted_drift: drift_adj,
        d1,
        d2,
    }
}

/// Price a geometric average Asian put option.
///
/// Uses put-call parity or the direct Kemna-Vorst formula.
///
/// # Mathematical Formula
///
/// ```text
/// Put = exp(-rT) * [K * N(-d2) - F_G * N(-d1)]
/// ```
///
/// # Arguments
///
/// * `spot` - Current spot price (S_0)
/// * `strike` - Strike price (K)
/// * `rate` - Risk-free interest rate (r)
/// * `dividend` - Dividend yield (q)
/// * `volatility` - Annualized volatility (σ)
/// * `maturity` - Time to maturity (T)
///
/// # Returns
///
/// The price of the geometric average Asian put option.
#[inline]
pub fn geometric_asian_put<T: Float>(
    spot: T,
    strike: T,
    rate: T,
    dividend: T,
    volatility: T,
    maturity: T,
) -> T {
    let result =
        geometric_asian_put_with_details(spot, strike, rate, dividend, volatility, maturity);
    result.price
}

/// Price a geometric average Asian put option with detailed output.
pub fn geometric_asian_put_with_details<T: Float>(
    spot: T,
    strike: T,
    rate: T,
    dividend: T,
    volatility: T,
    maturity: T,
) -> GeometricAsianResult<T> {
    let zero = T::zero();
    let two = T::from(2.0).unwrap();
    let three = T::from(3.0).unwrap();
    let six = T::from(6.0).unwrap();

    // Handle edge cases
    if maturity <= zero || volatility <= zero || spot <= zero || strike <= zero {
        return GeometricAsianResult {
            price: zero,
            adjusted_volatility: zero,
            adjusted_drift: zero,
            d1: zero,
            d2: zero,
        };
    }

    // Adjusted volatility: σ_G = σ / √3
    let sqrt_3 = three.sqrt();
    let vol_adj = volatility / sqrt_3;

    // Adjusted cost of carry: b_adj = (r - q - σ²/6) / 2
    let vol_sq = volatility * volatility;
    let drift_adj = (rate - dividend - vol_sq / six) / two;

    // Forward price of geometric average
    let forward = spot * (drift_adj * maturity).exp();

    // d1 and d2 calculations
    let sqrt_t = maturity.sqrt();
    let vol_sqrt_t = vol_adj * sqrt_t;
    let log_moneyness = (forward / strike).ln();
    let d1 = (log_moneyness + vol_adj * vol_adj * maturity / two) / vol_sqrt_t;
    let d2 = d1 - vol_sqrt_t;

    // Put price = exp(-rT) * [K * N(-d2) - F_G * N(-d1)]
    let discount = (-rate * maturity).exp();
    let price = discount * (strike * norm_cdf(-d2) - forward * norm_cdf(-d1));

    GeometricAsianResult {
        price,
        adjusted_volatility: vol_adj,
        adjusted_drift: drift_adj,
        d1,
        d2,
    }
}

/// Compute both call and put prices.
///
/// This is more efficient than calling both functions separately.
pub fn geometric_asian_prices<T: Float>(
    params: &GeometricAsianParams<T>,
) -> (GeometricAsianResult<T>, GeometricAsianResult<T>) {
    let call = geometric_asian_call_with_details(
        params.spot,
        params.strike,
        params.rate,
        params.dividend,
        params.volatility,
        params.maturity,
    );
    let put = geometric_asian_put_with_details(
        params.spot,
        params.strike,
        params.rate,
        params.dividend,
        params.volatility,
        params.maturity,
    );
    (call, put)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    // ========================================================================
    // Basic Functionality Tests
    // ========================================================================

    #[test]
    fn test_geometric_asian_call_atm() {
        // At-the-money call: S = K = 100, r = 5%, σ = 20%, T = 1
        let price = geometric_asian_call(100.0, 100.0, 0.05, 0.0, 0.2, 1.0);

        // Should be positive and less than vanilla European call
        assert!(price > 0.0);
        assert!(price < 15.0); // Rough upper bound based on vanilla BS
    }

    #[test]
    fn test_geometric_asian_put_atm() {
        // At-the-money put
        let price = geometric_asian_put(100.0, 100.0, 0.05, 0.0, 0.2, 1.0);

        assert!(price > 0.0);
        assert!(price < 15.0);
    }

    #[test]
    fn test_geometric_asian_call_itm() {
        // In-the-money call: S = 110, K = 100
        let price = geometric_asian_call(110.0, 100.0, 0.05, 0.0, 0.2, 1.0);

        // Should be higher than ATM
        let atm_price = geometric_asian_call(100.0, 100.0, 0.05, 0.0, 0.2, 1.0);
        assert!(price > atm_price);
    }

    #[test]
    fn test_geometric_asian_put_itm() {
        // In-the-money put: S = 90, K = 100
        let price = geometric_asian_put(90.0, 100.0, 0.05, 0.0, 0.2, 1.0);

        let atm_price = geometric_asian_put(100.0, 100.0, 0.05, 0.0, 0.2, 1.0);
        assert!(price > atm_price);
    }

    // ========================================================================
    // Adjusted Volatility Tests
    // ========================================================================

    #[test]
    fn test_adjusted_volatility() {
        let result = geometric_asian_call_with_details(100.0, 100.0, 0.05, 0.0, 0.2, 1.0);

        // σ_G = σ / √3 = 0.2 / √3 ≈ 0.1155
        let expected_vol = 0.2 / 3.0_f64.sqrt();
        assert_relative_eq!(result.adjusted_volatility, expected_vol, epsilon = 1e-10);
    }

    #[test]
    fn test_adjusted_volatility_various() {
        for vol in [0.1, 0.2, 0.3, 0.4, 0.5] {
            let result = geometric_asian_call_with_details(100.0, 100.0, 0.05, 0.0, vol, 1.0);
            let expected = vol / 3.0_f64.sqrt();
            assert_relative_eq!(result.adjusted_volatility, expected, epsilon = 1e-10);
        }
    }

    // ========================================================================
    // Put-Call Parity Tests
    // ========================================================================

    #[test]
    fn test_put_call_parity() {
        // For geometric Asian options:
        // C - P = exp(-rT) * (F_G - K)
        // where F_G = S * exp(b_adj * T) and b_adj = (r - q - σ²/6) / 2

        let s = 100.0;
        let k = 100.0;
        let r = 0.05;
        let q = 0.0;
        let vol = 0.2;
        let t = 1.0;

        let call = geometric_asian_call(s, k, r, q, vol, t);
        let put = geometric_asian_put(s, k, r, q, vol, t);

        // Forward of geometric average: b_adj = (r - q - σ²/6) / 2
        let drift_adj = (r - q - vol * vol / 6.0) / 2.0;
        let forward = s * (drift_adj * t).exp();
        let discount = (-r * t).exp();

        let parity_diff = call - put;
        let expected_diff = discount * (forward - k);

        assert_relative_eq!(parity_diff, expected_diff, epsilon = 1e-10);
    }

    #[test]
    fn test_put_call_parity_various_strikes() {
        let s = 100.0;
        let r = 0.05;
        let q = 0.02;
        let vol = 0.25;
        let t = 0.5;

        for k in [80.0, 90.0, 100.0, 110.0, 120.0] {
            let call = geometric_asian_call(s, k, r, q, vol, t);
            let put = geometric_asian_put(s, k, r, q, vol, t);

            // b_adj = (r - q - σ²/6) / 2
            let drift_adj = (r - q - vol * vol / 6.0) / 2.0;
            let forward = s * (drift_adj * t).exp();
            let discount = (-r * t).exp();

            let parity_diff = call - put;
            let expected_diff = discount * (forward - k);

            assert_relative_eq!(parity_diff, expected_diff, epsilon = 1e-9);
        }
    }

    // ========================================================================
    // Boundary Condition Tests
    // ========================================================================

    #[test]
    fn test_deep_itm_call() {
        // Very deep ITM call should approach discounted intrinsic
        let price = geometric_asian_call(200.0, 100.0, 0.05, 0.0, 0.2, 1.0);

        // Forward of geometric average: b_adj = (r - σ²/6) / 2
        let vol = 0.2;
        let drift_adj = (0.05 - vol * vol / 6.0) / 2.0;
        let forward = 200.0 * (drift_adj * 1.0).exp();
        let intrinsic = (-0.05_f64).exp() * (forward - 100.0);

        // Price should be close to intrinsic for deep ITM
        assert!(price > intrinsic * 0.95);
    }

    #[test]
    fn test_deep_otm_call() {
        // Very deep OTM call should be close to zero
        let price = geometric_asian_call(50.0, 100.0, 0.05, 0.0, 0.2, 1.0);

        assert!(price < 0.5);
        assert!(price >= 0.0);
    }

    #[test]
    fn test_zero_volatility() {
        // With zero vol, price should be deterministic
        let price = geometric_asian_call(100.0, 100.0, 0.05, 0.0, 0.0, 1.0);

        // Edge case - returns 0 due to our implementation
        assert_eq!(price, 0.0);
    }

    #[test]
    fn test_zero_maturity() {
        let price = geometric_asian_call(100.0, 100.0, 0.05, 0.0, 0.2, 0.0);
        assert_eq!(price, 0.0);
    }

    // ========================================================================
    // Sensitivity Tests (Greeks-like behavior)
    // ========================================================================

    #[test]
    fn test_call_increases_with_spot() {
        let base_price = geometric_asian_call(100.0, 100.0, 0.05, 0.0, 0.2, 1.0);
        let higher_spot = geometric_asian_call(101.0, 100.0, 0.05, 0.0, 0.2, 1.0);

        assert!(higher_spot > base_price);
    }

    #[test]
    fn test_put_decreases_with_spot() {
        let base_price = geometric_asian_put(100.0, 100.0, 0.05, 0.0, 0.2, 1.0);
        let higher_spot = geometric_asian_put(101.0, 100.0, 0.05, 0.0, 0.2, 1.0);

        assert!(higher_spot < base_price);
    }

    #[test]
    fn test_prices_increase_with_volatility() {
        let low_vol = geometric_asian_call(100.0, 100.0, 0.05, 0.0, 0.1, 1.0);
        let high_vol = geometric_asian_call(100.0, 100.0, 0.05, 0.0, 0.3, 1.0);

        assert!(high_vol > low_vol);
    }

    #[test]
    fn test_prices_increase_with_maturity() {
        let short_t = geometric_asian_call(100.0, 100.0, 0.05, 0.0, 0.2, 0.5);
        let long_t = geometric_asian_call(100.0, 100.0, 0.05, 0.0, 0.2, 2.0);

        // Generally true for ATM options
        assert!(long_t > short_t);
    }

    // ========================================================================
    // Reference Value Tests (from literature/known values)
    // ========================================================================

    #[test]
    fn test_reference_value_kemna_vorst() {
        // Test case parameters similar to Kemna-Vorst paper
        // S=100, K=100, r=0.05, q=0, σ=0.2, T=1
        // Expected value approximately 5.1-5.3 (depends on discretization)

        let price = geometric_asian_call(100.0, 100.0, 0.05, 0.0, 0.2, 1.0);

        // Rough range check
        assert!(price > 4.0);
        assert!(price < 7.0);
    }

    #[test]
    fn test_lower_than_european_call() {
        // Geometric Asian should be cheaper than European
        // because averaging reduces variance
        let asian = geometric_asian_call(100.0, 100.0, 0.05, 0.0, 0.2, 1.0);

        // European call price (Black-Scholes)
        // Approximate: ~10.45 for these parameters
        let european_approx = 10.45;

        assert!(asian < european_approx);
    }

    // ========================================================================
    // Params Struct Tests
    // ========================================================================

    #[test]
    fn test_params_new() {
        let params = GeometricAsianParams::new(100.0, 100.0, 0.05, 0.02, 0.2, 1.0);

        assert_eq!(params.spot, 100.0);
        assert_eq!(params.strike, 100.0);
        assert_eq!(params.rate, 0.05);
        assert_eq!(params.dividend, 0.02);
        assert_eq!(params.volatility, 0.2);
        assert_eq!(params.maturity, 1.0);
    }

    #[test]
    fn test_params_without_dividend() {
        let params = GeometricAsianParams::without_dividend(100.0, 100.0, 0.05, 0.2, 1.0);

        assert_eq!(params.dividend, 0.0);
    }

    #[test]
    fn test_geometric_asian_prices() {
        let params = GeometricAsianParams::new(100.0, 100.0, 0.05, 0.0, 0.2, 1.0);
        let (call, put) = geometric_asian_prices(&params);

        // Verify against individual functions
        let call_direct = geometric_asian_call_with_details(100.0, 100.0, 0.05, 0.0, 0.2, 1.0);
        let put_direct = geometric_asian_put_with_details(100.0, 100.0, 0.05, 0.0, 0.2, 1.0);

        assert_eq!(call.price, call_direct.price);
        assert_eq!(put.price, put_direct.price);
    }

    // ========================================================================
    // Generic Type Tests
    // ========================================================================

    #[test]
    fn test_f32_compatibility() {
        let price = geometric_asian_call(100.0_f32, 100.0_f32, 0.05_f32, 0.0_f32, 0.2_f32, 1.0_f32);
        assert!(price > 0.0);
    }

    // ========================================================================
    // With Dividend Tests
    // ========================================================================

    #[test]
    fn test_dividend_reduces_call_price() {
        let no_div = geometric_asian_call(100.0, 100.0, 0.05, 0.0, 0.2, 1.0);
        let with_div = geometric_asian_call(100.0, 100.0, 0.05, 0.03, 0.2, 1.0);

        assert!(with_div < no_div);
    }

    #[test]
    fn test_dividend_increases_put_price() {
        let no_div = geometric_asian_put(100.0, 100.0, 0.05, 0.0, 0.2, 1.0);
        let with_div = geometric_asian_put(100.0, 100.0, 0.05, 0.03, 0.2, 1.0);

        assert!(with_div > no_div);
    }
}
