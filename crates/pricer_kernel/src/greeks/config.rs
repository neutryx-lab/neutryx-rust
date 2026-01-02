//! Greeks calculation configuration.
//!
//! Provides [`GreeksConfig`] for configuring bump widths and calculation modes,
//! and [`GreeksMode`] for selecting between different calculation methods.

/// Calculation mode for Greeks computation.
///
/// Determines which method is used to compute Greeks sensitivities.
///
/// # Variants
///
/// * `BumpRevalue` - Finite differences (bump-and-revalue)
/// * `NumDual` - Forward-mode AD using num-dual library
/// * `EnzymeAAD` - Enzyme LLVM-level AAD (requires `enzyme-ad` feature)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum GreeksMode {
    /// Bump-and-revalue using finite differences.
    ///
    /// Uses central differences for first-order Greeks and
    /// three-point formula for second-order Greeks.
    /// This is the default mode and works with all payoff types.
    #[default]
    BumpRevalue,

    /// Forward-mode AD using num-dual library.
    ///
    /// Uses dual numbers for first-order Greeks and
    /// hyperdual numbers for second-order Greeks.
    /// Requires the `num-dual-mode` feature.
    NumDual,

    /// Enzyme LLVM-level automatic differentiation.
    ///
    /// Uses Enzyme's reverse-mode AD for efficient computation
    /// of multiple Greeks simultaneously.
    /// Requires the `enzyme-ad` feature and nightly Rust.
    #[cfg(feature = "enzyme-ad")]
    EnzymeAAD,
}

/// Configuration for Greeks calculation.
///
/// Controls bump widths for finite differences and verification tolerances.
/// Use the builder pattern via [`GreeksConfig::builder()`] for construction.
///
/// # Default Values
///
/// | Parameter | Default | Description |
/// |-----------|---------|-------------|
/// | `spot_bump_relative` | 0.01 (1%) | Relative bump for spot price |
/// | `vol_bump_absolute` | 0.01 | Absolute bump for volatility |
/// | `time_bump_years` | 1/252 | Time bump in years (1 trading day) |
/// | `rate_bump_absolute` | 0.01 | Absolute bump for interest rate |
/// | `verification_tolerance` | 1e-6 | Tolerance for mode comparison |
///
/// # Examples
///
/// ```rust
/// use pricer_kernel::greeks::{GreeksConfig, GreeksMode};
///
/// // Use defaults
/// let config = GreeksConfig::default();
///
/// // Use builder for custom values
/// let config = GreeksConfig::builder()
///     .spot_bump_relative(0.005)  // 0.5% bump
///     .vol_bump_absolute(0.01)
///     .mode(GreeksMode::BumpRevalue)
///     .build()
///     .unwrap();
/// ```
#[derive(Clone, Debug)]
pub struct GreeksConfig {
    /// Calculation mode (BumpRevalue, NumDual, or EnzymeAAD).
    pub mode: GreeksMode,

    /// Relative bump for spot price (default: 0.01 = 1%).
    ///
    /// The actual bump is `spot_bump_relative * spot_price`.
    pub spot_bump_relative: f64,

    /// Absolute bump for volatility (default: 0.01 = 1 vol point).
    pub vol_bump_absolute: f64,

    /// Time bump in years (default: 1/252 ≈ 1 trading day).
    pub time_bump_years: f64,

    /// Absolute bump for interest rate (default: 0.01 = 1%).
    pub rate_bump_absolute: f64,

    /// Tolerance for verification between calculation modes (default: 1e-6).
    pub verification_tolerance: f64,
}

impl Default for GreeksConfig {
    fn default() -> Self {
        Self {
            mode: GreeksMode::default(),
            spot_bump_relative: 0.01,
            vol_bump_absolute: 0.01,
            time_bump_years: 1.0 / 252.0,
            rate_bump_absolute: 0.01,
            verification_tolerance: 1e-6,
        }
    }
}

impl GreeksConfig {
    /// Creates a new builder for constructing a `GreeksConfig`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pricer_kernel::greeks::GreeksConfig;
    ///
    /// let config = GreeksConfig::builder()
    ///     .spot_bump_relative(0.02)
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn builder() -> GreeksConfigBuilder {
        GreeksConfigBuilder::default()
    }

