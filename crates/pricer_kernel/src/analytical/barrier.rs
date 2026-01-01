//! Barrier option analytical pricing.
//!
//! Implements Rubinstein-Reiner (1991) closed-form solutions for European barrier options.
//!
//! # Barrier Types
//!
//! There are 8 types of single-barrier options:
//! - **Down-and-In/Out Call**: Barrier below spot
//! - **Down-and-In/Out Put**: Barrier below spot
//! - **Up-and-In/Out Call**: Barrier above spot
//! - **Up-and-In/Out Put**: Barrier above spot
//!
//! # Key Relationship
//!
//! **In-Out Parity**: Knock-In + Knock-Out = Vanilla

use num_traits::Float;

/// Barrier option direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BarrierDirection {
    /// Barrier is above the current spot price
    Up,
    /// Barrier is below the current spot price
    Down,
}

/// Barrier option knock type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KnockType {
    /// Option is activated (knocked in) when barrier is hit
    In,
    /// Option is deactivated (knocked out) when barrier is hit
    Out,
}

/// Option type (call or put).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OptionType {
    /// Call option
    Call,
    /// Put option
    Put,
}

/// Complete barrier type specification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BarrierType {
    /// Direction of barrier (up or down)
    pub direction: BarrierDirection,
    /// Knock type (in or out)
    pub knock: KnockType,
    /// Option type (call or put)
    pub option_type: OptionType,
}

impl BarrierType {
    /// Creates a new barrier type specification.
    pub fn new(direction: BarrierDirection, knock: KnockType, option_type: OptionType) -> Self {
        Self {
            direction,
            knock,
            option_type,
        }
    }

    /// Down-and-In Call
    pub fn down_in_call() -> Self {
        Self::new(BarrierDirection::Down, KnockType::In, OptionType::Call)
    }
    /// Down-and-Out Call
    pub fn down_out_call() -> Self {
        Self::new(BarrierDirection::Down, KnockType::Out, OptionType::Call)
    }
    /// Down-and-In Put
    pub fn down_in_put() -> Self {
        Self::new(BarrierDirection::Down, KnockType::In, OptionType::Put)
    }
    /// Down-and-Out Put
    pub fn down_out_put() -> Self {
        Self::new(BarrierDirection::Down, KnockType::Out, OptionType::Put)
    }
    /// Up-and-In Call
    pub fn up_in_call() -> Self {
        Self::new(BarrierDirection::Up, KnockType::In, OptionType::Call)
    }
    /// Up-and-Out Call
    pub fn up_out_call() -> Self {
        Self::new(BarrierDirection::Up, KnockType::Out, OptionType::Call)
    }
    /// Up-and-In Put
    pub fn up_in_put() -> Self {
        Self::new(BarrierDirection::Up, KnockType::In, OptionType::Put)
    }
    /// Up-and-Out Put
    pub fn up_out_put() -> Self {
        Self::new(BarrierDirection::Up, KnockType::Out, OptionType::Put)
    }
}

/// Parameters for barrier option pricing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BarrierParams<T: Float> {
    /// Spot price
    pub spot: T,
    /// Strike price
    pub strike: T,
    /// Barrier level
    pub barrier: T,
    /// Risk-free rate
    pub rate: T,
    /// Dividend yield
    pub dividend: T,
    /// Volatility
    pub volatility: T,
    /// Time to maturity
    pub maturity: T,
    /// Barrier type specification
    pub barrier_type: BarrierType,
}

impl<T: Float> BarrierParams<T> {
    /// Creates new barrier option parameters.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        spot: T,
        strike: T,
        barrier: T,
        rate: T,
        dividend: T,
        volatility: T,
        maturity: T,
        barrier_type: BarrierType,
    ) -> Self {
        Self {
            spot,
            strike,
            barrier,
            rate,
            dividend,
            volatility,
            maturity,
            barrier_type,
        }
    }
}

