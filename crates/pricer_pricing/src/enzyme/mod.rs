//! Enzyme autodiff bindings for pricer_pricing.
//!
//! This module provides the interface for Enzyme LLVM-level automatic differentiation.
//! Enzyme operates at the LLVM IR level, enabling high-performance gradient computation
//! for financial derivative pricing.
//!
//! # Module Structure
//!
//! - [`wrappers`]: Enzyme `#[autodiff_*]` macro wrappers for option pricing
//! - [`forward`]: Forward mode AD types (`ForwardAD<T>`)
//! - [`reverse`]: Reverse mode AD types (`ReverseAD<T>`, `GammaAD<T>`)
//! - [`smooth`]: Enzyme-compatible smooth approximation functions
//! - [`loops`]: Enzyme-compatible loop patterns and iterators
//! - [`greeks`]: Enzyme-based Greeks calculation for Monte Carlo pricing
//! - [`parallel`]: Parallel adjoint aggregation for Monte Carlo Greeks
//! - [`checkpoint_ad`]: Checkpointing integration for path-dependent AD
//! - [`fallback`]: Fallback to finite differences when Enzyme is disabled
//! - [`verification`]: Verification tests against analytical and FD methods
//!
//! # Enzyme Integration
//!
//! When the `enzyme-ad` feature is enabled, this module uses actual LLVM-level
//! automatic differentiation via Enzyme. When disabled, it falls back to
//! finite difference approximations.
//!
//! ```rust,ignore
//! #![feature(autodiff)]
//!
//! #[autodiff_reverse(df, Active)]
//! pub fn f(x: f64) -> f64 {
//!     x * x
//! }
//! ```
//!
//! # Usage
//!
//! ```rust
//! use pricer_pricing::enzyme::{Activity, ADMode, gradient};
//!
//! // Simple gradient computation
//! let grad = gradient(|x| x * x, 3.0);
//! assert!((grad - 6.0).abs() < 1e-6);
//!
//! // AD mode selection for EnzymeContext
//! let mode = ADMode::Forward;
//! assert!(mode.is_forward());
//! ```

// Submodules
pub mod checkpoint_ad;
pub mod fallback;
pub mod forward;
pub mod greeks;
pub mod loops;
pub mod parallel;
pub mod reverse;
pub mod smooth;
pub mod verification;
pub mod wrappers;

// =============================================================================
// ADMode - AD (自動微分) モード列挙型
// =============================================================================

/// ADモード（自動微分モード）
///
/// Enzymeコンテキストで使用される自動微分モードを表す。
/// Forward mode（前進モード）とReverse mode（逆伝播モード）の
/// 切り替えを一元管理するために使用する。
///
/// # バリアント
///
/// - `Inactive`: 微分無効。通常の関数評価のみ行う。
/// - `Forward`: Forward mode AD。tangent（接線）値を入力から出力へ伝播する。
///   単一パラメータに対するGreeks計算（Delta、Vega等）に効率的。
/// - `Reverse`: Reverse mode AD。adjoint（随伴）値を出力から入力へ逆伝播する。
///   多数のパラメータに対するGreeksを一括計算する場合に効率的。
///
/// # 使用例
///
/// ```rust
/// use pricer_pricing::enzyme::ADMode;
///
/// // デフォルトはInactive（微分無効）
/// let mode = ADMode::default();
/// assert!(!mode.is_active());
///
/// // Forward modeでDelta計算
/// let forward = ADMode::Forward;
/// assert!(forward.is_forward());
/// assert!(forward.is_active());
///
/// // Reverse modeで全Greeks一括計算
/// let reverse = ADMode::Reverse;
/// assert!(reverse.is_reverse());
/// ```
///
/// # 要件
///
/// - Requirements 4.1, 4.3, 4.4, 4.6
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum ADMode {
    /// 微分無効
    ///
    /// 自動微分を行わず、通常の関数評価のみ実行する。
    /// EnzymeContextの初期状態。
    #[default]
    Inactive,

    /// Forward mode（前進モード）AD
    ///
    /// tangent（接線）値を入力から出力へ伝播する。
    /// 各入力変数に対するtangent seedを設定し、
    /// 出力のtangent値として微分を取得する。
    ///
    /// - 計算量: O(n) where n = 入力次元
    /// - 用途: Delta、Vega等の単一パラメータGreeks
    Forward,

    /// Reverse mode（逆伝播モード）AD
    ///
    /// adjoint（随伴）値を出力から入力へ逆伝播する。
    /// 出力にadjoint seed（通常は1.0）を設定し、
    /// 全入力パラメータに対する勾配を一度に取得する。
    ///
    /// - 計算量: O(m) where m = 出力次元
    /// - 用途: 多数のパラメータに対する全Greeks一括計算
    Reverse,
}