    /// Validates the configuration.
    ///
    /// Returns an error if any parameter is out of valid range.
    pub fn validate(&self) -> Result<(), GreeksConfigError> {
        // Spot bump must be positive and reasonable (0 < bump <= 100%)
        if self.spot_bump_relative <= 0.0 {
            return Err(GreeksConfigError::InvalidSpotBump(
                "spot_bump_relative must be positive".to_string(),
            ));
        }
        if self.spot_bump_relative > 1.0 {
            return Err(GreeksConfigError::InvalidSpotBump(
                "spot_bump_relative must be <= 1.0 (100%)".to_string(),
            ));
        }

        // Vol bump must be positive
        if self.vol_bump_absolute <= 0.0 {
            return Err(GreeksConfigError::InvalidVolBump(
                "vol_bump_absolute must be positive".to_string(),
            ));
        }
        if self.vol_bump_absolute > 0.5 {
            return Err(GreeksConfigError::InvalidVolBump(
                "vol_bump_absolute must be <= 0.5 (50 vol points)".to_string(),
            ));
        }

        // Time bump must be positive
        if self.time_bump_years <= 0.0 {
            return Err(GreeksConfigError::InvalidTimeBump(
                "time_bump_years must be positive".to_string(),
            ));
        }
        if self.time_bump_years > 1.0 {
            return Err(GreeksConfigError::InvalidTimeBump(
                "time_bump_years must be <= 1.0 (1 year)".to_string(),
            ));
        }

        // Rate bump must be positive
        if self.rate_bump_absolute <= 0.0 {
            return Err(GreeksConfigError::InvalidRateBump(
                "rate_bump_absolute must be positive".to_string(),
            ));
        }
        if self.rate_bump_absolute > 0.1 {
            return Err(GreeksConfigError::InvalidRateBump(
                "rate_bump_absolute must be <= 0.1 (10%)".to_string(),
            ));
        }

        // Verification tolerance must be positive
        if self.verification_tolerance <= 0.0 {
            return Err(GreeksConfigError::InvalidTolerance(
                "verification_tolerance must be positive".to_string(),
            ));
        }

        Ok(())
    }

    /// Computes the absolute spot bump for a given spot price.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pricer_kernel::greeks::GreeksConfig;
    ///
    /// let config = GreeksConfig::default(); // 1% bump
    /// let bump = config.compute_spot_bump(100.0);
    /// assert!((bump - 1.0).abs() < 1e-10); // 1% of 100 = 1.0
    /// ```
    #[inline]
    pub fn compute_spot_bump(&self, spot: f64) -> f64 {
        (self.spot_bump_relative * spot).max(1e-8)
    }
}

/// Builder for [`GreeksConfig`].
///
/// Provides a fluent interface for constructing a `GreeksConfig` with
/// custom parameters.
#[derive(Debug, Default)]
pub struct GreeksConfigBuilder {
    mode: Option<GreeksMode>,
    spot_bump_relative: Option<f64>,
    vol_bump_absolute: Option<f64>,
    time_bump_years: Option<f64>,
    rate_bump_absolute: Option<f64>,
    verification_tolerance: Option<f64>,
}

impl GreeksConfigBuilder {
    /// Sets the calculation mode.
    pub fn mode(mut self, mode: GreeksMode) -> Self {
        self.mode = Some(mode);
        self
    }

    /// Sets the relative spot bump (default: 0.01 = 1%).
    pub fn spot_bump_relative(mut self, bump: f64) -> Self {
        self.spot_bump_relative = Some(bump);
        self
    }

    /// Sets the absolute volatility bump (default: 0.01).
    pub fn vol_bump_absolute(mut self, bump: f64) -> Self {
        self.vol_bump_absolute = Some(bump);
        self
    }

    /// Sets the time bump in years (default: 1/252).
    pub fn time_bump_years(mut self, bump: f64) -> Self {
        self.time_bump_years = Some(bump);
        self
    }

    /// Sets the absolute rate bump (default: 0.01).
    pub fn rate_bump_absolute(mut self, bump: f64) -> Self {
        self.rate_bump_absolute = Some(bump);
        self
    }

    /// Sets the verification tolerance (default: 1e-6).
    pub fn verification_tolerance(mut self, tolerance: f64) -> Self {
        self.verification_tolerance = Some(tolerance);
        self
    }

    /// Builds the configuration, validating all parameters.
    ///
    /// # Errors
    ///
    /// Returns [`GreeksConfigError`] if any parameter is invalid.
    pub fn build(self) -> Result<GreeksConfig, GreeksConfigError> {
        let config = GreeksConfig {
            mode: self.mode.unwrap_or_default(),
            spot_bump_relative: self.spot_bump_relative.unwrap_or(0.01),
            vol_bump_absolute: self.vol_bump_absolute.unwrap_or(0.01),
            time_bump_years: self.time_bump_years.unwrap_or(1.0 / 252.0),
            rate_bump_absolute: self.rate_bump_absolute.unwrap_or(0.01),
            verification_tolerance: self.verification_tolerance.unwrap_or(1e-6),
        };

        config.validate()?;
        Ok(config)
    }
}

/// Error type for [`GreeksConfig`] validation.
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::enum_variant_names)]
pub enum GreeksConfigError {
    /// Invalid spot bump value.
    InvalidSpotBump(String),
    /// Invalid volatility bump value.
    InvalidVolBump(String),
    /// Invalid time bump value.
    InvalidTimeBump(String),
    /// Invalid rate bump value.
    InvalidRateBump(String),
    /// Invalid verification tolerance.
    InvalidTolerance(String),
}

impl std::fmt::Display for GreeksConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSpotBump(msg) => write!(f, "Invalid spot bump: {}", msg),
            Self::InvalidVolBump(msg) => write!(f, "Invalid vol bump: {}", msg),
            Self::InvalidTimeBump(msg) => write!(f, "Invalid time bump: {}", msg),
            Self::InvalidRateBump(msg) => write!(f, "Invalid rate bump: {}", msg),
            Self::InvalidTolerance(msg) => write!(f, "Invalid tolerance: {}", msg),
        }
    }
}

impl std::error::Error for GreeksConfigError {}
