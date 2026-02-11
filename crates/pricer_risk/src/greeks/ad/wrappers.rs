//! Enzyme autodiff wrapper functions.

/// Prices a European call option using Black-Scholes.
#[inline]
pub fn price_european_call(spot: f64, strike: f64, rate: f64, vol: f64, time: f64) -> f64 {
    if time <= 0.0 {
        return (spot - strike).max(0.0);
    }

    let sqrt_t = time.sqrt();
    let d1 = ((spot / strike).ln() + (rate + 0.5 * vol * vol) * time) / (vol * sqrt_t);
    let d2 = d1 - vol * sqrt_t;

    let n_d1 = normal_cdf(d1);
    let n_d2 = normal_cdf(d2);

    spot * n_d1 - strike * (-rate * time).exp() * n_d2
}

#[inline]
fn normal_cdf(x: f64) -> f64 {
    const SQRT_2: f64 = std::f64::consts::SQRT_2;

    if x.abs() > 8.0 {
        return if x > 0.0 { 1.0 } else { 0.0 };
    }

    const A1: f64 = 0.254829592;
    const A2: f64 = -0.284496736;
    const A3: f64 = 1.421413741;
    const A4: f64 = -1.453152027;
    const A5: f64 = 1.061405429;
    const P: f64 = 0.3275911;

    let arg = -x / SQRT_2;
    let abs_arg = arg.abs();
    let t = 1.0 / (1.0 + P * abs_arg);
    let poly = A1 + t * (A2 + t * (A3 + t * (A4 + t * A5)));
    let erfc_abs = t * poly * (-abs_arg * abs_arg).exp();

    let erfc_val = if arg < 0.0 { 2.0 - erfc_abs } else { erfc_abs };

    0.5 * erfc_val
}

#[inline]
fn normal_pdf(x: f64) -> f64 {
    const INV_SQRT_2PI: f64 = 0.3989422804014327;
    INV_SQRT_2PI * (-0.5 * x * x).exp()
}

/// Computes European call price and Delta using forward mode AD.
#[cfg(feature = "enzyme-ad")]
pub fn price_european_delta(spot: f64, strike: f64, rate: f64, vol: f64, time: f64) -> (f64, f64) {
    let mut d_price = 0.0;
    let price = d_price_spot(spot, 1.0, strike, rate, vol, time, &mut d_price);
    (price, d_price)
}

/// Computes European call price and Delta using finite difference fallback.
#[cfg(not(feature = "enzyme-ad"))]
pub fn price_european_delta(spot: f64, strike: f64, rate: f64, vol: f64, time: f64) -> (f64, f64) {
    let price = price_european_call(spot, strike, rate, vol, time);
    let delta = compute_delta_fd(spot, strike, rate, vol, time);
    (price, delta)
}

/// Computes European call price and Vega using finite difference fallback.
#[cfg(not(feature = "enzyme-ad"))]
pub fn price_european_vega(spot: f64, strike: f64, rate: f64, vol: f64, time: f64) -> (f64, f64) {
    let price = price_european_call(spot, strike, rate, vol, time);
    let vega = compute_vega_fd(spot, strike, rate, vol, time);
    (price, vega)
}

/// Computes European call price and Vega using Enzyme forward mode AD.
#[cfg(feature = "enzyme-ad")]
pub fn price_european_vega(spot: f64, strike: f64, rate: f64, vol: f64, time: f64) -> (f64, f64) {
    let mut d_price = 0.0;
    let price = d_price_vol(spot, strike, rate, vol, 1.0, time, &mut d_price);
    (price, d_price)
}

/// All Greeks computed via reverse mode AD.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AllGreeks {
    /// Option price.
    pub price: f64,
    /// Delta.
    pub delta: f64,
    /// Rho.
    pub rho: f64,
    /// Vega.
    pub vega: f64,
    /// Theta.
    pub theta: f64,
}

