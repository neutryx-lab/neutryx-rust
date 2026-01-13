//! Result types for IRS Greeks calculation.

use num_traits::Float;

/// IRS Delta calculation result.
///
/// Contains tenor Deltas, DV01, and computation timing information.
///
/// # Type Parameters
///
/// * `T` - Floating-point type (e.g., `f64`, `Dual64`)
#[derive(Clone, Debug)]
pub struct IrsDeltaResult<T: Float> {
    /// Tenor points in years (e.g., [0.25, 0.5, 1.0, 2.0, ...]).
    pub tenors: Vec<T>,

    /// Delta values for each tenor point.
    pub deltas: Vec<T>,

    /// DV01: Present value change for 1bp parallel shift.
    pub dv01: T,

    /// Computation time in nanoseconds.
    pub compute_time_ns: u64,
}

impl<T: Float> IrsDeltaResult<T> {
    /// Creates a new IrsDeltaResult.
    pub fn new(tenors: Vec<T>, deltas: Vec<T>, dv01: T, compute_time_ns: u64) -> Self {
        Self {
            tenors,
            deltas,
            dv01,
            compute_time_ns,
        }
    }

    /// Returns the number of tenor points.
    #[inline]
    pub fn num_tenors(&self) -> usize {
        self.tenors.len()
    }

    /// Returns true if the result is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.tenors.is_empty()
    }
}

impl<T: Float> Default for IrsDeltaResult<T> {
    fn default() -> Self {
        Self {
            tenors: Vec::new(),
            deltas: Vec::new(),
            dv01: T::zero(),
            compute_time_ns: 0,
        }
    }
}

/// IRS Greeks calculation result with mode comparison.
///
/// Contains NPV and optional results from both AAD and bump-and-revalue
/// calculations for comparison and verification.
#[derive(Clone, Debug)]
pub struct IrsGreeksResult<T: Float> {
    /// Net Present Value of the swap.
    pub npv: T,

    /// AAD calculation result (if computed).
    pub aad_result: Option<IrsDeltaResult<T>>,

    /// Bump-and-Revalue calculation result (if computed).
    pub bump_result: Option<IrsDeltaResult<T>>,

    /// Relative errors between AAD and bump results (if both computed).
    pub accuracy_check: Option<Vec<T>>,
}

impl<T: Float> IrsGreeksResult<T> {
    /// Creates a new result with only NPV.
    pub fn new(npv: T) -> Self {
        Self {
            npv,
            aad_result: None,
            bump_result: None,
            accuracy_check: None,
        }
    }

    /// Sets the AAD result.
    pub fn with_aad_result(mut self, result: IrsDeltaResult<T>) -> Self {
        self.aad_result = Some(result);
        self
    }

    /// Sets the bump-and-revalue result.
    pub fn with_bump_result(mut self, result: IrsDeltaResult<T>) -> Self {
        self.bump_result = Some(result);
        self
    }

    /// Sets the accuracy check results.
    pub fn with_accuracy_check(mut self, errors: Vec<T>) -> Self {
        self.accuracy_check = Some(errors);
        self
    }

    /// Returns true if AAD result is available.
    #[inline]
    pub fn has_aad_result(&self) -> bool {
        self.aad_result.is_some()
    }

    /// Returns true if bump result is available.
    #[inline]
    pub fn has_bump_result(&self) -> bool {
        self.bump_result.is_some()
    }

    /// Returns the DV01 from the preferred result (AAD if available, else bump).
    pub fn dv01(&self) -> Option<T> {
        self.aad_result
            .as_ref()
            .map(|r| r.dv01)
            .or_else(|| self.bump_result.as_ref().map(|r| r.dv01))
    }

    /// Returns the deltas from the preferred result.
    pub fn deltas(&self) -> Option<&[T]> {
        self.aad_result
            .as_ref()
            .map(|r| r.deltas.as_slice())
            .or_else(|| self.bump_result.as_ref().map(|r| r.deltas.as_slice()))
    }
}

impl<T: Float> Default for IrsGreeksResult<T> {
    fn default() -> Self {
        Self {
            npv: T::zero(),
            aad_result: None,
            bump_result: None,
            accuracy_check: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delta_result_new() {
        let tenors = vec![0.25, 0.5, 1.0, 2.0, 5.0];
        let deltas = vec![100.0, 200.0, 400.0, 800.0, 2000.0];
        let dv01 = 3500.0;
        let compute_time = 1000u64;

        let result = IrsDeltaResult::new(tenors.clone(), deltas.clone(), dv01, compute_time);

        assert_eq!(result.num_tenors(), 5);
        assert!(!result.is_empty());
        assert!((result.dv01 - 3500.0).abs() < 1e-10);
        assert_eq!(result.compute_time_ns, 1000);
    }

    #[test]
    fn test_delta_result_default() {
        let result: IrsDeltaResult<f64> = IrsDeltaResult::default();
        assert!(result.is_empty());
        assert_eq!(result.num_tenors(), 0);
        assert!((result.dv01 - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_greeks_result_new() {
        let result = IrsGreeksResult::<f64>::new(1000.0);
        assert!((result.npv - 1000.0).abs() < 1e-10);
        assert!(!result.has_aad_result());
        assert!(!result.has_bump_result());
    }

    #[test]
    fn test_greeks_result_with_aad() {
        let delta_result = IrsDeltaResult::new(vec![1.0], vec![100.0], 100.0, 1000);
        let result = IrsGreeksResult::<f64>::new(1000.0).with_aad_result(delta_result);

        assert!(result.has_aad_result());
        assert!(result.dv01().is_some());
        assert!((result.dv01().unwrap() - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_greeks_result_with_both() {
        let aad_result = IrsDeltaResult::new(vec![1.0], vec![100.0], 100.0, 1000);
        let bump_result = IrsDeltaResult::new(vec![1.0], vec![100.5], 100.5, 5000);

        let result = IrsGreeksResult::<f64>::new(1000.0)
            .with_aad_result(aad_result)
            .with_bump_result(bump_result)
            .with_accuracy_check(vec![0.005]);

        assert!(result.has_aad_result());
        assert!(result.has_bump_result());
        assert!(result.accuracy_check.is_some());
        // AAD result is preferred
        assert!((result.dv01().unwrap() - 100.0).abs() < 1e-10);
    }
}
