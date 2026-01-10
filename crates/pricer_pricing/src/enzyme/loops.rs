//! Enzyme-compatible loop patterns for automatic differentiation.
//!
//! This module provides fixed-size iteration patterns that are compatible
//! with Enzyme LLVM-level automatic differentiation. Enzyme requires
//! deterministic control flow at compile time, which means:
//!
//! - **Fixed-size `for` loops**: Use `for i in 0..N` where N is known
//! - **No `while` loops**: Enzyme cannot analyse dynamic loop bounds
//! - **No iterator adaptors with dynamic termination**: Avoid `take_while`, `skip_while`
//!
//! # Enzyme Compatibility Guidelines
//!
//! ## Compatible Patterns
//!
//! ```rust,ignore
//! // Fixed-size for loop - GOOD
//! for i in 0..n_steps {
//!     // process step i
//! }
//!
//! // Range-based iteration with known bounds - GOOD
//! for (idx, value) in data.iter().enumerate().take(n_steps) {
//!     // process value at idx
//! }
//! ```
//!
//! ## Incompatible Patterns
//!
//! ```rust,ignore
//! // while loop - BAD (dynamic termination)
//! while condition {
//!     // ...
//! }
//!
//! // take_while - BAD (dynamic termination)
//! for x in iter.take_while(|x| x.is_valid()) {
//!     // ...
//! }
//!
//! // Early return in loop - BAD (breaks Enzyme analysis)
//! for i in 0..n {
//!     if should_stop() { break; }
//! }
//! ```
//!
//! # Provided Types
//!
//! - [`FixedSteps`]: Iterator for fixed-size path simulation steps
//! - [`FixedPaths`]: Iterator for fixed-size path batch iteration
//! - [`EnzymeLoop`]: Wrapper ensuring Enzyme-compatible iteration
//!
//! # Example: Enzyme-Compatible Path Generation
//!
//! ```rust
//! use pricer_pricing::enzyme::loops::{FixedSteps, FixedPaths, EnzymeLoop};
//!
//! // Simulate paths with fixed step count
//! let n_paths = 1000usize;
//! let n_steps = 252usize;
//!
//! for path_idx in FixedPaths::new(n_paths) {
//!     let mut spot = 100.0f64;
//!     for step_idx in FixedSteps::new(n_steps) {
//!         // GBM step - all operations are Enzyme-compatible
//!         spot *= 1.0 + 0.05 / 252.0 + 0.2 * (1.0 / 252.0_f64).sqrt() * 0.1;
//!     }
//! }
//! ```

use num_traits::Float;

/// Fixed-size iterator for simulation time steps.
///
/// This iterator provides Enzyme-compatible iteration over time steps
/// in a path simulation. Unlike `0..n`, this type carries semantic
/// meaning and can be used with Enzyme's activity analysis.
///
/// # Example
///
/// ```rust
/// use pricer_pricing::enzyme::loops::FixedSteps;
///
/// let steps = FixedSteps::new(252);
/// assert_eq!(steps.len(), 252);
///
/// for step in steps {
///     // Process step
/// }
/// ```
#[derive(Clone, Copy, Debug)]
pub struct FixedSteps {
    current: usize,
    total: usize,
}

impl FixedSteps {
    /// Creates a new fixed-step iterator.
    ///
    /// # Arguments
    ///
    /// * `n_steps` - Total number of steps (must be > 0 for meaningful iteration)
    #[inline]
    pub fn new(n_steps: usize) -> Self {
        Self {
            current: 0,
            total: n_steps,
        }
    }

    /// Returns the total number of steps.
    #[inline]
    pub fn len(&self) -> usize {
        self.total
    }

    /// Returns true if the iterator is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.total == 0
    }

    /// Returns the number of remaining steps.
    #[inline]
    pub fn remaining(&self) -> usize {
        self.total.saturating_sub(self.current)
    }
}

