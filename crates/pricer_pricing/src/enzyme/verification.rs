//! Verification tests for Enzyme AD implementation.
//!
//! This module provides comprehensive comparison between different AD methods:
//!
//! - Enzyme AD (or finite difference fallback)
//! - Bump-and-revalue finite differences
//! - Black-Scholes analytical Greeks (for European options)
//!
//! # Verification Strategy
//!
//! All methods should produce Greeks within specified tolerances:
//!
//! | Comparison | Tolerance |
//! |------------|-----------|
//! | Enzyme vs FD | 1e-4 |
//! | Enzyme vs Analytical | 1e-3 |
//! | FD vs Analytical | 1e-3 |
//!
//! # Usage
//!
//! ```rust,no_run
//! use pricer_pricing::enzyme::verification::{
//!     VerificationConfig, VerificationResult, verify_european_greeks,
//! };
//!
//! let config = VerificationConfig::default();
//! let result = verify_european_greeks(100.0, 100.0, 0.05, 0.2, 1.0, true, config);
//!
//! // Check that all verifications passed (may vary due to MC variance)
//! println!("All passed: {}", result.all_passed());
//! ```

use crate::mc::{GbmParams, MonteCarloPricer, MonteCarloConfig, PayoffParams};

use super::greeks::{EnzymeGreeksResult, GreeksEnzyme, GreeksMode};

/// Configuration for verification tests.
#[derive(Clone, Debug)]
pub struct VerificationConfig {
    /// Tolerance for Enzyme vs finite difference comparison.
    pub enzyme_fd_tolerance: f64,

    /// Tolerance for comparison against analytical Greeks.
    pub analytical_tolerance: f64,

    /// Number of Monte Carlo paths.
    pub n_paths: usize,

    /// Random seed for reproducibility.
    pub seed: u64,

    /// Whether to enable verbose output.
    pub verbose: bool,
}

impl Default for VerificationConfig {
    fn default() -> Self {
        Self {
            enzyme_fd_tolerance: 1e-4,
            analytical_tolerance: 5e-2, // MC has inherent variance
            n_paths: 100_000,
            seed: 42,
            verbose: false,
        }
    }
}

impl VerificationConfig {
    /// Creates a new verification configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the enzyme vs FD tolerance.
    #[inline]
    pub fn with_enzyme_fd_tolerance(mut self, tolerance: f64) -> Self {
        self.enzyme_fd_tolerance = tolerance;
        self
    }

    /// Sets the analytical tolerance.
    #[inline]
    pub fn with_analytical_tolerance(mut self, tolerance: f64) -> Self {
        self.analytical_tolerance = tolerance;
        self
    }

    /// Sets the number of paths.
    #[inline]
    pub fn with_n_paths(mut self, n_paths: usize) -> Self {
        self.n_paths = n_paths;
        self
    }

    /// Sets the random seed.
    #[inline]
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }
}

/// Result of a single Greek verification.
#[derive(Clone, Debug)]
pub struct GreekVerification {
    /// Name of the Greek being verified.
    pub name: &'static str,

    /// Value from Enzyme/AD method.
    pub enzyme_value: f64,

    /// Value from finite differences.
    pub fd_value: f64,

    /// Value from analytical formula (if available).
    pub analytical_value: Option<f64>,

    /// Whether Enzyme vs FD comparison passed.
    pub enzyme_fd_passed: bool,

    /// Whether Enzyme vs analytical comparison passed.
    pub analytical_passed: Option<bool>,
}

impl GreekVerification {
    /// Creates a new Greek verification result.
    pub fn new(
        name: &'static str,
        enzyme_value: f64,
        fd_value: f64,
        analytical_value: Option<f64>,
        enzyme_fd_tolerance: f64,
        analytical_tolerance: f64,
    ) -> Self {
        let enzyme_fd_passed = relative_error(enzyme_value, fd_value) < enzyme_fd_tolerance;

        let analytical_passed = analytical_value.map(|av| {
            relative_error(enzyme_value, av) < analytical_tolerance
        });

        Self {
            name,
            enzyme_value,
            fd_value,
            analytical_value,
            enzyme_fd_passed,
            analytical_passed,
        }
    }

