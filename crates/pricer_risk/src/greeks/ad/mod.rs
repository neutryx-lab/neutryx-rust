//! Enzyme autodiff bindings for pricer_pricing.

pub mod binder;
pub mod checkpoint_ad;
pub mod enzyme_greeks;
pub mod fallback;
pub mod forward;
pub mod implicit_solver;
pub mod kernel;
pub mod loops;
pub mod parallel;
pub mod reverse;
pub mod shadow;
pub mod smooth;
pub mod verification;
pub mod wrappers;

/// AD mode (automatic differentiation mode).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum ADMode {
    /// No differentiation; the function is evaluated without AD.
    #[default]
    Inactive,
    /// Forward-mode AD (tangent propagation).
    Forward,
    /// Reverse-mode AD (adjoint/back-propagation).
    Reverse,
}

impl ADMode {
    /// Returns true if this is forward mode.
    #[inline]
    pub fn is_forward(&self) -> bool { matches!(self, ADMode::Forward) }

    /// Returns true if this is reverse mode.
    #[inline]
    pub fn is_reverse(&self) -> bool { matches!(self, ADMode::Reverse) }

    /// Returns true if AD is active (not Inactive).
    #[inline]
    pub fn is_active(&self) -> bool { !matches!(self, ADMode::Inactive) }
}

/// Activity annotations for autodiff parameters.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Activity {
    /// Parameter is constant and excluded from differentiation.
    Const,
    /// Forward-mode dual number: carries both primal and tangent values.
    Dual,
    /// Reverse-mode active scalar: receives an adjoint during back-propagation.
    Active,
    /// Reverse-mode duplicated pointer: shadow allocation stores the adjoint.
    Duplicated,
    /// Like `Duplicated`, but only the shadow (adjoint) is returned.
    DuplicatedOnly,
}

impl Activity {
    /// Returns true if this activity participates in differentiation.
    #[inline]
    pub fn is_active(&self) -> bool { !matches!(self, Activity::Const) }

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
    pub fn is_forward_mode(&self) -> bool { matches!(self, Activity::Dual) }
}

/// Compute gradient of `f` at `x` using central finite difference (placeholder
/// for Enzyme).
#[inline]
pub fn gradient<F>(f: F, x: f64) -> f64
where
    F: Fn(f64) -> f64,
{
    const H: f64 = 1e-8;
    (f(x + H) - f(x - H)) / (2.0 * H)
}

/// Compute gradient of `f` at `x` with custom step size.
#[inline]
pub fn gradient_with_step<F>(f: F, x: f64, h: f64) -> f64
where
    F: Fn(f64) -> f64,
{
    (f(x + h) - f(x - h)) / (2.0 * h)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn test_admode_clone() {
        let mode = ADMode::Forward;
        let cloned = mode.clone();
        assert_eq!(mode, cloned);
    }

    #[test]
    fn test_admode_copy() {
        let mode = ADMode::Reverse;
        let copied = mode;
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
        let mut set = HashSet::new();
        set.insert(ADMode::Forward);
        set.insert(ADMode::Reverse);
        set.insert(ADMode::Inactive);
        assert_eq!(set.len(), 3);
    }

    #[test]
    fn test_admode_hash() {
        use std::{
            collections::hash_map::DefaultHasher,
            hash::{Hash, Hasher},
        };

        fn hash_value<T: Hash>(t: &T) -> u64 {
            let mut s = DefaultHasher::new();
            t.hash(&mut s);
            s.finish()
        }

        assert_eq!(hash_value(&ADMode::Forward), hash_value(&ADMode::Forward));
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
        assert_eq!(ADMode::default(), ADMode::Inactive);
    }

    #[test]
    fn test_activity_clone() {
        let activity = Activity::Dual;
        let cloned = activity.clone();
        assert_eq!(activity, cloned);
    }

    #[test]
    fn test_activity_copy() {
        let activity = Activity::Active;
        let copied = activity;
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
        use std::{
            collections::hash_map::DefaultHasher,
            hash::{Hash, Hasher},
        };

        fn hash_value<T: Hash>(t: &T) -> u64 {
            let mut s = DefaultHasher::new();
            t.hash(&mut s);
            s.finish()
        }

        assert_eq!(hash_value(&Activity::Dual), hash_value(&Activity::Dual));
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

    #[test]
    fn test_gradient_square() {
        let grad = gradient(|x| x * x, 3.0);
        assert_relative_eq!(grad, 6.0, epsilon = 1e-6);
    }

    #[test]
    fn test_gradient_cubic() {
        let grad = gradient(|x| x * x * x, 2.0);
        assert_relative_eq!(grad, 12.0, epsilon = 1e-5);
    }

    #[test]
    fn test_gradient_sin() {
        let grad = gradient(|x| x.sin(), 0.0);
        assert_relative_eq!(grad, 1.0, epsilon = 1e-6);
    }

    #[test]
    fn test_gradient_with_custom_step() {
        let grad = gradient_with_step(|x| x * x, 3.0, 1e-6);
        assert_relative_eq!(grad, 6.0, epsilon = 1e-4);
    }
}