impl Iterator for FixedSteps {
    type Item = usize;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.current < self.total {
            let idx = self.current;
            self.current += 1;
            Some(idx)
        } else {
            None
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.remaining();
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for FixedSteps {}

/// Fixed-size iterator for path batch iteration.
///
/// This iterator provides Enzyme-compatible iteration over simulation
/// paths. It guarantees that the loop bound is fixed at construction
/// time, enabling Enzyme's static analysis.
///
/// # Example
///
/// ```rust
/// use pricer_pricing::enzyme::loops::FixedPaths;
///
/// let paths = FixedPaths::new(10000);
/// assert_eq!(paths.len(), 10000);
///
/// for path_idx in paths {
///     // Simulate path
/// }
/// ```
#[derive(Clone, Copy, Debug)]
pub struct FixedPaths {
    current: usize,
    total: usize,
}

impl FixedPaths {
    /// Creates a new fixed-path iterator.
    ///
    /// # Arguments
    ///
    /// * `n_paths` - Total number of paths (must be > 0 for meaningful iteration)
    #[inline]
    pub fn new(n_paths: usize) -> Self {
        Self {
            current: 0,
            total: n_paths,
        }
    }

    /// Returns the total number of paths.
    #[inline]
    pub fn len(&self) -> usize {
        self.total
    }

    /// Returns true if the iterator is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.total == 0
    }

    /// Returns the number of remaining paths.
    #[inline]
    pub fn remaining(&self) -> usize {
        self.total.saturating_sub(self.current)
    }
}

impl Iterator for FixedPaths {
    type Item = usize;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.current < self.total {
            let idx = self.current;
            self.current += 1;
            Some(idx)
        } else {
            None
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.remaining();
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for FixedPaths {}

/// Wrapper type ensuring Enzyme-compatible iteration.
///
/// This type wraps any `ExactSizeIterator` and provides guarantees
/// that the iteration count is fixed. It prevents accidental use
/// of dynamic termination patterns.
///
/// # Example
///
/// ```rust
/// use pricer_pricing::enzyme::loops::EnzymeLoop;
///
/// // Wrap a standard range in EnzymeLoop for clarity
/// let enzyme_iter = EnzymeLoop::from_range(0..100);
///
/// for i in enzyme_iter {
///     // Enzyme-compatible iteration
/// }
/// ```
#[derive(Clone, Debug)]
pub struct EnzymeLoop<I: ExactSizeIterator> {
    inner: I,
    total: usize,
}

impl<I: ExactSizeIterator> EnzymeLoop<I> {
    /// Creates a new Enzyme-compatible loop from an exact-size iterator.
    ///
    /// # Arguments
    ///
    /// * `iter` - Any iterator implementing `ExactSizeIterator`
    #[inline]
    pub fn new(iter: I) -> Self {
        let total = iter.len();
        Self { inner: iter, total }
    }

    /// Returns the total iteration count (fixed at construction).
    #[inline]
    pub fn len(&self) -> usize {
        self.total
    }

    /// Returns true if the iterator is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.total == 0
    }
}

impl<T> EnzymeLoop<std::ops::Range<T>>
where
    std::ops::Range<T>: ExactSizeIterator,
{
    /// Creates an EnzymeLoop from a range.
    ///
    /// # Example
    ///
    /// ```rust
    /// use pricer_pricing::enzyme::loops::EnzymeLoop;
    ///
    /// let iter = EnzymeLoop::from_range(0..10);
    /// assert_eq!(iter.len(), 10);
    /// ```
    #[inline]
    pub fn from_range(range: std::ops::Range<T>) -> Self {
        Self::new(range)
    }
}

impl<I: ExactSizeIterator> Iterator for EnzymeLoop<I> {
    type Item = I::Item;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<I: ExactSizeIterator> ExactSizeIterator for EnzymeLoop<I> {}

/// Trait for Enzyme-compatible aggregation operations.
///
/// This trait provides accumulation methods that are compatible with
/// Enzyme's reverse mode AD. It ensures that reduction operations
/// can be properly differentiated.
pub trait EnzymeAccumulate<T: Float> {
    /// Accumulates values using addition (Enzyme-compatible).
    ///
    /// This is the preferred method for summing values in AD contexts
    /// as it uses a simple addition which Enzyme can differentiate.
    fn enzyme_sum(&mut self, value: T);