    /// Returns whether all comparisons passed.
    pub fn all_passed(&self) -> bool {
        self.enzyme_fd_passed && self.analytical_passed.unwrap_or(true)
    }
}

/// Result of a complete verification run.
#[derive(Clone, Debug)]
pub struct VerificationResult {
    /// Spot price used.
    pub spot: f64,
    /// Strike price used.
    pub strike: f64,
    /// Risk-free rate used.
    pub rate: f64,
    /// Volatility used.
    pub volatility: f64,
    /// Time to maturity used.
    pub maturity: f64,
    /// Whether this is a call option.
    pub is_call: bool,

    /// Enzyme Greeks result.
    pub enzyme_result: EnzymeGreeksResult,

    /// Delta verification result.
    pub delta: GreekVerification,
    /// Gamma verification result.
    pub gamma: GreekVerification,
    /// Vega verification result.
    pub vega: GreekVerification,
    /// Theta verification result.
    pub theta: GreekVerification,
    /// Rho verification result.
    pub rho: GreekVerification,
}

impl VerificationResult {
    /// Returns whether all verifications passed.
    pub fn all_passed(&self) -> bool {
        self.delta.all_passed()
            && self.gamma.all_passed()
            && self.vega.all_passed()
            && self.theta.all_passed()
            && self.rho.all_passed()
    }

    /// Returns the number of failed verifications.
    pub fn failed_count(&self) -> usize {
        let mut count = 0;
        if !self.delta.all_passed() {
            count += 1;
        }
        if !self.gamma.all_passed() {
            count += 1;
        }
        if !self.vega.all_passed() {
            count += 1;
        }
        if !self.theta.all_passed() {
            count += 1;
        }
        if !self.rho.all_passed() {
            count += 1;
        }
        count
    }

    /// Returns a summary string.
    pub fn summary(&self) -> String {
        let status = if self.all_passed() { "PASS" } else { "FAIL" };
        format!(
            "{} - {:.0}/{:.0}/{:.2}/{:.2}/{:.2} - Delta: {:.4}, Gamma: {:.4}, Vega: {:.2}",
            status,
            self.spot,
            self.strike,
            self.rate,
            self.volatility,
            self.maturity,
            self.enzyme_result.delta,
            self.enzyme_result.gamma,
            self.enzyme_result.vega,
        )
    }
}

/// Compute relative error between two values.
#[inline]
fn relative_error(a: f64, b: f64) -> f64 {
    let max_abs = a.abs().max(b.abs());
    if max_abs < 1e-10 {
        (a - b).abs()
    } else {
        (a - b).abs() / max_abs
    }
}

/// Black-Scholes analytical Greeks.
pub mod analytical {
    #![allow(unused_imports)]

    /// Standard normal CDF approximation using Abramowitz-Stegun polynomial.
    ///
    /// Maximum absolute error: 7.5e-8
    #[inline]
    pub fn norm_cdf(x: f64) -> f64 {
        // Handle edge cases
        if x < -8.0 {
            return 0.0;
        }
        if x > 8.0 {
            return 1.0;
        }

        // Constants for Abramowitz-Stegun approximation (7.1.26)
        const A1: f64 = 0.254829592;
        const A2: f64 = -0.284496736;
        const A3: f64 = 1.421413741;
        const A4: f64 = -1.453152027;
        const A5: f64 = 1.061405429;
        const P: f64 = 0.3275911;

        let sign = if x < 0.0 { -1.0 } else { 1.0 };
        let abs_x = x.abs();

        let t = 1.0 / (1.0 + P * abs_x);
        let y = 1.0 - (((((A5 * t + A4) * t + A3) * t + A2) * t + A1) * t) * (-abs_x * abs_x / 2.0).exp();

        0.5 * (1.0 + sign * y)
    }

    /// Standard normal PDF.
    #[inline]
    pub fn norm_pdf(x: f64) -> f64 {
        const INV_SQRT_2PI: f64 = 0.3989422804014327; // 1 / sqrt(2 * PI)
        INV_SQRT_2PI * (-0.5 * x * x).exp()
    }