/// Computes all Greeks using reverse mode AD.
#[cfg(not(feature = "enzyme-ad"))]
pub fn price_european_greeks(spot: f64, strike: f64, rate: f64, vol: f64, time: f64) -> AllGreeks {
    let price = price_european_call(spot, strike, rate, vol, time);
    AllGreeks {
        price,
        delta: compute_delta_fd(spot, strike, rate, vol, time),
        rho: compute_rho_fd(spot, strike, rate, vol, time),
        vega: compute_vega_fd(spot, strike, rate, vol, time),
        theta: compute_theta_fd(spot, strike, rate, vol, time),
    }
}

/// Computes all Greeks using Enzyme reverse mode AD.
#[cfg(feature = "enzyme-ad")]
pub fn price_european_greeks(spot: f64, strike: f64, rate: f64, vol: f64, time: f64) -> AllGreeks {
    let mut d_spot = 0.0;
    let mut d_rate = 0.0;
    let mut d_vol = 0.0;
    let mut d_time = 0.0;

    let price = d_price_all(
        spot,
        &mut d_spot,
        strike,
        rate,
        &mut d_rate,
        vol,
        &mut d_vol,
        time,
        &mut d_time,
    );

    AllGreeks {
        price,
        delta: d_spot,
        rho: d_rate,
        vega: d_vol,
        theta: -d_time,
    }
}

#[cfg(feature = "enzyme-ad")]
mod enzyme_impl {
    use std::autodiff::autodiff;

    #[autodiff(d_price_spot, Forward, Dual, Const, Const, Const, Const, Dual)]
    pub fn price_european_call_primal(
        spot: f64,
        strike: f64,
        rate: f64,
        vol: f64,
        time: f64,
    ) -> f64 {
        super::price_european_call(spot, strike, rate, vol, time)
    }

    #[autodiff(d_price_vol, Forward, Const, Const, Const, Dual, Const, Dual)]
    pub fn price_european_call_vol_primal(
        spot: f64,
        strike: f64,
        rate: f64,
        vol: f64,
        time: f64,
    ) -> f64 {
        super::price_european_call(spot, strike, rate, vol, time)
    }

    #[autodiff(
        d_price_all,
        Reverse,
        Duplicated,
        Const,
        Duplicated,
        Duplicated,
        Duplicated,
        Active
    )]
    pub fn price_european_call_adjoint(
        spot: f64,
        strike: f64,
        rate: f64,
        vol: f64,
        time: f64,
    ) -> f64 {
        super::price_european_call(spot, strike, rate, vol, time)
    }
}

#[cfg(feature = "enzyme-ad")]
pub use enzyme_impl::{d_price_all, d_price_spot, d_price_vol};

const FD_BUMP_RELATIVE: f64 = 0.01;
const FD_BUMP_ABSOLUTE: f64 = 0.0001;

fn compute_delta_fd(spot: f64, strike: f64, rate: f64, vol: f64, time: f64) -> f64 {
    let bump = (spot * FD_BUMP_RELATIVE).max(FD_BUMP_ABSOLUTE);
    let price_up = price_european_call(spot + bump, strike, rate, vol, time);
    let price_down = price_european_call(spot - bump, strike, rate, vol, time);
    (price_up - price_down) / (2.0 * bump)
}

fn compute_vega_fd(spot: f64, strike: f64, rate: f64, vol: f64, time: f64) -> f64 {
    let bump = FD_BUMP_ABSOLUTE;
    let price_up = price_european_call(spot, strike, rate, vol + bump, time);
    let price_down = price_european_call(spot, strike, rate, (vol - bump).max(0.001), time);
    (price_up - price_down) / (2.0 * bump)
}

fn compute_rho_fd(spot: f64, strike: f64, rate: f64, vol: f64, time: f64) -> f64 {
    let bump = FD_BUMP_ABSOLUTE;
    let price_up = price_european_call(spot, strike, rate + bump, vol, time);
    let price_down = price_european_call(spot, strike, rate - bump, vol, time);
    (price_up - price_down) / (2.0 * bump)
}