/// Result from barrier option pricing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BarrierResult<T: Float> {
    /// Option price
    pub price: T,
    /// λ parameter
    pub lambda: T,
    /// y parameter
    pub y: T,
    /// x1 parameter
    pub x1: T,
    /// y1 parameter
    pub y1: T,
}

/// Standard normal CDF using Abramowitz-Stegun approximation.
#[inline]
fn norm_cdf<T: Float>(x: T) -> T {
    let one = T::one();
    let zero = T::zero();
    let half = T::from(0.5).unwrap();
    let sqrt_2 = T::from(std::f64::consts::SQRT_2).unwrap();

    if x.abs() > T::from(8.0).unwrap() {
        return if x > zero { one } else { zero };
    }

    let a1 = T::from(0.254829592).unwrap();
    let a2 = T::from(-0.284496736).unwrap();
    let a3 = T::from(1.421413741).unwrap();
    let a4 = T::from(-1.453152027).unwrap();
    let a5 = T::from(1.061405429).unwrap();
    let p = T::from(0.3275911).unwrap();

    let arg = -x / sqrt_2;
    let abs_arg = arg.abs();
    let t = one / (one + p * abs_arg);
    let poly = a1 + t * (a2 + t * (a3 + t * (a4 + t * a5)));
    let erfc_abs = t * poly * (-abs_arg * abs_arg).exp();

    let two = T::from(2.0).unwrap();
    let erfc_val = if arg < zero { two - erfc_abs } else { erfc_abs };
    half * erfc_val
}

/// Rubinstein-Reiner building block terms.
/// A, B, C, D terms for computing barrier option prices.
struct RRTerms<T: Float> {
    a: T,
    b: T,
    c: T,
    d: T,
}

impl<T: Float> RRTerms<T> {
    /// Compute Rubinstein-Reiner terms for barrier options.
    /// phi = +1 for calls, -1 for puts
    /// eta = +1 for down barriers, -1 for up barriers
    #[allow(clippy::too_many_arguments)]
    fn compute(s: T, k: T, h: T, r: T, q: T, vol: T, t: T, phi: T, eta: T) -> Self {
        let zero = T::zero();
        let two = T::from(2.0).unwrap();

        if t <= zero || vol <= zero {
            return Self {
                a: zero,
                b: zero,
                c: zero,
                d: zero,
            };
        }

        let sqrt_t = t.sqrt();
        let vol_sqrt_t = vol * sqrt_t;
        let vol_sq = vol * vol;

        // Lambda = (r - q + σ²/2) / σ²
        let lambda = (r - q + vol_sq / two) / vol_sq;

        // x1 = ln(S/K)/(σ√T) + λσ√T
        let x1 = (s / k).ln() / vol_sqrt_t + lambda * vol_sqrt_t;
        // x2 = ln(S/H)/(σ√T) + λσ√T
        let x2 = (s / h).ln() / vol_sqrt_t + lambda * vol_sqrt_t;
        // y1 = ln(H²/(SK))/(σ√T) + λσ√T
        let y1 = (h * h / (s * k)).ln() / vol_sqrt_t + lambda * vol_sqrt_t;
        // y2 = ln(H/S)/(σ√T) + λσ√T
        let y2 = (h / s).ln() / vol_sqrt_t + lambda * vol_sqrt_t;

        let discount = (-r * t).exp();
        let fwd_factor = ((-q) * t).exp();

        // (H/S)^(2λ)
        let h_s_2l = (h / s).powf(two * lambda);
        // (H/S)^(2λ-2)
        let h_s_2l_m2 = (h / s).powf(two * lambda - two);

        // Term A: phi controls call vs put sign
        let a = phi * s * fwd_factor * norm_cdf(phi * x1)
            - phi * k * discount * norm_cdf(phi * x1 - phi * vol_sqrt_t);

        // Term B
        let b = phi * s * fwd_factor * norm_cdf(phi * x2)
            - phi * k * discount * norm_cdf(phi * x2 - phi * vol_sqrt_t);

        // Term C: eta controls up vs down
        let c = phi * s * fwd_factor * h_s_2l * norm_cdf(eta * y1)
            - phi * k * discount * h_s_2l_m2 * norm_cdf(eta * y1 - eta * vol_sqrt_t);

        // Term D
        let d = phi * s * fwd_factor * h_s_2l * norm_cdf(eta * y2)
            - phi * k * discount * h_s_2l_m2 * norm_cdf(eta * y2 - eta * vol_sqrt_t);

        Self { a, b, c, d }
    }
}