impl ADMode {
    /// Forward modeか判定する
    ///
    /// # 戻り値
    ///
    /// `true` if `self == ADMode::Forward`, `false` otherwise
    ///
    /// # 使用例
    ///
    /// ```rust
    /// use pricer_pricing::enzyme::ADMode;
    ///
    /// assert!(ADMode::Forward.is_forward());
    /// assert!(!ADMode::Reverse.is_forward());
    /// assert!(!ADMode::Inactive.is_forward());
    /// ```
    #[inline]
    pub fn is_forward(&self) -> bool {
        matches!(self, ADMode::Forward)
    }

    /// Reverse modeか判定する
    ///
    /// # 戻り値
    ///
    /// `true` if `self == ADMode::Reverse`, `false` otherwise
    ///
    /// # 使用例
    ///
    /// ```rust
    /// use pricer_pricing::enzyme::ADMode;
    ///
    /// assert!(ADMode::Reverse.is_reverse());
    /// assert!(!ADMode::Forward.is_reverse());
    /// assert!(!ADMode::Inactive.is_reverse());
    /// ```
    #[inline]
    pub fn is_reverse(&self) -> bool {
        matches!(self, ADMode::Reverse)
    }

    /// ADが有効か判定する（Inactive以外）
    ///
    /// # 戻り値
    ///
    /// `true` if `self != ADMode::Inactive`, `false` otherwise
    ///
    /// # 使用例
    ///
    /// ```rust
    /// use pricer_pricing::enzyme::ADMode;
    ///
    /// assert!(!ADMode::Inactive.is_active());
    /// assert!(ADMode::Forward.is_active());
    /// assert!(ADMode::Reverse.is_active());
    /// ```
    #[inline]
    pub fn is_active(&self) -> bool {
        !matches!(self, ADMode::Inactive)
    }
}

// =============================================================================
// Activity - Activity annotation列挙型
// =============================================================================

/// Activity annotations for autodiff parameters.
///
/// These annotations specify how each parameter participates in differentiation:
///
/// - `Const`: The parameter is not differentiated (treated as constant)
/// - `Dual`: Forward mode - carries tangent value alongside primal
/// - `Active`: Reverse mode - accumulates gradients during backward pass
/// - `Duplicated`: Reverse mode - parameter has a separate shadow for gradients
///
/// # Phase 4 Usage
///
/// ```rust,ignore
/// #[autodiff_forward(df, Dual, Const, Dual)]
/// pub fn f(x: &[f64], y: f64) -> f64 { ... }
///
/// #[autodiff_reverse(df, Duplicated, Const, Active)]
/// pub fn g(x: &mut [f64], y: f64) -> f64 { ... }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Activity {
    /// Parameter is constant (not differentiated).
    ///
    /// Use for parameters that do not affect the derivative,
    /// such as configuration values or fixed constants.
    Const,

    /// Parameter carries dual/tangent value (forward mode).
    ///
    /// In forward mode AD, each variable carries both its primal value
    /// and its tangent (derivative with respect to input).
    Dual,

    /// Parameter is active (reverse mode accumulation).
    ///
    /// In reverse mode AD, active parameters accumulate gradients
    /// during the backward pass. Use for scalar outputs.
    Active,

    /// Parameter is duplicated with shadow (reverse mode).
    ///
    /// The parameter has a separate shadow buffer where gradients
    /// are accumulated. Use for arrays or mutable buffers.
    Duplicated,

    /// Shadow only, no primal value (reverse mode).
    ///
    /// Only the shadow gradient buffer is used; primal value is ignored.
    /// Useful for output-only gradient buffers.
    DuplicatedOnly,
}

impl Activity {
    /// Returns true if this activity participates in differentiation.
    #[inline]
    pub fn is_active(&self) -> bool {
        !matches!(self, Activity::Const)
    }

    /// Returns true if this is a reverse mode activity.
    #[inline]
    pub fn is_reverse_mode(&self) -> bool {
        matches!(
            self,
            Activity::Active | Activity::Duplicated | Activity::DuplicatedOnly
        )
    }

