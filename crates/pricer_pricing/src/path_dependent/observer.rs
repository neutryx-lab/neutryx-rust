//! PathObserver: Streaming statistics accumulation for path-dependent options.
//!
//! This module provides efficient path observation with streaming statistics
//! computation. Statistics are accumulated incrementally as prices are observed,
//! avoiding the need to store the full path.
//!
//! # Streaming Statistics
//!
//! - **Arithmetic average**: Running sum / count
//! - **Geometric average**: exp(running log sum / count)
//! - **Maximum**: Running maximum
//! - **Minimum**: Running minimum
//!
//! # Enzyme AD Compatibility
//!
//! All operations are branch-free on floating-point values and use smooth
//! approximations where needed, ensuring compatibility with Enzyme's LLVM-level
//! automatic differentiation.

use num_traits::Float;

/// Streaming path observation statistics.
///
/// Accumulates statistics about a price path incrementally, enabling
/// efficient computation of path-dependent payoffs without storing
/// the full path.
///
/// # Type Parameters
///
/// * `T` - Floating-point type (e.g., `f64`, `f32`, `Dual64`)
///
/// # Example
///
/// ```
/// use pricer_pricing::path_dependent::PathObserver;
///
/// let mut observer: PathObserver<f64> = PathObserver::new();
///
/// // Observe prices along the path
/// observer.observe(100.0);
/// observer.observe(105.0);
/// observer.observe(110.0);
/// observer.observe(95.0);
///
/// // Set terminal price
/// observer.set_terminal(100.0);
///
/// // Access statistics
/// println!("Arithmetic average: {}", observer.arithmetic_average());
/// println!("Maximum: {}", observer.maximum());
/// println!("Minimum: {}", observer.minimum());
/// ```
#[derive(Clone, Debug)]
pub struct PathObserver<T: Float> {
    /// Running sum for arithmetic average: Σ S_i
    running_sum: T,
    /// Running log sum for geometric average: Σ ln(S_i)
    running_product_log: T,
    /// Running maximum price observed
    running_max: T,
    /// Running minimum price observed
    running_min: T,
    /// Number of observations
    count: usize,
    /// Terminal price (typically the last observation)
    terminal: T,
}

impl<T: Float> PathObserver<T> {
    /// Creates a new empty path observer.
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_pricing::path_dependent::PathObserver;
    ///
    /// let observer: PathObserver<f64> = PathObserver::new();
    /// assert_eq!(observer.count(), 0);
    /// ```
    #[inline]
    pub fn new() -> Self {
        Self {
            running_sum: T::zero(),
            running_product_log: T::zero(),
            running_max: T::neg_infinity(),
            running_min: T::infinity(),
            count: 0,
            terminal: T::zero(),
        }
    }

    /// Observes a new price and updates running statistics.
    ///
    /// This method updates all streaming statistics incrementally:
    /// - Adds price to running sum
    /// - Adds ln(price) to log sum (for geometric average)
    /// - Updates max/min if necessary
    /// - Increments count
    ///
    /// # Arguments
    ///
    /// * `price` - The observed price (must be positive for geometric average)
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_pricing::path_dependent::PathObserver;
    ///
    /// let mut observer: PathObserver<f64> = PathObserver::new();
    /// observer.observe(100.0);
    /// observer.observe(110.0);
    /// assert_eq!(observer.count(), 2);
    /// ```
    #[inline]
    pub fn observe(&mut self, price: T) {
        self.running_sum = self.running_sum + price;
        self.running_product_log = self.running_product_log + price.ln();
        self.running_max = self.running_max.max(price);
        self.running_min = self.running_min.min(price);
        self.count += 1;
    }

    /// Sets the terminal price.
    ///
    /// The terminal price is used for payoff calculations that require
    /// the final spot price (e.g., barrier options).
    ///
    /// # Arguments
    ///
    /// * `price` - The terminal (final) price
    #[inline]
    pub fn set_terminal(&mut self, price: T) {
        self.terminal = price;
    }

    /// Resets all statistics to initial state.
    ///
    /// Call this before starting a new path simulation.
    #[inline]
    pub fn reset(&mut self) {
        self.running_sum = T::zero();
        self.running_product_log = T::zero();
        self.running_max = T::neg_infinity();
        self.running_min = T::infinity();
        self.count = 0;
        self.terminal = T::zero();
    }