/// Price a barrier option.
pub fn barrier_price<T: Float>(params: &BarrierParams<T>) -> T {
    barrier_price_with_details(params).price
}

/// Price a barrier option with detailed output.
pub fn barrier_price_with_details<T: Float>(params: &BarrierParams<T>) -> BarrierResult<T> {
    let zero = T::zero();
    let one = T::one();
    let two = T::from(2.0).unwrap();

    let s = params.spot;
    let k = params.strike;
    let h = params.barrier;
    let r = params.rate;
    let q = params.dividend;
    let vol = params.volatility;
    let t = params.maturity;

    if t <= zero || vol <= zero || s <= zero || k <= zero || h <= zero {
        return BarrierResult {
            price: zero,
            lambda: zero,
            y: zero,
            x1: zero,
            y1: zero,
        };
    }

    let sqrt_t = t.sqrt();
    let vol_sqrt_t = vol * sqrt_t;
    let vol_sq = vol * vol;

    let lambda = (r - q + vol_sq / two) / vol_sq;
    let y = (h * h / (s * k)).ln() / vol_sqrt_t + lambda * vol_sqrt_t;
    let x1 = (s / k).ln() / vol_sqrt_t + lambda * vol_sqrt_t;
    let y1 = (h / s).ln() / vol_sqrt_t + lambda * vol_sqrt_t;

    // phi: +1 for call, -1 for put
    let phi = match params.barrier_type.option_type {
        OptionType::Call => one,
        OptionType::Put => -one,
    };

    // eta: +1 for down, -1 for up
    let eta = match params.barrier_type.direction {
        BarrierDirection::Down => one,
        BarrierDirection::Up => -one,
    };

    let price = match params.barrier_type.direction {
        BarrierDirection::Down => {
            compute_down_price(s, k, h, r, q, vol, t, phi, eta, params.barrier_type)
        }
        BarrierDirection::Up => {
            compute_up_price(s, k, h, r, q, vol, t, phi, eta, params.barrier_type)
        }
    };

    BarrierResult {
        price: price.max(zero),
        lambda,
        y,
        x1,
        y1,
    }
}

/// Compute down barrier option price.
#[allow(clippy::too_many_arguments)]
fn compute_down_price<T: Float>(
    s: T,
    k: T,
    h: T,
    r: T,
    q: T,
    vol: T,
    t: T,
    phi: T,
    eta: T,
    bt: BarrierType,
) -> T {
    let zero = T::zero();
    let one = T::one();

    // If already knocked in/out
    if s <= h {
        return match bt.knock {
            KnockType::In => {
                // Already knocked in, return vanilla
                let terms = RRTerms::compute(s, k, h, r, q, vol, t, phi, eta);
                terms.a
            }
            KnockType::Out => zero,
        };
    }

    let terms = RRTerms::compute(s, k, h, r, q, vol, t, phi, one);

    // Rubinstein-Reiner formulas depend on K vs H relationship
    match bt.option_type {
        OptionType::Call => {
            if k > h {
                // Down barrier call, K > H
                match bt.knock {
                    KnockType::In => terms.c,
                    KnockType::Out => terms.a - terms.c,
                }
            } else {
                // Down barrier call, K <= H
                match bt.knock {
                    KnockType::In => terms.a - terms.b + terms.d,
                    KnockType::Out => terms.b - terms.d,
                }
            }
        }
        OptionType::Put => {
            // For puts, we need phi = -1
            let terms_put = RRTerms::compute(s, k, h, r, q, vol, t, -one, one);
            if k > h {
                // Down barrier put, K > H
                match bt.knock {
                    KnockType::In => terms_put.b - terms_put.c + terms_put.d,
                    KnockType::Out => terms_put.a - terms_put.b + terms_put.c - terms_put.d,
                }
            } else {
                // Down barrier put, K <= H
                match bt.knock {
                    KnockType::In => terms_put.a,
                    KnockType::Out => zero,
                }
            }
        }
    }
}