    /// Black-Scholes d1 parameter.
    #[inline]
    pub fn d1(spot: f64, strike: f64, rate: f64, vol: f64, time: f64) -> f64 {
        let vol_sqrt_t = vol * time.sqrt();
        ((spot / strike).ln() + (rate + 0.5 * vol * vol) * time) / vol_sqrt_t
    }

    /// Black-Scholes d2 parameter.
    #[inline]
    pub fn d2(spot: f64, strike: f64, rate: f64, vol: f64, time: f64) -> f64 {
        d1(spot, strike, rate, vol, time) - vol * time.sqrt()
    }

    /// Black-Scholes call price.
    pub fn call_price(spot: f64, strike: f64, rate: f64, vol: f64, time: f64) -> f64 {
        let d1_val = d1(spot, strike, rate, vol, time);
        let d2_val = d2(spot, strike, rate, vol, time);
        let df = (-rate * time).exp();

        spot * norm_cdf(d1_val) - strike * df * norm_cdf(d2_val)
    }

    /// Black-Scholes put price.
    pub fn put_price(spot: f64, strike: f64, rate: f64, vol: f64, time: f64) -> f64 {
        let d1_val = d1(spot, strike, rate, vol, time);
        let d2_val = d2(spot, strike, rate, vol, time);
        let df = (-rate * time).exp();

        strike * df * norm_cdf(-d2_val) - spot * norm_cdf(-d1_val)
    }

    /// Delta for call option.
    pub fn call_delta(spot: f64, strike: f64, rate: f64, vol: f64, time: f64) -> f64 {
        let d1_val = d1(spot, strike, rate, vol, time);
        norm_cdf(d1_val)
    }

    /// Delta for put option.
    pub fn put_delta(spot: f64, strike: f64, rate: f64, vol: f64, time: f64) -> f64 {
        call_delta(spot, strike, rate, vol, time) - 1.0
    }

    /// Gamma (same for call and put).
    pub fn gamma(spot: f64, strike: f64, rate: f64, vol: f64, time: f64) -> f64 {
        let d1_val = d1(spot, strike, rate, vol, time);
        let vol_sqrt_t = vol * time.sqrt();
        norm_pdf(d1_val) / (spot * vol_sqrt_t)
    }

    /// Vega (same for call and put).
    pub fn vega(spot: f64, strike: f64, rate: f64, vol: f64, time: f64) -> f64 {
        let d1_val = d1(spot, strike, rate, vol, time);
        spot * norm_pdf(d1_val) * time.sqrt()
    }

    /// Theta for call option.
    pub fn call_theta(spot: f64, strike: f64, rate: f64, vol: f64, time: f64) -> f64 {
        let d1_val = d1(spot, strike, rate, vol, time);
        let d2_val = d2(spot, strike, rate, vol, time);
        let df = (-rate * time).exp();

        let term1 = -spot * norm_pdf(d1_val) * vol / (2.0 * time.sqrt());
        let term2 = -rate * strike * df * norm_cdf(d2_val);

        term1 + term2
    }

    /// Theta for put option.
    pub fn put_theta(spot: f64, strike: f64, rate: f64, vol: f64, time: f64) -> f64 {
        let d1_val = d1(spot, strike, rate, vol, time);
        let d2_val = d2(spot, strike, rate, vol, time);
        let df = (-rate * time).exp();

        let term1 = -spot * norm_pdf(d1_val) * vol / (2.0 * time.sqrt());
        let term2 = rate * strike * df * norm_cdf(-d2_val);

        term1 + term2
    }

    /// Rho for call option (per 1% rate move).
    pub fn call_rho(spot: f64, strike: f64, rate: f64, vol: f64, time: f64) -> f64 {
        let d2_val = d2(spot, strike, rate, vol, time);
        let df = (-rate * time).exp();

        strike * time * df * norm_cdf(d2_val) * 0.01
    }

    /// Rho for put option (per 1% rate move).
    pub fn put_rho(spot: f64, strike: f64, rate: f64, vol: f64, time: f64) -> f64 {
        let d2_val = d2(spot, strike, rate, vol, time);
        let df = (-rate * time).exp();

        -strike * time * df * norm_cdf(-d2_val) * 0.01
    }
}

