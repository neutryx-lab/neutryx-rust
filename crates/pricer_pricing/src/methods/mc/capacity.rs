//! Shared capacity growth strategy for workspace buffers.

/// Calculates the new capacity using a doubling strategy.
///
/// Returns `requested.max(current * 2)`, ensuring at least the
/// requested capacity while amortising allocation costs.
#[inline]
pub fn calculate_growth_capacity(current: usize, requested: usize) -> usize {
    requested.max(current * 2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_growth_when_requested_is_larger() {
        assert_eq!(calculate_growth_capacity(100, 500), 500);
    }

    #[test]
    fn test_growth_when_doubling_is_larger() {
        assert_eq!(calculate_growth_capacity(100, 150), 200);
    }

    #[test]
    fn test_growth_from_zero() {
        assert_eq!(calculate_growth_capacity(0, 10), 10);
    }

    #[test]
    fn test_growth_exact_double() {
        assert_eq!(calculate_growth_capacity(100, 200), 200);
    }
}