/// Compute up barrier option price.
#[allow(clippy::too_many_arguments)]
fn compute_up_price<T: Float>(
    s: T,
    k: T,
    h: T,
    r: T,
    q: T,
    vol: T,
    t: T,
    phi: T,
    eta: T,
    bt: BarrierType,
) -> T {
    let zero = T::zero();
    let one = T::one();

    // If already knocked in/out
    if s >= h {
        return match bt.knock {
            KnockType::In => {
                // Already knocked in, return vanilla
                let terms = RRTerms::compute(s, k, h, r, q, vol, t, phi, eta);
                terms.a
            }
            KnockType::Out => zero,
        };
    }

    match bt.option_type {
        OptionType::Call => {
            // For up barrier calls, use phi = +1, eta = -1
            let terms = RRTerms::compute(s, k, h, r, q, vol, t, one, -one);
            if k > h {
                // Up barrier call, K > H
                match bt.knock {
                    KnockType::In => terms.a,
                    KnockType::Out => zero,
                }
            } else {
                // Up barrier call, K <= H
                match bt.knock {
                    KnockType::In => terms.b - terms.c + terms.d,
                    KnockType::Out => terms.a - terms.b + terms.c - terms.d,
                }
            }
        }
        OptionType::Put => {
            // For up barrier puts, use phi = -1, eta = -1
            let terms = RRTerms::compute(s, k, h, r, q, vol, t, -one, -one);
            if k > h {
                // Up barrier put, K > H
                match bt.knock {
                    KnockType::In => terms.a - terms.b + terms.d,
                    KnockType::Out => terms.b - terms.d,
                }
            } else {
                // Up barrier put, K <= H
                match bt.knock {
                    KnockType::In => terms.c,
                    KnockType::Out => terms.a - terms.c,
                }
            }
        }
    }
}

// Convenience functions
/// Price a down-and-out call option.
#[inline]
pub fn down_out_call<T: Float>(
    spot: T,
    strike: T,
    barrier: T,
    rate: T,
    dividend: T,
    volatility: T,
    maturity: T,
) -> T {
    barrier_price(&BarrierParams::new(
        spot,
        strike,
        barrier,
        rate,
        dividend,
        volatility,
        maturity,
        BarrierType::down_out_call(),
    ))
}

/// Price a down-and-in call option.
#[inline]
pub fn down_in_call<T: Float>(
    spot: T,
    strike: T,
    barrier: T,
    rate: T,
    dividend: T,
    volatility: T,
    maturity: T,
) -> T {
    barrier_price(&BarrierParams::new(
        spot,
        strike,
        barrier,
        rate,
        dividend,
        volatility,
        maturity,
        BarrierType::down_in_call(),
    ))
}

/// Price an up-and-out call option.
#[inline]
pub fn up_out_call<T: Float>(
    spot: T,
    strike: T,
    barrier: T,
    rate: T,
    dividend: T,
    volatility: T,
    maturity: T,
) -> T {
    barrier_price(&BarrierParams::new(
        spot,
        strike,
        barrier,
        rate,
        dividend,
        volatility,
        maturity,
        BarrierType::up_out_call(),
    ))
}