    /// Returns true if this is a forward mode activity.
    #[inline]
    pub fn is_forward_mode(&self) -> bool {
        matches!(self, Activity::Dual)
    }
}

/// Compute gradient of function `f` at point `x` using Enzyme.
///
/// # Phase 3.0 Implementation
///
/// This is a **placeholder implementation** using central finite difference:
/// ```text
/// f'(x) ≈ (f(x + h) - f(x - h)) / (2h)
/// ```
///
/// The finite difference step size `h` is set to `1e-8` which provides
/// a good balance between truncation and rounding errors for most functions.
///
/// # Phase 4 Implementation
///
/// Phase 4 will replace this with actual Enzyme autodiff:
/// ```rust,ignore
/// #[autodiff_reverse(gradient_impl, Active)]
/// pub fn gradient<F>(f: F, x: f64) -> f64
/// where
///     F: Fn(f64) -> f64;
/// ```
///
/// # Arguments
///
/// * `f` - The function to differentiate. Must be a closure or function
///   that takes `f64` and returns `f64`.
/// * `x` - The point at which to evaluate the gradient.
///
/// # Returns
///
/// The gradient (derivative) of `f` at point `x`.
///
/// # Examples
///
/// ```rust
/// use pricer_pricing::enzyme::gradient;
///
/// // Gradient of x^2 is 2x
/// let grad = gradient(|x| x * x, 3.0);
/// assert!((grad - 6.0).abs() < 1e-6);
///
/// // Gradient of sin(x) is cos(x)
/// let grad = gradient(|x| x.sin(), 0.0);
/// assert!((grad - 1.0).abs() < 1e-6);
/// ```
#[inline]
pub fn gradient<F>(f: F, x: f64) -> f64
where
    F: Fn(f64) -> f64,
{
    // Phase 3.0: Finite difference approximation
    // Phase 4: Replace with Enzyme autodiff
    const H: f64 = 1e-8;
    (f(x + H) - f(x - H)) / (2.0 * H)
}

