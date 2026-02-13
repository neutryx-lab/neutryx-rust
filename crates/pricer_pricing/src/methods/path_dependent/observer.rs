//! PathObserver: Streaming statistics accumulation for path-dependent options.

use num_traits::Float;
pub use crate::checkpoint::PathObserverState;

/// Streaming path observation statistics.
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
    #[inline]
    pub fn observe(&mut self, price: T) {
        self.running_sum = self.running_sum + price;
        self.running_product_log = self.running_product_log + price.ln();
        self.running_max = self.running_max.max(price);
        self.running_min = self.running_min.min(price);
        self.count += 1;
    }

    /// Sets the terminal price.
    #[inline]
    pub fn set_terminal(&mut self, price: T) { self.terminal = price; }

    /// Resets all statistics to initial state.
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
    #[inline]
    pub fn arithmetic_average(&self) -> T {
        if self.count == 0 {
            T::zero()
        } else {
            self.running_sum / T::from(self.count).unwrap()
        }
    }

    /// Returns the geometric average of observed prices.
    #[inline]
    pub fn geometric_average(&self) -> T {
        if self.count == 0 {
            T::zero()
        } else {
            (self.running_product_log / T::from(self.count).unwrap()).exp()
        }
    }

    /// Returns the maximum observed price.
    #[inline]
    pub fn maximum(&self) -> T { self.running_max }

    /// Returns the minimum observed price.
    #[inline]
    pub fn minimum(&self) -> T { self.running_min }

    /// Returns the terminal price.
    #[inline]
    pub fn terminal(&self) -> T { self.terminal }

    /// Returns the number of observations.
    #[inline]
    pub fn count(&self) -> usize { self.count }

    /// Creates a snapshot of the current state for checkpointing.
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
    #[inline]
    pub fn restore(&mut self, state: &PathObserverState<T>) {
        self.running_sum = state.running_sum;
        self.running_product_log = state.running_product_log;
        self.running_max = state.running_max;
        self.running_min = state.running_min;
        self.count = state.count;
    }
}

impl<T: Float> Default for PathObserver<T> {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

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