    /// Returns the arithmetic average of observed prices.
    ///
    /// # Returns
    ///
    /// `Σ S_i / n` if `n > 0`, otherwise `0`.
    #[inline]
    pub fn arithmetic_average(&self) -> T {
        if self.count == 0 {
            T::zero()
        } else {
            self.running_sum / T::from(self.count).unwrap()
        }
    }

    /// Returns the geometric average of observed prices.
    ///
    /// Computed as `exp(Σ ln(S_i) / n)`.
    ///
    /// # Returns
    ///
    /// Geometric mean if `n > 0`, otherwise `0`.
    ///
    /// # Note
    ///
    /// Requires all observed prices to be positive.
    #[inline]
    pub fn geometric_average(&self) -> T {
        if self.count == 0 {
            T::zero()
        } else {
            (self.running_product_log / T::from(self.count).unwrap()).exp()
        }
    }

    /// Returns the maximum observed price.
    ///
    /// # Returns
    ///
    /// Maximum price, or `-inf` if no observations.
    #[inline]
    pub fn maximum(&self) -> T {
        self.running_max
    }

    /// Returns the minimum observed price.
    ///
    /// # Returns
    ///
    /// Minimum price, or `+inf` if no observations.
    #[inline]
    pub fn minimum(&self) -> T {
        self.running_min
    }

    /// Returns the terminal price.
    #[inline]
    pub fn terminal(&self) -> T {
        self.terminal
    }

    /// Returns the number of observations.
    #[inline]
    pub fn count(&self) -> usize {
        self.count
    }

    /// Creates a snapshot of the current state for checkpointing.
    ///
    /// # Returns
    ///
    /// A [`PathObserverState`] that can be used to restore the observer.
    #[inline]
    pub fn snapshot(&self) -> PathObserverState<T> {
        PathObserverState {
            running_sum: self.running_sum,
            running_product_log: self.running_product_log,
            running_max: self.running_max,
            running_min: self.running_min,
            count: self.count,
        }
    }

    /// Restores the observer from a checkpointed state.
    ///
    /// # Arguments
    ///
    /// * `state` - The state to restore from
    #[inline]
    pub fn restore(&mut self, state: &PathObserverState<T>) {
        self.running_sum = state.running_sum;
        self.running_product_log = state.running_product_log;
        self.running_max = state.running_max;
        self.running_min = state.running_min;
        self.count = state.count;
        // Note: terminal is not part of checkpoint state
    }
}

impl<T: Float> Default for PathObserver<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Checkpointable state of a PathObserver.
///
/// This struct contains the minimal state needed to resume path observation
/// from a checkpoint. Used for memory-efficient AD with checkpointing.
#[derive(Clone, Debug)]
pub struct PathObserverState<T: Float> {
    /// Running sum of prices
    pub running_sum: T,
    /// Running sum of log prices
    pub running_product_log: T,
    /// Maximum price observed
    pub running_max: T,
    /// Minimum price observed
    pub running_min: T,
    /// Number of observations
    pub count: usize,
}

impl<T: Float> Default for PathObserverState<T> {
    fn default() -> Self {
        Self {
            running_sum: T::zero(),
            running_product_log: T::zero(),
            running_max: T::neg_infinity(),
            running_min: T::infinity(),
            count: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_observer_default() {
        let observer: PathObserver<f64> = Default::default();
        assert_eq!(observer.count(), 0);
    }

    #[test]
    fn test_observer_clone() {
        let mut observer1: PathObserver<f64> = PathObserver::new();
        observer1.observe(100.0);
        observer1.observe(110.0);

        let observer2 = observer1.clone();
        assert_eq!(observer2.count(), 2);
        assert_relative_eq!(observer2.arithmetic_average(), 105.0, epsilon = 1e-10);
    }

    #[test]
    fn test_geometric_average_powers_of_two() {
        let mut observer: PathObserver<f64> = PathObserver::new();

        // Geometric mean of [1, 2, 4, 8] = (1*2*4*8)^(1/4) = 64^(1/4) = 2.828...
        observer.observe(1.0);
        observer.observe(2.0);
        observer.observe(4.0);
        observer.observe(8.0);

        let expected = (64.0_f64).powf(0.25);
        assert_relative_eq!(observer.geometric_average(), expected, epsilon = 1e-10);
    }

    #[test]
    fn test_state_default() {
        let state: PathObserverState<f64> = Default::default();
        assert_eq!(state.count, 0);
        assert_eq!(state.running_sum, 0.0);
    }
}