/// Compute gradient of function `f` at point `x` with custom step size.
///
/// Similar to [`gradient`], but allows specifying the finite difference step size.
/// This is useful for functions with very large or small values where the
/// default step size may not be appropriate.
///
/// # Arguments
///
/// * `f` - The function to differentiate.
/// * `x` - The point at which to evaluate the gradient.
/// * `h` - The finite difference step size.
///
/// # Returns
///
/// The gradient (derivative) of `f` at point `x`.
#[inline]
pub fn gradient_with_step<F>(f: F, x: f64, h: f64) -> f64
where
    F: Fn(f64) -> f64,
{
    (f(x + h) - f(x - h)) / (2.0 * h)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use std::collections::HashSet;

    // =============================================================================
    // ADMode Tests (Task 5.1)
    // =============================================================================

    #[test]
    fn test_admode_clone() {
        let mode = ADMode::Forward;
        let cloned = mode.clone();
        assert_eq!(mode, cloned);
    }

    #[test]
    fn test_admode_copy() {
        let mode = ADMode::Reverse;
        let copied = mode; // Copy trait allows this
        assert_eq!(mode, copied);
    }

    #[test]
    fn test_admode_debug() {
        let mode = ADMode::Inactive;
        let debug_str = format!("{:?}", mode);
        assert!(debug_str.contains("Inactive"));
    }

    #[test]
    fn test_admode_partial_eq() {
        assert_eq!(ADMode::Forward, ADMode::Forward);
        assert_ne!(ADMode::Forward, ADMode::Reverse);
        assert_ne!(ADMode::Inactive, ADMode::Forward);
    }

    #[test]
    fn test_admode_eq() {
        // Eq trait allows use in HashSet
        let mut set = HashSet::new();
        set.insert(ADMode::Forward);
        set.insert(ADMode::Reverse);
        set.insert(ADMode::Inactive);
        assert_eq!(set.len(), 3);
    }

    #[test]
    fn test_admode_hash() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        fn hash_value<T: Hash>(t: &T) -> u64 {
            let mut s = DefaultHasher::new();
            t.hash(&mut s);
            s.finish()
        }

        // Same values should hash the same
        assert_eq!(hash_value(&ADMode::Forward), hash_value(&ADMode::Forward));
        // Different values should (likely) hash differently
        assert_ne!(hash_value(&ADMode::Forward), hash_value(&ADMode::Reverse));
    }

    #[test]
    fn test_admode_is_forward() {
        assert!(!ADMode::Inactive.is_forward());
        assert!(ADMode::Forward.is_forward());
        assert!(!ADMode::Reverse.is_forward());
    }

    #[test]
    fn test_admode_is_reverse() {
        assert!(!ADMode::Inactive.is_reverse());
        assert!(!ADMode::Forward.is_reverse());
        assert!(ADMode::Reverse.is_reverse());
    }

    #[test]
    fn test_admode_is_active() {
        assert!(!ADMode::Inactive.is_active());
        assert!(ADMode::Forward.is_active());
        assert!(ADMode::Reverse.is_active());
    }

    #[test]
    fn test_admode_default() {
        // Default should be Inactive (no differentiation)
        assert_eq!(ADMode::default(), ADMode::Inactive);
    }

    // =============================================================================
    // Activity Tests (Task 5.1 - verify existing + ensure Copy)
    // =============================================================================

    #[test]
    fn test_activity_clone() {
        let activity = Activity::Dual;
        let cloned = activity.clone();
        assert_eq!(activity, cloned);
    }

    #[test]
    fn test_activity_copy() {
        let activity = Activity::Active;
        let copied = activity; // Copy trait allows this
        assert_eq!(activity, copied);
    }

    #[test]
    fn test_activity_debug() {
        let activity = Activity::Duplicated;
        let debug_str = format!("{:?}", activity);
        assert!(debug_str.contains("Duplicated"));
    }

    #[test]
    fn test_activity_partial_eq() {
        assert_eq!(Activity::Const, Activity::Const);
        assert_ne!(Activity::Const, Activity::Dual);
        assert_ne!(Activity::Active, Activity::Duplicated);
    }

    #[test]
    fn test_activity_eq() {
        // Eq trait allows use in HashSet
        let mut set = HashSet::new();
        set.insert(Activity::Const);
        set.insert(Activity::Dual);
        set.insert(Activity::Active);
        set.insert(Activity::Duplicated);
        set.insert(Activity::DuplicatedOnly);
        assert_eq!(set.len(), 5);
    }

    #[test]
    fn test_activity_hash() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        fn hash_value<T: Hash>(t: &T) -> u64 {
            let mut s = DefaultHasher::new();
            t.hash(&mut s);
            s.finish()
        }

        // Same values should hash the same
        assert_eq!(hash_value(&Activity::Dual), hash_value(&Activity::Dual));
        // Different values should (likely) hash differently
        assert_ne!(hash_value(&Activity::Const), hash_value(&Activity::Active));
    }

    #[test]
    fn test_activity_is_active() {
        assert!(!Activity::Const.is_active());
        assert!(Activity::Dual.is_active());
        assert!(Activity::Active.is_active());
        assert!(Activity::Duplicated.is_active());
        assert!(Activity::DuplicatedOnly.is_active());
    }

    #[test]
    fn test_activity_mode_detection() {
        assert!(Activity::Dual.is_forward_mode());
        assert!(!Activity::Dual.is_reverse_mode());

        assert!(!Activity::Active.is_forward_mode());
        assert!(Activity::Active.is_reverse_mode());

        assert!(Activity::Duplicated.is_reverse_mode());
        assert!(Activity::DuplicatedOnly.is_reverse_mode());
    }

    // =============================================================================
    // Gradient Tests (existing tests)
    // =============================================================================

    #[test]
    fn test_gradient_square() {
        // f(x) = x^2, f'(x) = 2x
        let grad = gradient(|x| x * x, 3.0);
        assert_relative_eq!(grad, 6.0, epsilon = 1e-6);
    }

    #[test]
    fn test_gradient_cubic() {
        // f(x) = x^3, f'(x) = 3x^2
        let grad = gradient(|x| x * x * x, 2.0);
        assert_relative_eq!(grad, 12.0, epsilon = 1e-5);
    }

    #[test]
    fn test_gradient_sin() {
        // f(x) = sin(x), f'(x) = cos(x)
        let grad = gradient(|x| x.sin(), 0.0);
        assert_relative_eq!(grad, 1.0, epsilon = 1e-6);
    }

    #[test]
    fn test_gradient_with_custom_step() {
        let grad = gradient_with_step(|x| x * x, 3.0, 1e-6);
        assert_relative_eq!(grad, 6.0, epsilon = 1e-4);
    }
}
