//! Barrier option analytical pricing.
//!
//! Implements closed-form solutions for European barrier options.
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

/// Standard normal CDF.
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

/// Vanilla European call (Black-Scholes).
#[inline]
fn vanilla_call<T: Float>(s: T, k: T, r: T, q: T, vol: T, t: T) -> T {
    let zero = T::zero();
    let two = T::from(2.0).unwrap();
    if t <= zero || vol <= zero {
        return zero;
    }

    let sqrt_t = t.sqrt();
    let d1 = ((s / k).ln() + (r - q + vol * vol / two) * t) / (vol * sqrt_t);
    let d2 = d1 - vol * sqrt_t;

    s * ((-q) * t).exp() * norm_cdf(d1) - k * (-r * t).exp() * norm_cdf(d2)
}

/// Vanilla European put (Black-Scholes).
#[inline]
fn vanilla_put<T: Float>(s: T, k: T, r: T, q: T, vol: T, t: T) -> T {
    let zero = T::zero();
    let two = T::from(2.0).unwrap();
    if t <= zero || vol <= zero {
        return zero;
    }

    let sqrt_t = t.sqrt();
    let d1 = ((s / k).ln() + (r - q + vol * vol / two) * t) / (vol * sqrt_t);
    let d2 = d1 - vol * sqrt_t;

    k * (-r * t).exp() * norm_cdf(-d2) - s * ((-q) * t).exp() * norm_cdf(-d1)
}

/// Price a barrier option.
pub fn barrier_price<T: Float>(params: &BarrierParams<T>) -> T {
    barrier_price_with_details(params).price
}

/// Price a barrier option with detailed output.
pub fn barrier_price_with_details<T: Float>(params: &BarrierParams<T>) -> BarrierResult<T> {
    let zero = T::zero();
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

    let vol_sqrt_t = vol * t.sqrt();
    let vol_sq = vol * vol;

    let lambda = (r - q + vol_sq / two) / vol_sq;
    let y = (h * h / (s * k)).ln() / vol_sqrt_t + lambda * vol_sqrt_t;
    let x1 = (s / h).ln() / vol_sqrt_t + lambda * vol_sqrt_t;
    let y1 = (h / s).ln() / vol_sqrt_t + lambda * vol_sqrt_t;

    let price = match params.barrier_type.direction {
        BarrierDirection::Down => compute_down(
            s,
            k,
            h,
            r,
            q,
            vol,
            t,
            lambda,
            y,
            x1,
            y1,
            params.barrier_type.knock,
            params.barrier_type.option_type,
        ),
        BarrierDirection::Up => compute_up(
            s,
            k,
            h,
            r,
            q,
            vol,
            t,
            lambda,
            y,
            x1,
            y1,
            params.barrier_type.knock,
            params.barrier_type.option_type,
        ),
    };

    BarrierResult {
        price,
        lambda,
        y,
        x1,
        y1,
    }
}

#[allow(clippy::too_many_arguments)]
fn compute_down<T: Float>(
    s: T,
    k: T,
    h: T,
    r: T,
    q: T,
    vol: T,
    t: T,
    lambda: T,
    y: T,
    x1: T,
    y1: T,
    knock: KnockType,
    opt: OptionType,
) -> T {
    let zero = T::zero();
    if s <= h {
        return match knock {
            KnockType::In => match opt {
                OptionType::Call => vanilla_call(s, k, r, q, vol, t),
                OptionType::Put => vanilla_put(s, k, r, q, vol, t),
            },
            KnockType::Out => zero,
        };
    }

    let knock_in = down_knock_in(s, k, h, r, q, vol, t, lambda, y, x1, y1, opt);
    match knock {
        KnockType::In => knock_in,
        KnockType::Out => {
            let vanilla = match opt {
                OptionType::Call => vanilla_call(s, k, r, q, vol, t),
                OptionType::Put => vanilla_put(s, k, r, q, vol, t),
            };
            (vanilla - knock_in).max(zero)
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn compute_up<T: Float>(
    s: T,
    k: T,
    h: T,
    r: T,
    q: T,
    vol: T,
    t: T,
    lambda: T,
    y: T,
    x1: T,
    y1: T,
    knock: KnockType,
    opt: OptionType,
) -> T {
    let zero = T::zero();
    if s >= h {
        return match knock {
            KnockType::In => match opt {
                OptionType::Call => vanilla_call(s, k, r, q, vol, t),
                OptionType::Put => vanilla_put(s, k, r, q, vol, t),
            },
            KnockType::Out => zero,
        };
    }

    let knock_in = up_knock_in(s, k, h, r, q, vol, t, lambda, y, x1, y1, opt);
    match knock {
        KnockType::In => knock_in,
        KnockType::Out => {
            let vanilla = match opt {
                OptionType::Call => vanilla_call(s, k, r, q, vol, t),
                OptionType::Put => vanilla_put(s, k, r, q, vol, t),
            };
            (vanilla - knock_in).max(zero)
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn down_knock_in<T: Float>(
    s: T,
    k: T,
    h: T,
    r: T,
    q: T,
    vol: T,
    t: T,
    lambda: T,
    y: T,
    _x1: T,
    _y1: T,
    opt: OptionType,
) -> T {
    let zero = T::zero();
    let two = T::from(2.0).unwrap();
    let vol_sqrt_t = vol * t.sqrt();
    let discount = (-r * t).exp();
    let fwd_factor = ((-q) * t).exp();
    let ratio = h / s;
    let h_s_2l = ratio.powf(two * lambda);
    let h_s_2l_m2 = ratio.powf(two * lambda - two);

    match opt {
        OptionType::Call => {
            let t1 = h_s_2l * s * fwd_factor * norm_cdf(y);
            let t2 = k * discount * h_s_2l_m2 * norm_cdf(y - vol_sqrt_t);
            (t1 - t2).max(zero)
        }
        OptionType::Put => {
            let t1 = k * discount * h_s_2l_m2 * norm_cdf(-y + vol_sqrt_t);
            let t2 = h_s_2l * s * fwd_factor * norm_cdf(-y);
            (t1 - t2).max(zero)
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn up_knock_in<T: Float>(
    s: T,
    k: T,
    h: T,
    r: T,
    q: T,
    vol: T,
    t: T,
    lambda: T,
    y: T,
    _x1: T,
    _y1: T,
    opt: OptionType,
) -> T {
    let zero = T::zero();
    let two = T::from(2.0).unwrap();
    let vol_sqrt_t = vol * t.sqrt();
    let discount = (-r * t).exp();
    let fwd_factor = ((-q) * t).exp();
    let ratio = h / s;
    let h_s_2l = ratio.powf(two * lambda);
    let h_s_2l_m2 = ratio.powf(two * lambda - two);

    match opt {
        OptionType::Call => {
            let t1 = h_s_2l * s * fwd_factor * norm_cdf(y);
            let t2 = k * discount * h_s_2l_m2 * norm_cdf(y - vol_sqrt_t);
            (t1 - t2).max(zero)
        }
        OptionType::Put => {
            let t1 = k * discount * h_s_2l_m2 * norm_cdf(-y + vol_sqrt_t);
            let t2 = h_s_2l * s * fwd_factor * norm_cdf(-y);
            (t1 - t2).max(zero)
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
