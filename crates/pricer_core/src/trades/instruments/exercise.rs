//! Option exercise style definitions.

use num_traits::Float;

/// Option exercise style.
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
    #[inline]
    pub fn bermudan(exercise_dates: Vec<T>) -> Self {
        ExerciseStyle::Bermudan { exercise_dates }
    }

    /// Creates a new Asian exercise style with averaging parameters.
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
