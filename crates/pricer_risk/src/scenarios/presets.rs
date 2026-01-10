//! Preset scenarios for common stress tests.
//!
//! Provides ready-to-use scenarios for typical risk analysis:
//! - Parallel rate shifts (+1bp, +10bp, +100bp)
//! - Twist scenarios (steepening/flattening)
//! - Butterfly scenarios (curvature)

use super::shifts::{BumpScenario, RiskFactorShift, Scenario};
use pricer_core::traits::risk::ShiftType;
use pricer_core::traits::Float;

/// Types of preset scenarios.
///
/// # Requirements
///
/// - Requirements: 10.5
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PresetScenarioType {
    /// Interest rate +1bp parallel shift
    RateUp1bp,
    /// Interest rate +10bp parallel shift
    RateUp10bp,
    /// Interest rate +100bp parallel shift
    RateUp100bp,
    /// Interest rate -1bp parallel shift
    RateDown1bp,
    /// Interest rate -10bp parallel shift
    RateDown10bp,
    /// Interest rate -100bp parallel shift
    RateDown100bp,
    /// Curve steepening (short rates down, long rates up)
    CurveSteepen,
    /// Curve flattening (short rates up, long rates down)
    CurveFlatten,
    /// Butterfly (middle rates spike)
    Butterfly,
    /// Credit spread +50bp
    CreditWiden50bp,
    /// Credit spread +100bp
    CreditWiden100bp,
    /// Equity -10%
    EquityDown10Pct,
    /// Equity -20%
    EquityDown20Pct,
    /// FX -10% (base currency weakens)
    FxDown10Pct,
    /// Volatility +5 vega points
    VolUp5Pts,
}

impl PresetScenarioType {
    /// Get all rate scenarios.
    pub fn rate_scenarios() -> Vec<Self> {
        vec![
            Self::RateUp1bp,
            Self::RateUp10bp,
            Self::RateUp100bp,
            Self::RateDown1bp,
            Self::RateDown10bp,
            Self::RateDown100bp,
        ]
    }

    /// Get all curve scenarios (twist, butterfly).
    pub fn curve_scenarios() -> Vec<Self> {
        vec![Self::CurveSteepen, Self::CurveFlatten, Self::Butterfly]
    }

    /// Get all stress scenarios.
    pub fn stress_scenarios() -> Vec<Self> {
        vec![
            Self::RateUp100bp,
            Self::RateDown100bp,
            Self::CreditWiden100bp,
            Self::EquityDown20Pct,
            Self::FxDown10Pct,
        ]
    }

    /// Get human-readable name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::RateUp1bp => "IR +1bp",
            Self::RateUp10bp => "IR +10bp",
            Self::RateUp100bp => "IR +100bp",
            Self::RateDown1bp => "IR -1bp",
            Self::RateDown10bp => "IR -10bp",
            Self::RateDown100bp => "IR -100bp",
            Self::CurveSteepen => "Curve Steepen",
            Self::CurveFlatten => "Curve Flatten",
            Self::Butterfly => "Butterfly",
            Self::CreditWiden50bp => "Credit +50bp",
            Self::CreditWiden100bp => "Credit +100bp",
            Self::EquityDown10Pct => "Equity -10%",
            Self::EquityDown20Pct => "Equity -20%",
            Self::FxDown10Pct => "FX -10%",
            Self::VolUp5Pts => "Vol +5pts",
        }
    }

    /// Get description.
    pub fn description(&self) -> &'static str {
        match self {
            Self::RateUp1bp => "Parallel interest rate shift +1 basis point",
            Self::RateUp10bp => "Parallel interest rate shift +10 basis points",
            Self::RateUp100bp => "Parallel interest rate shift +100 basis points",
            Self::RateDown1bp => "Parallel interest rate shift -1 basis point",
            Self::RateDown10bp => "Parallel interest rate shift -10 basis points",
            Self::RateDown100bp => "Parallel interest rate shift -100 basis points",
            Self::CurveSteepen => "Short rates -25bp, long rates +25bp",
            Self::CurveFlatten => "Short rates +25bp, long rates -25bp",
            Self::Butterfly => "Wings -10bp, belly +20bp",
            Self::CreditWiden50bp => "Credit spread widening +50 basis points",
            Self::CreditWiden100bp => "Credit spread widening +100 basis points",
            Self::EquityDown10Pct => "Equity prices decline 10%",
            Self::EquityDown20Pct => "Equity prices decline 20%",
            Self::FxDown10Pct => "Base currency weakens 10%",
            Self::VolUp5Pts => "Implied volatility increases 5 percentage points",
        }
    }
}