fn compute_theta_fd(spot: f64, strike: f64, rate: f64, vol: f64, time: f64) -> f64 {
    let bump = 1.0 / 252.0;
    if time <= bump {
        return 0.0;
    }
    let price_now = price_european_call(spot, strike, rate, vol, time);
    let price_later = price_european_call(spot, strike, rate, vol, time - bump);
    -(price_now - price_later) / bump
}

/// Compute Gamma using central finite difference on Delta.
pub fn compute_gamma_fd(spot: f64, strike: f64, rate: f64, vol: f64, time: f64) -> f64 {
    let bump = (spot * FD_BUMP_RELATIVE).max(FD_BUMP_ABSOLUTE);
    let delta_up = compute_delta_fd(spot + bump, strike, rate, vol, time);
    let delta_down = compute_delta_fd(spot - bump, strike, rate, vol, time);
    (delta_up - delta_down) / (2.0 * bump)
}

/// Analytical Delta for European call option: Delta = N(d1).
pub fn delta_analytical(spot: f64, strike: f64, rate: f64, vol: f64, time: f64) -> f64 {
    if time <= 0.0 {
        return if spot > strike { 1.0 } else { 0.0 };
    }
    let sqrt_t = time.sqrt();
    let d1 = ((spot / strike).ln() + (rate + 0.5 * vol * vol) * time) / (vol * sqrt_t);
    normal_cdf(d1)
}

/// Analytical Gamma for European call option: Gamma = N'(d1) / (S * sigma *
/// sqrt(T)).
pub fn gamma_analytical(spot: f64, strike: f64, rate: f64, vol: f64, time: f64) -> f64 {
    if time <= 0.0 {
        return 0.0;
    }
    let sqrt_t = time.sqrt();
    let d1 = ((spot / strike).ln() + (rate + 0.5 * vol * vol) * time) / (vol * sqrt_t);
    normal_pdf(d1) / (spot * vol * sqrt_t)
}

/// Analytical Vega for European call option: Vega = S * N'(d1) * sqrt(T).
pub fn vega_analytical(spot: f64, strike: f64, rate: f64, vol: f64, time: f64) -> f64 {
    if time <= 0.0 {
        return 0.0;
    }
    let sqrt_t = time.sqrt();
    let d1 = ((spot / strike).ln() + (rate + 0.5 * vol * vol) * time) / (vol * sqrt_t);
    spot * normal_pdf(d1) * sqrt_t
}

/// Analytical Theta for European call option.
pub fn theta_analytical(spot: f64, strike: f64, rate: f64, vol: f64, time: f64) -> f64 {
    if time <= 0.0 {
        return 0.0;
    }
    let sqrt_t = time.sqrt();
    let d1 = ((spot / strike).ln() + (rate + 0.5 * vol * vol) * time) / (vol * sqrt_t);
    let d2 = d1 - vol * sqrt_t;
    let n_prime_d1 = normal_pdf(d1);
    let n_d2 = normal_cdf(d2);
    let df = (-rate * time).exp();

    -spot * n_prime_d1 * vol / (2.0 * sqrt_t) - rate * strike * df * n_d2
}