/// Price an up-and-in call option.
#[inline]
pub fn up_in_call<T: Float>(
    spot: T,
    strike: T,
    barrier: T,
    rate: T,
    dividend: T,
    volatility: T,
    maturity: T,
) -> T {
    barrier_price(&BarrierParams::new(
        spot,
        strike,
        barrier,
        rate,
        dividend,
        volatility,
        maturity,
        BarrierType::up_in_call(),
    ))
}

/// Price a down-and-out put option.
#[inline]
pub fn down_out_put<T: Float>(
    spot: T,
    strike: T,
    barrier: T,
    rate: T,
    dividend: T,
    volatility: T,
    maturity: T,
) -> T {
    barrier_price(&BarrierParams::new(
        spot,
        strike,
        barrier,
        rate,
        dividend,
        volatility,
        maturity,
        BarrierType::down_out_put(),
    ))
}

/// Price a down-and-in put option.
#[inline]
pub fn down_in_put<T: Float>(
    spot: T,
    strike: T,
    barrier: T,
    rate: T,
    dividend: T,
    volatility: T,
    maturity: T,
) -> T {
    barrier_price(&BarrierParams::new(
        spot,
        strike,
        barrier,
        rate,
        dividend,
        volatility,
        maturity,
        BarrierType::down_in_put(),
    ))
}

/// Price an up-and-out put option.
#[inline]
pub fn up_out_put<T: Float>(
    spot: T,
    strike: T,
    barrier: T,
    rate: T,
    dividend: T,
    volatility: T,
    maturity: T,
) -> T {
    barrier_price(&BarrierParams::new(
        spot,
        strike,
        barrier,
        rate,
        dividend,
        volatility,
        maturity,
        BarrierType::up_out_put(),
    ))
}

/// Price an up-and-in put option.
#[inline]
pub fn up_in_put<T: Float>(
    spot: T,
    strike: T,
    barrier: T,
    rate: T,
    dividend: T,
    volatility: T,
    maturity: T,
) -> T {
    barrier_price(&BarrierParams::new(
        spot,
        strike,
        barrier,
        rate,
        dividend,
        volatility,
        maturity,
        BarrierType::up_in_put(),
    ))
}

/// Vanilla European call (Black-Scholes) for testing.
#[cfg(test)]
fn vanilla_call<T: Float>(s: T, k: T, r: T, q: T, vol: T, t: T) -> T {
    let zero = T::zero();
    let one = T::one();
    let terms = RRTerms::compute(s, k, s, r, q, vol, t, one, one);
    if t <= zero || vol <= zero {
        return zero;
    }
    terms.a
}