    /// Accumulates values using weighted addition.
    ///
    /// Useful for weighted averages where the weight is a constant
    /// (not differentiated).
    fn enzyme_weighted_sum(&mut self, value: T, weight: T);
}

/// Thread-local accumulator for parallel AD computations.
///
/// This struct provides a thread-safe way to accumulate adjoint values
/// during parallel Monte Carlo simulations. Each thread maintains its
/// own accumulator, which are later reduced.
///
/// # Example
///
/// ```rust
/// use pricer_pricing::enzyme::loops::AdjointAccumulator;
///
/// let mut acc: AdjointAccumulator<f64> = AdjointAccumulator::new();
///
/// // Accumulate delta contributions
/// acc.add_delta(0.5);
/// acc.add_delta(0.3);
///
/// assert!((acc.delta() - 0.8).abs() < 1e-10);
/// ```
#[derive(Clone, Copy, Debug, Default)]
pub struct AdjointAccumulator<T: Float> {
    delta: T,
    gamma: T,
    vega: T,
    theta: T,
    rho: T,
    count: usize,
}

impl<T: Float> AdjointAccumulator<T> {
    /// Creates a new accumulator with all values zeroed.
    #[inline]
    pub fn new() -> Self {
        Self {
            delta: T::zero(),
            gamma: T::zero(),
            vega: T::zero(),
            theta: T::zero(),
            rho: T::zero(),
            count: 0,
        }
    }

    /// Adds a delta contribution.
    #[inline]
    pub fn add_delta(&mut self, value: T) {
        self.delta = self.delta + value;
        self.count += 1;
    }

    /// Adds a gamma contribution.
    #[inline]
    pub fn add_gamma(&mut self, value: T) {
        self.gamma = self.gamma + value;
    }

    /// Adds a vega contribution.
    #[inline]
    pub fn add_vega(&mut self, value: T) {
        self.vega = self.vega + value;
    }

    /// Adds a theta contribution.
    #[inline]
    pub fn add_theta(&mut self, value: T) {
        self.theta = self.theta + value;
    }

    /// Adds a rho contribution.
    #[inline]
    pub fn add_rho(&mut self, value: T) {
        self.rho = self.rho + value;
    }

    /// Adds all Greeks from another accumulator.
    #[inline]
    pub fn merge(&mut self, other: &Self) {
        self.delta = self.delta + other.delta;
        self.gamma = self.gamma + other.gamma;
        self.vega = self.vega + other.vega;
        self.theta = self.theta + other.theta;
        self.rho = self.rho + other.rho;
        self.count += other.count;
    }

    /// Returns the accumulated delta.
    #[inline]
    pub fn delta(&self) -> T {
        self.delta
    }

    /// Returns the accumulated gamma.
    #[inline]
    pub fn gamma(&self) -> T {
        self.gamma
    }

    /// Returns the accumulated vega.
    #[inline]
    pub fn vega(&self) -> T {
        self.vega
    }

    /// Returns the accumulated theta.
    #[inline]
    pub fn theta(&self) -> T {
        self.theta
    }

    /// Returns the accumulated rho.
    #[inline]
    pub fn rho(&self) -> T {
        self.rho
    }

    /// Returns the number of contributions.
    #[inline]
    pub fn count(&self) -> usize {
        self.count
    }

    /// Returns averaged Greeks (divided by count).
    #[inline]
    pub fn averaged(&self) -> (T, T, T, T, T)
    where
        T: std::ops::Div<Output = T>,
    {
        if self.count == 0 {
            return (T::zero(), T::zero(), T::zero(), T::zero(), T::zero());
        }
        let n = T::from(self.count).unwrap_or(T::one());
        (
            self.delta / n,
            self.gamma / n,
            self.vega / n,
            self.theta / n,
            self.rho / n,
        )
    }

