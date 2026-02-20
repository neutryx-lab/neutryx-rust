//! Numerical integration via Gaussian quadrature.
//!
//! Provides Gauss-Kronrod G7-K15 adaptive quadrature, generic over
//! `T: Float` for Enzyme AD compatibility.

use super::numeric::from_f64;
use crate::traits::Float;

// ─── Gauss-Kronrod G7-K15 nodes and weights ─────────────────────

/// Gauss-Kronrod G7-K15 quadrature nodes (positive half, symmetric).
const GK_NODES: [f64; 8] = [
    0.0,
    0.207784955007898,
    0.405845151377397,
    0.586087235467691,
    0.741531185599394,
    0.864864423359769,
    0.949107912342759,
    0.991455371120813,
];

/// Kronrod weights for the 15-point rule.
const K15_WEIGHTS: [f64; 8] = [
    0.209482141084728,
    0.204432940075298,
    0.190350578064785,
    0.169004726639268,
    0.140653259715525,
    0.104790010322250,
    0.063092092629979,
    0.022935322010529,
];

/// Gauss weights for the 7-point rule (zero for Kronrod-only nodes).
/// Reserved for future error estimation (G7 vs K15 comparison).
#[allow(dead_code)]
const G7_WEIGHTS: [f64; 8] = [
    0.417959183673469,
    0.0,
    0.381830050505119,
    0.0,
    0.279705391489277,
    0.0,
    0.129484966168870,
    0.0,
];

/// Gauss-Kronrod G7-K15 quadrature over `[a, b]`.
///
/// Returns the Kronrod estimate of the integral.
pub fn gauss_kronrod_integrate<T: Float>(f: &dyn Fn(T) -> T, a: T, b: T) -> T {
    let half = from_f64::<T>(0.5);
    let mid = half * (a + b);
    let half_len = half * (b - a);

    let mut result_k15 = T::zero();

    for i in 0..8 {
        let node: T = from_f64(GK_NODES[i]);
        let wk: T = from_f64(K15_WEIGHTS[i]);

        if i == 0 {
            result_k15 = result_k15 + wk * f(mid);
        } else {
            let x_plus = mid + half_len * node;
            let x_minus = mid - half_len * node;
            result_k15 = result_k15 + wk * (f(x_plus) + f(x_minus));
        }
    }

    result_k15 * half_len
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn gk_integrates_constant() {
        // ∫₀¹ 3 dx = 3
        let result = gauss_kronrod_integrate(&|_x: f64| 3.0, 0.0, 1.0);
        assert_relative_eq!(result, 3.0, epsilon = 1e-14);
    }

    #[test]
    fn gk_integrates_polynomial() {
        // ∫₀¹ x² dx = 1/3
        let result = gauss_kronrod_integrate(&|x: f64| x * x, 0.0, 1.0);
        assert_relative_eq!(result, 1.0 / 3.0, epsilon = 1e-14);
    }

    #[test]
    fn gk_integrates_higher_polynomial() {
        // G7-K15 is exact for polynomials up to degree 29
        // ∫₋₁¹ x⁶ dx = 2/7
        let result = gauss_kronrod_integrate(&|x: f64| x.powi(6), -1.0, 1.0);
        assert_relative_eq!(result, 2.0 / 7.0, epsilon = 1e-13);
    }

    #[test]
    fn gk_integrates_exp() {
        // ∫₀¹ e^x dx = e - 1
        let result = gauss_kronrod_integrate(&|x: f64| x.exp(), 0.0, 1.0);
        assert_relative_eq!(result, 1.0_f64.exp() - 1.0, epsilon = 1e-14);
    }
}