/// Vanilla European put (Black-Scholes) for testing.
#[cfg(test)]
fn vanilla_put<T: Float>(s: T, k: T, r: T, q: T, vol: T, t: T) -> T {
    let zero = T::zero();
    let one = T::one();
    let terms = RRTerms::compute(s, k, s, r, q, vol, t, -one, one);
    if t <= zero || vol <= zero {
        return zero;
    }
    terms.a
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_barrier_type_constructors() {
        let dic = BarrierType::down_in_call();
        assert_eq!(dic.direction, BarrierDirection::Down);
        assert_eq!(dic.knock, KnockType::In);
        assert_eq!(dic.option_type, OptionType::Call);
    }

    #[test]
    fn test_down_in_out_parity_call() {
        let (s, k, h, r, q, vol, t) = (100.0, 100.0, 90.0, 0.05, 0.0, 0.2, 1.0);
        let di = down_in_call(s, k, h, r, q, vol, t);
        let do_ = down_out_call(s, k, h, r, q, vol, t);
        let v = vanilla_call(s, k, r, q, vol, t);
        assert_relative_eq!(di + do_, v, epsilon = 1e-6);
    }

    #[test]
    fn test_down_in_out_parity_put() {
        let (s, k, h, r, q, vol, t) = (100.0, 100.0, 90.0, 0.05, 0.0, 0.2, 1.0);
        let di = down_in_put(s, k, h, r, q, vol, t);
        let do_ = down_out_put(s, k, h, r, q, vol, t);
        let v = vanilla_put(s, k, r, q, vol, t);
        assert_relative_eq!(di + do_, v, epsilon = 1e-6);
    }

    #[test]
    fn test_up_in_out_parity_call() {
        let (s, k, h, r, q, vol, t) = (100.0, 100.0, 110.0, 0.05, 0.0, 0.2, 1.0);
        let ui = up_in_call(s, k, h, r, q, vol, t);
        let uo = up_out_call(s, k, h, r, q, vol, t);
        let v = vanilla_call(s, k, r, q, vol, t);
        assert_relative_eq!(ui + uo, v, epsilon = 1e-6);
    }

    #[test]
    fn test_up_in_out_parity_put() {
        let (s, k, h, r, q, vol, t) = (100.0, 100.0, 110.0, 0.05, 0.0, 0.2, 1.0);
        let ui = up_in_put(s, k, h, r, q, vol, t);
        let uo = up_out_put(s, k, h, r, q, vol, t);
        let v = vanilla_put(s, k, r, q, vol, t);
        assert_relative_eq!(ui + uo, v, epsilon = 1e-6);
    }

    #[test]
    fn test_already_knocked_in_down() {
        let (s, k, h, r, q, vol, t) = (90.0, 100.0, 95.0, 0.05, 0.0, 0.2, 1.0);
        let dic = down_in_call(s, k, h, r, q, vol, t);
        let v = vanilla_call(s, k, r, q, vol, t);
        assert_relative_eq!(dic, v, epsilon = 1e-10);
    }

    #[test]
    fn test_already_knocked_out_down() {
        let doc = down_out_call(90.0, 100.0, 95.0, 0.05, 0.0, 0.2, 1.0);
        assert_eq!(doc, 0.0);
    }

    #[test]
    fn test_already_knocked_in_up() {
        let (s, k, h, r, q, vol, t) = (110.0, 100.0, 105.0, 0.05, 0.0, 0.2, 1.0);
        let uic = up_in_call(s, k, h, r, q, vol, t);
        let v = vanilla_call(s, k, r, q, vol, t);
        assert_relative_eq!(uic, v, epsilon = 1e-10);
    }

    #[test]
    fn test_already_knocked_out_up() {
        let uoc = up_out_call(110.0, 100.0, 105.0, 0.05, 0.0, 0.2, 1.0);
        assert_eq!(uoc, 0.0);
    }

    #[test]
    fn test_down_out_call_positive() {
        let p = down_out_call(100.0, 100.0, 90.0, 0.05, 0.0, 0.2, 1.0);
        assert!(p > 0.0);
        assert!(p < 15.0);
    }

    #[test]
    fn test_closer_barrier_reduces_out_value() {
        let far = down_out_call(100.0, 100.0, 80.0, 0.05, 0.0, 0.2, 1.0);
        let close = down_out_call(100.0, 100.0, 95.0, 0.05, 0.0, 0.2, 1.0);
        assert!(far > close);
    }

    #[test]
    fn test_closer_barrier_increases_in_value() {
        let far = down_in_call(100.0, 100.0, 80.0, 0.05, 0.0, 0.2, 1.0);
        let close = down_in_call(100.0, 100.0, 95.0, 0.05, 0.0, 0.2, 1.0);
        assert!(close > far);
    }

    #[test]
    fn test_zero_maturity() {
        assert_eq!(down_out_call(100.0, 100.0, 90.0, 0.05, 0.0, 0.2, 0.0), 0.0);
    }

    #[test]
    fn test_zero_volatility() {
        assert_eq!(down_out_call(100.0, 100.0, 90.0, 0.05, 0.0, 0.0, 1.0), 0.0);
    }

    #[test]
    fn test_f32_compatibility() {
        let p = down_out_call(100.0_f32, 100.0, 90.0, 0.05, 0.0, 0.2, 1.0);
        assert!(p > 0.0);
    }
}
