//! Checkpointable state for path observers.

use num_traits::Float;

/// Checkpointable state of a PathObserver.
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
