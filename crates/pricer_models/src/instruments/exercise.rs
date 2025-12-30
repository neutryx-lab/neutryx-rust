//! Option exercise style definitions.
//!
//! This module provides exercise style types for options
//! including European, American, Bermudan, and Asian styles.

use num_traits::Float;

/// Option exercise style.
///
/// Defines when an option can be exercised during its lifetime.
///
/// # Type Parameters
/// * `T` - Floating-point type for time values (e.g., `f64`, `Dual64`)
///
/// # Variants
/// - `European`: Exercise only at expiry
/// - `American`: Exercise at any time before expiry
/// - `Bermudan`: Exercise on specified dates
/// - `Asian`: Path-dependent with averaging
///
/// # Examples
/// ```
/// use pricer_models::instruments::ExerciseStyle;
///
/// let european: ExerciseStyle<f64> = ExerciseStyle::European;
/// let american: ExerciseStyle<f64> = ExerciseStyle::American;
/// let bermudan = ExerciseStyle::Bermudan {
///     exercise_dates: vec![0.5, 0.75],
/// };
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum ExerciseStyle<T: Float> {
    /// European style: exercise only at expiry.
    European,

    /// American style: exercise at any time before expiry.
    American,

    /// Bermudan style: exercise on specific dates.
    Bermudan {
        /// Vector of exercise dates (times in years from now)
        exercise_dates: Vec<T>,
    },

    /// Asian style: path-dependent with averaging.
    Asian {
        /// Start time for averaging period (years from now)
        averaging_start: T,
        /// End time for averaging period (years from now)
        averaging_end: T,
        /// Number of observations for averaging
        num_observations: usize,
    },
}

impl<T: Float> ExerciseStyle<T> {
    /// Creates a new European exercise style.
    #[inline]
    pub fn european() -> Self {
        ExerciseStyle::European
    }

    /// Creates a new American exercise style.
    #[inline]
    pub fn american() -> Self {
        ExerciseStyle::American
    }

    /// Creates a new Bermudan exercise style with specified exercise dates.
    ///
    /// # Arguments
    /// * `exercise_dates` - Vector of times (in years) when option can be exercised
    ///
    /// # Examples
    /// ```
    /// use pricer_models::instruments::ExerciseStyle;
    ///
    /// let bermudan = ExerciseStyle::bermudan(vec![0.25, 0.5, 0.75, 1.0]);
    /// ```
    #[inline]
    pub fn bermudan(exercise_dates: Vec<T>) -> Self {
        ExerciseStyle::Bermudan { exercise_dates }
    }

    /// Creates a new Asian exercise style with averaging parameters.
    ///
    /// # Arguments
    /// * `averaging_start` - Start of averaging period (years from now)
    /// * `averaging_end` - End of averaging period (years from now)
    /// * `num_observations` - Number of observation points for averaging
    ///
    /// # Examples
    /// ```
    /// use pricer_models::instruments::ExerciseStyle;
    ///
    /// let asian = ExerciseStyle::asian(0.0, 1.0, 12); // Monthly averaging
    /// ```
    #[inline]
    pub fn asian(averaging_start: T, averaging_end: T, num_observations: usize) -> Self {
        ExerciseStyle::Asian {
            averaging_start,
            averaging_end,
            num_observations,
        }
    }

    /// Returns whether this is a European exercise style.
    #[inline]
    pub fn is_european(&self) -> bool {
        matches!(self, ExerciseStyle::European)
    }

    /// Returns whether this is an American exercise style.
    #[inline]
    pub fn is_american(&self) -> bool {
        matches!(self, ExerciseStyle::American)
    }

    /// Returns whether this is a Bermudan exercise style.
    #[inline]
    pub fn is_bermudan(&self) -> bool {
        matches!(self, ExerciseStyle::Bermudan { .. })
    }

    /// Returns whether this is an Asian exercise style.
    #[inline]
    pub fn is_asian(&self) -> bool {
        matches!(self, ExerciseStyle::Asian { .. })
    }

    /// Returns whether this is an early-exercise style (American or Bermudan).
    #[inline]
    pub fn allows_early_exercise(&self) -> bool {
        matches!(
            self,
            ExerciseStyle::American | ExerciseStyle::Bermudan { .. }
        )
    }