    /// Resets all accumulated values to zero.
    #[inline]
    pub fn reset(&mut self) {
        self.delta = T::zero();
        self.gamma = T::zero();
        self.vega = T::zero();
        self.theta = T::zero();
        self.rho = T::zero();
        self.count = 0;
    }
}

/// Enzyme-compatible GBM path generator with fixed iterations.
///
/// This struct generates Geometric Brownian Motion paths using only
/// fixed-size for loops, making it compatible with Enzyme AD.
///
/// # Example
///
/// ```rust
/// use pricer_pricing::enzyme::loops::EnzymeGbmGenerator;
///
/// let gen = EnzymeGbmGenerator::new(100.0, 0.05, 0.2, 1.0, 252);
///
/// // Generate terminal spots (would use actual random numbers in practice)
/// let randoms = vec![0.1; 252];
/// let terminal = gen.generate_terminal(&randoms);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct EnzymeGbmGenerator<T: Float> {
    spot: T,
    drift: T,
    vol_sqrt_dt: T,
    n_steps: usize,
}

impl<T: Float> EnzymeGbmGenerator<T> {
    /// Creates a new Enzyme-compatible GBM generator.
    ///
    /// # Arguments
    ///
    /// * `spot` - Initial spot price
    /// * `rate` - Risk-free rate (annual)
    /// * `vol` - Volatility (annual)
    /// * `maturity` - Time to maturity (years)
    /// * `n_steps` - Number of time steps (fixed)
    #[inline]
    pub fn new(spot: T, rate: T, vol: T, maturity: T, n_steps: usize) -> Self {
        let dt = maturity / T::from(n_steps).unwrap();
        let drift = (rate - vol * vol / T::from(2.0).unwrap()) * dt;
        let vol_sqrt_dt = vol * dt.sqrt();

        Self {
            spot,
            drift,
            vol_sqrt_dt,
            n_steps,
        }
    }

    /// Returns the number of steps (fixed at construction).
    #[inline]
    pub fn n_steps(&self) -> usize {
        self.n_steps
    }

    /// Generates a terminal spot price using provided random numbers.
    ///
    /// This method uses a fixed-size for loop internally, making it
    /// compatible with Enzyme AD.
    ///
    /// # Arguments
    ///
    /// * `randoms` - Slice of standard normal random numbers (length must equal n_steps)
    ///
    /// # Panics
    ///
    /// Panics if `randoms.len() != self.n_steps`
    #[inline]
    pub fn generate_terminal(&self, randoms: &[T]) -> T {
        assert_eq!(
            randoms.len(),
            self.n_steps,
            "Random numbers must match step count"
        );

        let mut log_spot = self.spot.ln();

        // Fixed-size for loop - Enzyme compatible
        for step_idx in 0..self.n_steps {
            log_spot = log_spot + self.drift + self.vol_sqrt_dt * randoms[step_idx];
        }

        log_spot.exp()
    }

    /// Generates a full path using provided random numbers.
    ///
    /// Returns a vector of spot prices at each time step.
    ///
    /// # Arguments
    ///
    /// * `randoms` - Slice of standard normal random numbers (length must equal n_steps)
    ///
    /// # Panics
    ///
    /// Panics if `randoms.len() != self.n_steps`
    pub fn generate_path(&self, randoms: &[T]) -> Vec<T> {
        assert_eq!(
            randoms.len(),
            self.n_steps,
            "Random numbers must match step count"
        );

        let mut path = Vec::with_capacity(self.n_steps + 1);
        let mut log_spot = self.spot.ln();

        path.push(self.spot);

        // Fixed-size for loop - Enzyme compatible
        for step_idx in 0..self.n_steps {
            log_spot = log_spot + self.drift + self.vol_sqrt_dt * randoms[step_idx];
            path.push(log_spot.exp());
        }

        path
    }