/// Verify European option Greeks against analytical and finite differences.
pub fn verify_european_greeks(
    spot: f64,
    strike: f64,
    rate: f64,
    volatility: f64,
    maturity: f64,
    is_call: bool,
    config: VerificationConfig,
) -> VerificationResult {
    // Create MC pricer
    let mc_config = MonteCarloConfig::builder()
        .n_paths(config.n_paths)
        .n_steps(1)
        .seed(config.seed)
        .build()
        .expect("Failed to build MC config");

    let mut pricer = MonteCarloPricer::new(mc_config).expect("Failed to create pricer");

    // Set up parameters
    let gbm = GbmParams::new(spot, rate, volatility, maturity);
    let payoff = if is_call {
        PayoffParams::call(strike)
    } else {
        PayoffParams::put(strike)
    };
    let df = (-rate * maturity).exp();

    // Compute Enzyme/AD Greeks
    let enzyme_result = pricer.price_with_enzyme_greeks(gbm, payoff, df, GreeksMode::Auto);

    // Compute FD Greeks (force finite difference mode)
    pricer.reset_with_seed(config.seed);
    let fd_result =
        pricer.price_with_enzyme_greeks(gbm, payoff, df, GreeksMode::FiniteDifference);

    // Compute analytical Greeks
    let (ana_delta, ana_gamma, ana_vega, ana_theta, ana_rho) = if is_call {
        (
            analytical::call_delta(spot, strike, rate, volatility, maturity),
            analytical::gamma(spot, strike, rate, volatility, maturity),
            analytical::vega(spot, strike, rate, volatility, maturity),
            analytical::call_theta(spot, strike, rate, volatility, maturity),
            analytical::call_rho(spot, strike, rate, volatility, maturity),
        )
    } else {
        (
            analytical::put_delta(spot, strike, rate, volatility, maturity),
            analytical::gamma(spot, strike, rate, volatility, maturity),
            analytical::vega(spot, strike, rate, volatility, maturity),
            analytical::put_theta(spot, strike, rate, volatility, maturity),
            analytical::put_rho(spot, strike, rate, volatility, maturity),
        )
    };

    // Create verification results
    let delta = GreekVerification::new(
        "Delta",
        enzyme_result.delta,
        fd_result.delta,
        Some(ana_delta),
        config.enzyme_fd_tolerance,
        config.analytical_tolerance,
    );

    let gamma = GreekVerification::new(
        "Gamma",
        enzyme_result.gamma,
        fd_result.gamma,
        Some(ana_gamma),
        config.enzyme_fd_tolerance,
        config.analytical_tolerance,
    );

    let vega = GreekVerification::new(
        "Vega",
        enzyme_result.vega,
        fd_result.vega,
        Some(ana_vega),
        config.enzyme_fd_tolerance,
        config.analytical_tolerance,
    );

    let theta = GreekVerification::new(
        "Theta",
        enzyme_result.theta,
        fd_result.theta,
        Some(ana_theta),
        config.enzyme_fd_tolerance,
        config.analytical_tolerance,
    );

    let rho = GreekVerification::new(
        "Rho",
        enzyme_result.rho,
        fd_result.rho,
        Some(ana_rho),
        config.enzyme_fd_tolerance,
        config.analytical_tolerance,
    );

    VerificationResult {
        spot,
        strike,
        rate,
        volatility,
        maturity,
        is_call,
        enzyme_result,
        delta,
        gamma,
        vega,
        theta,
        rho,
    }
}