    /// Returns whether this is a path-dependent style (Asian).
    #[inline]
    pub fn is_path_dependent(&self) -> bool {
        matches!(self, ExerciseStyle::Asian { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_european_construction() {
        let style: ExerciseStyle<f64> = ExerciseStyle::european();
        assert!(style.is_european());
        assert!(!style.is_american());
        assert!(!style.is_bermudan());
        assert!(!style.is_asian());
    }

    #[test]
    fn test_american_construction() {
        let style: ExerciseStyle<f64> = ExerciseStyle::american();
        assert!(style.is_american());
        assert!(!style.is_european());
        assert!(!style.is_bermudan());
        assert!(!style.is_asian());
    }

    #[test]
    fn test_bermudan_construction() {
        let dates = vec![0.25, 0.5, 0.75, 1.0];
        let style = ExerciseStyle::bermudan(dates.clone());
        assert!(style.is_bermudan());
        assert!(!style.is_european());
        assert!(!style.is_american());
        assert!(!style.is_asian());

        if let ExerciseStyle::Bermudan { exercise_dates } = style {
            assert_eq!(exercise_dates, dates);
        } else {
            panic!("Expected Bermudan variant");
        }
    }

    #[test]
    fn test_asian_construction() {
        let style = ExerciseStyle::asian(0.0_f64, 1.0, 12);
        assert!(style.is_asian());
        assert!(!style.is_european());
        assert!(!style.is_american());
        assert!(!style.is_bermudan());

        if let ExerciseStyle::Asian {
            averaging_start,
            averaging_end,
            num_observations,
        } = style
        {
            assert_eq!(averaging_start, 0.0);
            assert_eq!(averaging_end, 1.0);
            assert_eq!(num_observations, 12);
        } else {
            panic!("Expected Asian variant");
        }
    }

    #[test]
    fn test_allows_early_exercise() {
        let european: ExerciseStyle<f64> = ExerciseStyle::european();
        let american: ExerciseStyle<f64> = ExerciseStyle::american();
        let bermudan = ExerciseStyle::bermudan(vec![0.5_f64]);
        let asian = ExerciseStyle::asian(0.0_f64, 1.0, 12);

        assert!(!european.allows_early_exercise());
        assert!(american.allows_early_exercise());
        assert!(bermudan.allows_early_exercise());
        assert!(!asian.allows_early_exercise());
    }

    #[test]
    fn test_is_path_dependent() {
        let european: ExerciseStyle<f64> = ExerciseStyle::european();
        let american: ExerciseStyle<f64> = ExerciseStyle::american();
        let bermudan = ExerciseStyle::bermudan(vec![0.5_f64]);
        let asian = ExerciseStyle::asian(0.0_f64, 1.0, 12);

        assert!(!european.is_path_dependent());
        assert!(!american.is_path_dependent());
        assert!(!bermudan.is_path_dependent());
        assert!(asian.is_path_dependent());
    }

    #[test]
    fn test_f32_compatibility() {
        let style: ExerciseStyle<f32> = ExerciseStyle::asian(0.0, 1.0, 12);
        assert!(style.is_asian());
    }

    #[test]
    fn test_clone_and_equality() {
        let style1 = ExerciseStyle::bermudan(vec![0.5_f64, 1.0]);
        let style2 = style1.clone();
        assert_eq!(style1, style2);
    }

    #[test]
    fn test_debug() {
        let style: ExerciseStyle<f64> = ExerciseStyle::European;
        assert_eq!(format!("{:?}", style), "European");

        let bermudan = ExerciseStyle::bermudan(vec![0.5_f64]);
        let debug_str = format!("{:?}", bermudan);
        assert!(debug_str.contains("Bermudan"));
        assert!(debug_str.contains("exercise_dates"));
    }

    // AD compatibility test with Dual64
    #[test]
    fn test_dual64_compatibility() {
        use num_dual::Dual64;

        let style = ExerciseStyle::asian(Dual64::new(0.0, 0.0), Dual64::new(1.0, 0.0), 12);

        if let ExerciseStyle::Asian {
            averaging_start,
            averaging_end,
            num_observations,
        } = style
        {
            assert_eq!(averaging_start.re, 0.0);
            assert_eq!(averaging_end.re, 1.0);
            assert_eq!(num_observations, 12);
        } else {
            panic!("Expected Asian variant");
        }
    }
}