/// Analytical Rho for European call option: Rho = K * T * e^(-rT) * N(d2).
pub fn rho_analytical(spot: f64, strike: f64, rate: f64, vol: f64, time: f64) -> f64 {
    if time <= 0.0 {
        return 0.0;
    }
    let sqrt_t = time.sqrt();
    let d1 = ((spot / strike).ln() + (rate + 0.5 * vol * vol) * time) / (vol * sqrt_t);
    let d2 = d1 - vol * sqrt_t;
    let n_d2 = normal_cdf(d2);
    let df = (-rate * time).exp();

    strike * time * df * n_d2
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    const SPOT: f64 = 100.0;
    const STRIKE: f64 = 100.0;
    const RATE: f64 = 0.05;
    const VOL: f64 = 0.2;
    const TIME: f64 = 1.0;

    #[test]
    fn test_price_european_call() {
        let price = price_european_call(SPOT, STRIKE, RATE, VOL, TIME);
        assert!(price > 10.0 && price < 11.0, "Price = {}", price);
    }

    #[test]
    fn test_price_european_call_itm() {
        let price = price_european_call(110.0, 100.0, RATE, VOL, TIME);
        let atm_price = price_european_call(SPOT, STRIKE, RATE, VOL, TIME);
        assert!(price > atm_price);
    }

    #[test]
    fn test_price_european_call_otm() {
        let price = price_european_call(90.0, 100.0, RATE, VOL, TIME);
        let atm_price = price_european_call(SPOT, STRIKE, RATE, VOL, TIME);
        assert!(price < atm_price);
    }

    #[test]
    fn test_price_european_delta() {
        let (price, delta) = price_european_delta(SPOT, STRIKE, RATE, VOL, TIME);
        assert!(price > 0.0);
        assert!(delta > 0.4 && delta < 0.8, "Delta = {}", delta);
    }

    #[test]
    fn test_price_european_vega() {
        let (price, vega) = price_european_vega(SPOT, STRIKE, RATE, VOL, TIME);
        assert!(price > 0.0);
        assert!(vega > 0.0, "Vega = {}", vega);
    }

    #[test]
    fn test_price_european_greeks() {
        let greeks = price_european_greeks(SPOT, STRIKE, RATE, VOL, TIME);

        assert!(greeks.price > 0.0);
        assert!(
            greeks.delta > 0.4 && greeks.delta < 0.8,
            "Delta = {}",
            greeks.delta
        );
        assert!(greeks.vega > 0.0, "Vega = {}", greeks.vega);
        assert!(greeks.rho > 0.0, "Rho = {}", greeks.rho);
        assert!(greeks.theta < 0.0, "Theta = {}", greeks.theta);
    }

    #[test]
    fn test_delta_vs_analytical() {
        let (_, delta_fd) = price_european_delta(SPOT, STRIKE, RATE, VOL, TIME);
        let delta_ana = delta_analytical(SPOT, STRIKE, RATE, VOL, TIME);

        assert_relative_eq!(delta_fd, delta_ana, max_relative = 0.01);
    }

    #[test]
    fn test_gamma_fd_vs_analytical() {
        let gamma_fd = compute_gamma_fd(SPOT, STRIKE, RATE, VOL, TIME);
        let gamma_ana = gamma_analytical(SPOT, STRIKE, RATE, VOL, TIME);

        assert_relative_eq!(gamma_fd, gamma_ana, max_relative = 0.05);
    }

    #[test]
    fn test_vega_vs_analytical() {
        let (_, vega_fd) = price_european_vega(SPOT, STRIKE, RATE, VOL, TIME);
        let vega_ana = vega_analytical(SPOT, STRIKE, RATE, VOL, TIME);

        assert_relative_eq!(vega_fd, vega_ana, max_relative = 0.01);
    }

    #[test]
    fn test_theta_vs_analytical() {
        let greeks = price_european_greeks(SPOT, STRIKE, RATE, VOL, TIME);
        let theta_ana = theta_analytical(SPOT, STRIKE, RATE, VOL, TIME);

        assert_relative_eq!(greeks.theta, theta_ana, max_relative = 0.05);
    }

    #[test]
    fn test_rho_vs_analytical() {
        let greeks = price_european_greeks(SPOT, STRIKE, RATE, VOL, TIME);
        let rho_ana = rho_analytical(SPOT, STRIKE, RATE, VOL, TIME);

        assert_relative_eq!(greeks.rho, rho_ana, max_relative = 0.01);
    }

    #[test]
    fn test_expired_option() {
        let price = price_european_call(SPOT, STRIKE, RATE, VOL, 0.0);
        assert_eq!(price, 0.0);

        let itm_price = price_european_call(110.0, 100.0, RATE, VOL, 0.0);
        assert_eq!(itm_price, 10.0);
    }

    #[test]
    fn test_all_greeks_struct() {
        let greeks = AllGreeks {
            price: 10.0,
            delta: 0.5,
            rho: 15.0,
            vega: 25.0,
            theta: -0.05,
        };

        assert_eq!(greeks.price, 10.0);
        assert_eq!(greeks.delta, 0.5);
    }
}