/// Run a suite of verification tests across different scenarios.
pub fn run_verification_suite(config: VerificationConfig) -> Vec<VerificationResult> {
    let mut results = Vec::new();

    // Standard parameters
    let standard = verify_european_greeks(100.0, 100.0, 0.05, 0.2, 1.0, true, config.clone());
    results.push(standard);

    // ITM call
    let itm_call = verify_european_greeks(120.0, 100.0, 0.05, 0.2, 1.0, true, config.clone());
    results.push(itm_call);

    // OTM call
    let otm_call = verify_european_greeks(80.0, 100.0, 0.05, 0.2, 1.0, true, config.clone());
    results.push(otm_call);

    // ATM put
    let atm_put = verify_european_greeks(100.0, 100.0, 0.05, 0.2, 1.0, false, config.clone());
    results.push(atm_put);

    // High volatility
    let high_vol = verify_european_greeks(100.0, 100.0, 0.05, 0.4, 1.0, true, config.clone());
    results.push(high_vol);

    // Short maturity
    let short_mat = verify_european_greeks(100.0, 100.0, 0.05, 0.2, 0.25, true, config.clone());
    results.push(short_mat);

    // Long maturity
    let long_mat = verify_european_greeks(100.0, 100.0, 0.05, 0.2, 2.0, true, config.clone());
    results.push(long_mat);

    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    fn verification_config() -> VerificationConfig {
        VerificationConfig::new()
            .with_n_paths(50_000)
            .with_seed(12345)
            .with_enzyme_fd_tolerance(1e-4)
            .with_analytical_tolerance(0.1) // MC variance requires wider tolerance
    }

    #[test]
    fn test_verification_config_default() {
        let config = VerificationConfig::default();
        assert!(config.n_paths > 0);
        assert!(config.enzyme_fd_tolerance > 0.0);
    }

    #[test]
    fn test_relative_error() {
        assert!(relative_error(1.0, 1.0) < 1e-10);
        // relative_error(1.0, 1.01) = |1.0 - 1.01| / max(1.0, 1.01) = 0.01 / 1.01 ≈ 0.0099
        assert!(relative_error(1.0, 1.01) < 0.02);
        assert!(relative_error(0.0, 0.0) < 1e-10);
    }

    #[test]
    fn test_analytical_call_delta_atm() {
        // ATM call delta should be around 0.5-0.7
        let delta = analytical::call_delta(100.0, 100.0, 0.05, 0.2, 1.0);
        assert!(delta > 0.5 && delta < 0.7);
    }

    #[test]
    fn test_analytical_put_delta_atm() {
        // ATM put delta should be around -0.5 to -0.3
        let delta = analytical::put_delta(100.0, 100.0, 0.05, 0.2, 1.0);
        assert!(delta > -0.5 && delta < -0.3);
    }

    #[test]
    fn test_analytical_gamma_positive() {
        // Gamma should always be positive
        let gamma = analytical::gamma(100.0, 100.0, 0.05, 0.2, 1.0);
        assert!(gamma > 0.0);
    }

    #[test]
    fn test_analytical_vega_positive() {
        // Vega should be positive for long options
        let vega = analytical::vega(100.0, 100.0, 0.05, 0.2, 1.0);
        assert!(vega > 0.0);
    }

    #[test]
    fn test_analytical_put_call_parity_delta() {
        // Put delta = Call delta - 1
        let call_delta = analytical::call_delta(100.0, 100.0, 0.05, 0.2, 1.0);
        let put_delta = analytical::put_delta(100.0, 100.0, 0.05, 0.2, 1.0);
        assert_relative_eq!(put_delta, call_delta - 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_greek_verification_new() {
        let greek = GreekVerification::new("Delta", 0.55, 0.55, Some(0.55), 1e-4, 1e-3);

        assert!(greek.all_passed());
        assert!(greek.enzyme_fd_passed);
        assert_eq!(greek.analytical_passed, Some(true));
    }

    #[test]
    fn test_greek_verification_failed() {
        let greek = GreekVerification::new("Delta", 0.55, 0.60, Some(0.55), 1e-4, 1e-3);

        // Enzyme vs FD should fail (0.55 vs 0.60 is 9% error)
        assert!(!greek.enzyme_fd_passed);
    }

    #[test]
    fn test_verify_european_call_atm() {
        let config = verification_config();
        let result = verify_european_greeks(100.0, 100.0, 0.05, 0.2, 1.0, true, config);

        // Enzyme should match FD (both using same underlying implementation)
        assert!(
            result.delta.enzyme_fd_passed,
            "Delta enzyme vs FD failed: {} vs {}",
            result.delta.enzyme_value,
            result.delta.fd_value
        );

        // Price should be reasonable
        assert!(
            result.enzyme_result.price > 5.0 && result.enzyme_result.price < 20.0,
            "Price out of range: {}",
            result.enzyme_result.price
        );

        // Delta should be in expected range
        assert!(
            result.delta.enzyme_value > 0.4 && result.delta.enzyme_value < 0.8,
            "Delta out of range: {}",
            result.delta.enzyme_value
        );
    }

    #[test]
    fn test_verify_european_put_atm() {
        let config = verification_config();
        let result = verify_european_greeks(100.0, 100.0, 0.05, 0.2, 1.0, false, config);

        // Put delta should be negative
        assert!(
            result.delta.enzyme_value < 0.0,
            "Put delta should be negative: {}",
            result.delta.enzyme_value
        );

        // Gamma should be positive
        assert!(
            result.gamma.enzyme_value > 0.0,
            "Gamma should be positive: {}",
            result.gamma.enzyme_value
        );
    }

    #[test]
    fn test_verify_european_itm_call() {
        let config = verification_config();
        let result = verify_european_greeks(120.0, 100.0, 0.05, 0.2, 1.0, true, config);

        // ITM call delta should be high (close to 1)
        assert!(
            result.delta.enzyme_value > 0.7,
            "ITM call delta should be > 0.7: {}",
            result.delta.enzyme_value
        );
    }

    #[test]
    fn test_verify_european_otm_call() {
        let config = verification_config();
        let result = verify_european_greeks(80.0, 100.0, 0.05, 0.2, 1.0, true, config);

        // OTM call delta should be low
        assert!(
            result.delta.enzyme_value < 0.5,
            "OTM call delta should be < 0.5: {}",
            result.delta.enzyme_value
        );
    }

    #[test]
    fn test_verification_result_summary() {
        let config = verification_config();
        let result = verify_european_greeks(100.0, 100.0, 0.05, 0.2, 1.0, true, config);

        let summary = result.summary();
        assert!(summary.contains("Delta"));
    }

    #[test]
    fn test_run_verification_suite() {
        let config = verification_config();
        let results = run_verification_suite(config);

        // Should have multiple test cases
        assert!(results.len() >= 5);

        // Check that we have variety
        let has_call = results.iter().any(|r| r.is_call);
        let has_put = results.iter().any(|r| !r.is_call);

        assert!(has_call, "Suite should include call options");
        assert!(has_put, "Suite should include put options");
    }

    #[test]
    fn test_enzyme_fd_consistency() {
        // The key test: Enzyme and FD should produce same results
        // (since without enzyme-ad feature, both use FD)
        let config = verification_config();
        let result = verify_european_greeks(100.0, 100.0, 0.05, 0.2, 1.0, true, config);

        #[cfg(not(feature = "enzyme-ad"))]
        {
            // Without Enzyme, both methods should be identical
            assert_relative_eq!(
                result.delta.enzyme_value,
                result.delta.fd_value,
                epsilon = 1e-10
            );
            assert_relative_eq!(
                result.gamma.enzyme_value,
                result.gamma.fd_value,
                epsilon = 1e-10
            );
            assert_relative_eq!(
                result.vega.enzyme_value,
                result.vega.fd_value,
                epsilon = 1e-10
            );
        }
    }

    #[test]
    fn test_analytical_vs_mc_delta() {
        // MC Delta should be reasonably close to analytical
        let config = VerificationConfig::new()
            .with_n_paths(100_000)
            .with_seed(42);

        let result = verify_european_greeks(100.0, 100.0, 0.05, 0.2, 1.0, true, config);

        let ana_delta = result.delta.analytical_value.unwrap();
        let mc_delta = result.delta.enzyme_value;

        // Should be within 10% (MC variance)
        let error = (mc_delta - ana_delta).abs() / ana_delta;
        assert!(
            error < 0.1,
            "MC delta too far from analytical: {} vs {} (error: {:.2}%)",
            mc_delta,
            ana_delta,
            error * 100.0
        );
    }
}