/// Generator for preset scenarios.
///
/// # Requirements
///
/// - Requirements: 10.5
#[derive(Clone, Debug, Default)]
pub struct PresetScenario<T: Float> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Float> PresetScenario<T> {
    /// Create a new preset scenario generator.
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }

    /// Generate a scenario from a preset type.
    pub fn generate(&self, preset: PresetScenarioType) -> Scenario<T> {
        let bumps = self.create_bumps(preset);
        Scenario::new(preset.name(), preset.description(), bumps)
    }

    /// Generate multiple scenarios from preset types.
    pub fn generate_all(&self, presets: &[PresetScenarioType]) -> Vec<Scenario<T>> {
        presets.iter().map(|p| self.generate(*p)).collect()
    }

    /// Generate all rate scenarios.
    pub fn rate_scenarios(&self) -> Vec<Scenario<T>> {
        self.generate_all(&PresetScenarioType::rate_scenarios())
    }

    /// Generate all curve scenarios.
    pub fn curve_scenarios(&self) -> Vec<Scenario<T>> {
        self.generate_all(&PresetScenarioType::curve_scenarios())
    }

    /// Generate all stress scenarios.
    pub fn stress_scenarios(&self) -> Vec<Scenario<T>> {
        self.generate_all(&PresetScenarioType::stress_scenarios())
    }

    /// Create bump scenario for a preset.
    fn create_bumps(&self, preset: PresetScenarioType) -> BumpScenario<T> {
        match preset {
            PresetScenarioType::RateUp1bp => {
                self.parallel_rate_shift(T::from(0.0001).unwrap_or(T::zero()))
            }
            PresetScenarioType::RateUp10bp => {
                self.parallel_rate_shift(T::from(0.001).unwrap_or(T::zero()))
            }
            PresetScenarioType::RateUp100bp => {
                self.parallel_rate_shift(T::from(0.01).unwrap_or(T::zero()))
            }
            PresetScenarioType::RateDown1bp => {
                self.parallel_rate_shift(T::from(-0.0001).unwrap_or(T::zero()))
            }
            PresetScenarioType::RateDown10bp => {
                self.parallel_rate_shift(T::from(-0.001).unwrap_or(T::zero()))
            }
            PresetScenarioType::RateDown100bp => {
                self.parallel_rate_shift(T::from(-0.01).unwrap_or(T::zero()))
            }
            PresetScenarioType::CurveSteepen => self.curve_twist(
                T::from(-0.0025).unwrap_or(T::zero()),
                T::from(0.0025).unwrap_or(T::zero()),
            ),
            PresetScenarioType::CurveFlatten => self.curve_twist(
                T::from(0.0025).unwrap_or(T::zero()),
                T::from(-0.0025).unwrap_or(T::zero()),
            ),
            PresetScenarioType::Butterfly => self.curve_butterfly(
                T::from(-0.001).unwrap_or(T::zero()),
                T::from(0.002).unwrap_or(T::zero()),
            ),
            PresetScenarioType::CreditWiden50bp => {
                self.credit_shift(T::from(0.005).unwrap_or(T::zero()))
            }
            PresetScenarioType::CreditWiden100bp => {
                self.credit_shift(T::from(0.01).unwrap_or(T::zero()))
            }
            PresetScenarioType::EquityDown10Pct => {
                self.equity_shift(T::from(-0.10).unwrap_or(T::zero()))
            }
            PresetScenarioType::EquityDown20Pct => {
                self.equity_shift(T::from(-0.20).unwrap_or(T::zero()))
            }
            PresetScenarioType::FxDown10Pct => self.fx_shift(T::from(-0.10).unwrap_or(T::zero())),
            PresetScenarioType::VolUp5Pts => {
                self.volatility_shift(T::from(0.05).unwrap_or(T::zero()))
            }
        }
    }

    fn parallel_rate_shift(&self, amount: T) -> BumpScenario<T> {
        BumpScenario::new().with_shift(RiskFactorShift::rate_parallel("*", amount))
    }

    fn curve_twist(&self, short_shift: T, long_shift: T) -> BumpScenario<T> {
        BumpScenario::new()
            .with_shift(RiskFactorShift::new(
                pricer_core::traits::risk::RiskFactorType::InterestRate,
                "*.SHORT.*",
                ShiftType::Parallel(short_shift),
            ))
            .with_shift(RiskFactorShift::new(
                pricer_core::traits::risk::RiskFactorType::InterestRate,
                "*.LONG.*",
                ShiftType::Parallel(long_shift),
            ))
    }

    fn curve_butterfly(&self, wing_shift: T, belly_shift: T) -> BumpScenario<T> {
        BumpScenario::new()
            .with_shift(RiskFactorShift::new(
                pricer_core::traits::risk::RiskFactorType::InterestRate,
                "*.SHORT.*",
                ShiftType::Parallel(wing_shift),
            ))
            .with_shift(RiskFactorShift::new(
                pricer_core::traits::risk::RiskFactorType::InterestRate,
                "*.MID.*",
                ShiftType::Parallel(belly_shift),
            ))
            .with_shift(RiskFactorShift::new(
                pricer_core::traits::risk::RiskFactorType::InterestRate,
                "*.LONG.*",
                ShiftType::Parallel(wing_shift),
            ))
    }

    fn credit_shift(&self, amount: T) -> BumpScenario<T> {
        BumpScenario::new().with_shift(RiskFactorShift::credit_spread("*", amount))
    }

    fn equity_shift(&self, percentage: T) -> BumpScenario<T> {
        BumpScenario::new().with_shift(RiskFactorShift::equity_relative("*", percentage))
    }

    fn fx_shift(&self, percentage: T) -> BumpScenario<T> {
        BumpScenario::new().with_shift(RiskFactorShift::fx_relative("*", percentage))
    }

    fn volatility_shift(&self, amount: T) -> BumpScenario<T> {
        BumpScenario::new().with_shift(RiskFactorShift::volatility_shift("*", amount))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ================================================================
    // Task 11.5: Preset scenarios tests (TDD)
    // ================================================================

    #[test]
    fn test_preset_scenario_type_name() {
        assert_eq!(PresetScenarioType::RateUp1bp.name(), "IR +1bp");
        assert_eq!(PresetScenarioType::RateUp100bp.name(), "IR +100bp");
        assert_eq!(PresetScenarioType::CurveSteepen.name(), "Curve Steepen");
    }

    #[test]
    fn test_preset_scenario_type_description() {
        let desc = PresetScenarioType::RateUp1bp.description();
        assert!(desc.contains("1 basis point"));
    }

    #[test]
    fn test_preset_scenario_type_rate_scenarios() {
        let scenarios = PresetScenarioType::rate_scenarios();
        assert_eq!(scenarios.len(), 6);
        assert!(scenarios.contains(&PresetScenarioType::RateUp1bp));
        assert!(scenarios.contains(&PresetScenarioType::RateDown100bp));
    }

    #[test]
    fn test_preset_scenario_type_curve_scenarios() {
        let scenarios = PresetScenarioType::curve_scenarios();
        assert_eq!(scenarios.len(), 3);
        assert!(scenarios.contains(&PresetScenarioType::CurveSteepen));
        assert!(scenarios.contains(&PresetScenarioType::Butterfly));
    }

    #[test]
    fn test_preset_scenario_type_stress_scenarios() {
        let scenarios = PresetScenarioType::stress_scenarios();
        assert_eq!(scenarios.len(), 5);
        assert!(scenarios.contains(&PresetScenarioType::RateUp100bp));
        assert!(scenarios.contains(&PresetScenarioType::EquityDown20Pct));
    }

    #[test]
    fn test_preset_scenario_generate() {
        let generator = PresetScenario::<f64>::new();
        let scenario = generator.generate(PresetScenarioType::RateUp1bp);

        assert_eq!(scenario.name(), "IR +1bp");
        assert!(!scenario.bumps().is_empty());
    }

    #[test]
    fn test_preset_scenario_generate_all() {
        let generator = PresetScenario::<f64>::new();
        let scenarios = generator.generate_all(&[
            PresetScenarioType::RateUp1bp,
            PresetScenarioType::RateUp10bp,
        ]);

        assert_eq!(scenarios.len(), 2);
    }

    #[test]
    fn test_preset_scenario_rate_scenarios() {
        let generator = PresetScenario::<f64>::new();
        let scenarios = generator.rate_scenarios();

        assert_eq!(scenarios.len(), 6);
        assert_eq!(scenarios[0].name(), "IR +1bp");
    }

    #[test]
    fn test_preset_scenario_curve_scenarios() {
        let generator = PresetScenario::<f64>::new();
        let scenarios = generator.curve_scenarios();

        assert_eq!(scenarios.len(), 3);
    }

    #[test]
    fn test_preset_scenario_stress_scenarios() {
        let generator = PresetScenario::<f64>::new();
        let scenarios = generator.stress_scenarios();

        assert_eq!(scenarios.len(), 5);
    }

    #[test]
    fn test_preset_scenario_credit_widen() {
        let generator = PresetScenario::<f64>::new();
        let scenario = generator.generate(PresetScenarioType::CreditWiden50bp);

        assert_eq!(scenario.name(), "Credit +50bp");
        assert!(scenario
            .bumps()
            .has_shifts_for(pricer_core::traits::risk::RiskFactorType::Credit));
    }

    #[test]
    fn test_preset_scenario_equity_down() {
        let generator = PresetScenario::<f64>::new();
        let scenario = generator.generate(PresetScenarioType::EquityDown10Pct);

        assert_eq!(scenario.name(), "Equity -10%");
        assert!(scenario
            .bumps()
            .has_shifts_for(pricer_core::traits::risk::RiskFactorType::Equity));
    }

    #[test]
    fn test_preset_scenario_vol_up() {
        let generator = PresetScenario::<f64>::new();
        let scenario = generator.generate(PresetScenarioType::VolUp5Pts);

        assert_eq!(scenario.name(), "Vol +5pts");
        assert!(scenario
            .bumps()
            .has_shifts_for(pricer_core::traits::risk::RiskFactorType::Volatility));
    }
}