    /// Generates terminal spot and writes path to provided buffer.
    ///
    /// This is the most efficient variant as it avoids allocation.
    ///
    /// # Arguments
    ///
    /// * `randoms` - Slice of standard normal random numbers
    /// * `path_buffer` - Buffer to write path into (will be resized)
    ///
    /// # Returns
    ///
    /// Terminal spot price
    #[inline]
    pub fn generate_terminal_with_path(&self, randoms: &[T], path_buffer: &mut Vec<T>) -> T {
        assert_eq!(
            randoms.len(),
            self.n_steps,
            "Random numbers must match step count"
        );

        path_buffer.clear();
        path_buffer.reserve(self.n_steps + 1);

        let mut log_spot = self.spot.ln();
        path_buffer.push(self.spot);

        // Fixed-size for loop - Enzyme compatible
        for step_idx in 0..self.n_steps {
            log_spot = log_spot + self.drift + self.vol_sqrt_dt * randoms[step_idx];
            path_buffer.push(log_spot.exp());
        }

        *path_buffer.last().unwrap()
    }
}

/// Marker trait for Enzyme-compatible functions.
///
/// This trait serves as documentation that a function follows
/// Enzyme compatibility guidelines:
///
/// 1. Uses only fixed-size for loops
/// 2. Has no dynamic control flow (while loops, early returns)
/// 3. Uses smooth approximations for discontinuities
/// 4. Has differentiable mathematical operations
///
/// # Implementation Note
///
/// This is a marker trait with no methods. It exists to document
/// intent and can be used with compile-time assertions.
pub trait EnzymeCompatible {}

/// Compile-time assertion macro for Enzyme compatibility.
///
/// This macro provides documentation that a code block is designed
/// for Enzyme compatibility. While it cannot enforce all rules at
/// compile time, it serves as a reminder for code reviewers.
///
/// # Example
///
/// ```rust
/// use pricer_pricing::enzyme_compatible;
///
/// fn price_option(spot: f64, strike: f64, vol: f64, n_steps: usize) -> f64 {
///     enzyme_compatible! {
///         let mut sum = 0.0;
///         // Fixed-size for loop
///         for i in 0..n_steps {
///             sum += vol * (i as f64);
///         }
///         sum
///     }
/// }
/// ```
#[macro_export]
macro_rules! enzyme_compatible {
    ($($body:tt)*) => {{
        // This block is marked as Enzyme-compatible.
        // Reviewers should verify:
        // 1. No while loops
        // 2. No early returns or breaks
        // 3. Fixed-size for loops only
        // 4. Smooth approximations for discontinuities
        $($body)*
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_steps_basic() {
        let steps = FixedSteps::new(10);
        assert_eq!(steps.len(), 10);
        assert!(!steps.is_empty());

        let collected: Vec<_> = steps.collect();
        assert_eq!(collected, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    }

    #[test]
    fn test_fixed_steps_empty() {
        let steps = FixedSteps::new(0);
        assert_eq!(steps.len(), 0);
        assert!(steps.is_empty());

        let collected: Vec<_> = steps.collect();
        assert!(collected.is_empty());
    }

    #[test]
    fn test_fixed_steps_remaining() {
        let mut steps = FixedSteps::new(5);
        assert_eq!(steps.remaining(), 5);

        steps.next();
        assert_eq!(steps.remaining(), 4);

        steps.next();
        steps.next();
        assert_eq!(steps.remaining(), 2);
    }

    #[test]
    fn test_fixed_paths_basic() {
        let paths = FixedPaths::new(100);
        assert_eq!(paths.len(), 100);

        let count = paths.count();
        assert_eq!(count, 100);
    }

    #[test]
    fn test_enzyme_loop_from_range() {
        let iter = EnzymeLoop::from_range(0..5);
        assert_eq!(iter.len(), 5);

        let collected: Vec<_> = iter.collect();
        assert_eq!(collected, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn test_enzyme_loop_exact_size() {
        let iter = EnzymeLoop::new(vec![1, 2, 3].into_iter());
        assert_eq!(iter.len(), 3);
    }

    #[test]
    fn test_adjoint_accumulator_basic() {
        let mut acc = AdjointAccumulator::<f64>::new();

        acc.add_delta(0.5);
        acc.add_delta(0.3);
        acc.add_gamma(0.01);
        acc.add_vega(10.0);

        assert!((acc.delta() - 0.8).abs() < 1e-10);
        assert!((acc.gamma() - 0.01).abs() < 1e-10);
        assert!((acc.vega() - 10.0).abs() < 1e-10);
        assert_eq!(acc.count(), 2);
    }

    #[test]
    fn test_adjoint_accumulator_merge() {
        let mut acc1 = AdjointAccumulator::<f64>::new();
        acc1.add_delta(0.5);
        acc1.add_gamma(0.01);

        let mut acc2 = AdjointAccumulator::<f64>::new();
        acc2.add_delta(0.3);
        acc2.add_gamma(0.02);

        acc1.merge(&acc2);

        assert!((acc1.delta() - 0.8).abs() < 1e-10);
        assert!((acc1.gamma() - 0.03).abs() < 1e-10);
    }

    #[test]
    fn test_adjoint_accumulator_averaged() {
        let mut acc = AdjointAccumulator::<f64>::new();
        acc.add_delta(1.0);
        acc.add_delta(2.0);
        acc.add_delta(3.0);

        let (avg_delta, _, _, _, _) = acc.averaged();
        assert!((avg_delta - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_adjoint_accumulator_reset() {
        let mut acc = AdjointAccumulator::<f64>::new();
        acc.add_delta(1.0);
        acc.add_gamma(0.5);

        acc.reset();

        assert_eq!(acc.delta(), 0.0);
        assert_eq!(acc.gamma(), 0.0);
        assert_eq!(acc.count(), 0);
    }

    #[test]
    fn test_enzyme_gbm_generator_terminal() {
        let gen = EnzymeGbmGenerator::new(100.0, 0.05, 0.2, 1.0, 10);

        // Use zero random numbers (deterministic drift only)
        let randoms = vec![0.0; 10];
        let terminal = gen.generate_terminal(&randoms);

        // With zero volatility shocks, expect pure drift
        // log(S_T) = log(S_0) + (r - σ²/2) * T
        let expected = 100.0 * ((0.05 - 0.02) * 1.0).exp();
        assert!((terminal - expected).abs() < 1e-10);
    }

    #[test]
    fn test_enzyme_gbm_generator_path() {
        let gen = EnzymeGbmGenerator::new(100.0, 0.05, 0.2, 1.0, 5);

        let randoms = vec![0.0; 5];
        let path = gen.generate_path(&randoms);

        assert_eq!(path.len(), 6); // n_steps + 1
        assert_eq!(path[0], 100.0); // Initial spot
    }

    #[test]
    fn test_enzyme_gbm_generator_n_steps() {
        let gen = EnzymeGbmGenerator::new(100.0_f64, 0.05, 0.2, 1.0, 252);
        assert_eq!(gen.n_steps(), 252);
    }

    #[test]
    #[should_panic(expected = "Random numbers must match step count")]
    fn test_enzyme_gbm_generator_wrong_randoms_length() {
        let gen = EnzymeGbmGenerator::new(100.0_f64, 0.05, 0.2, 1.0, 10);
        let randoms = vec![0.0; 5]; // Wrong length
        gen.generate_terminal(&randoms);
    }

    #[test]
    fn test_enzyme_compatible_macro() {
        let result = enzyme_compatible! {
            let mut sum = 0.0;
            for i in 0..10 {
                sum += i as f64;
            }
            sum
        };

        assert!((result - 45.0).abs() < 1e-10);
    }

    #[test]
    fn test_fixed_steps_size_hint() {
        let steps = FixedSteps::new(10);
        assert_eq!(steps.size_hint(), (10, Some(10)));
    }

    #[test]
    fn test_fixed_paths_size_hint() {
        let paths = FixedPaths::new(100);
        assert_eq!(paths.size_hint(), (100, Some(100)));
    }

    #[test]
    fn test_nested_enzyme_loops() {
        let mut count = 0;

        for _path in FixedPaths::new(10) {
            for _step in FixedSteps::new(5) {
                count += 1;
            }
        }

        assert_eq!(count, 50);
    }

    #[test]
    fn test_enzyme_gbm_with_path_buffer() {
        let gen = EnzymeGbmGenerator::new(100.0_f64, 0.05, 0.2, 1.0, 5);
        let randoms = vec![0.0; 5];
        let mut path_buffer = Vec::new();

        let terminal = gen.generate_terminal_with_path(&randoms, &mut path_buffer);

        assert_eq!(path_buffer.len(), 6);
        assert_eq!(path_buffer[0], 100.0);
        assert!((terminal - path_buffer[5]).abs() < 1e-10);
    }
}
