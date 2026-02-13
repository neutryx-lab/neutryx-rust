//! Generic script product configuration for structured exotic products.
//!
//! Provides data-driven product definitions that compile into
//! `ScriptKernel` IR via `ExoticCompiler`.

use pricer_core::kernel::BarrierType;

/// High-level product type identifier.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScriptProductType {
    /// Target Accrual Redemption Forward.
    Tarf,
    /// Autocallable structured note.
    Autocallable,
    /// Accumulator Forward (quantity-based target).
    AccumulatorForward,
    /// Snowball Note (autocallable with memory coupon).
    SnowballNote,
}

impl std::fmt::Display for ScriptProductType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tarf => write!(f, "TARF"),
            Self::Autocallable => write!(f, "Autocallable"),
            Self::AccumulatorForward => write!(f, "Accumulator Forward"),
            Self::SnowballNote => write!(f, "Snowball Note"),
        }
    }
}

/// Observation schedule entry for a single observation date.
#[derive(Clone, Debug)]
pub struct ObservationSchedule {
    /// Time in years from valuation date.
    pub time: f64,
    /// Action to perform at this observation.
    pub action: ObservationAction,
}

/// Action to perform at each observation date.
#[derive(Clone, Debug)]
pub enum ObservationAction {
    /// Check autocall barrier; if triggered, pay coupon + principal and terminate.
    AutocallCheck {
        /// Autocall barrier level (absolute).
        barrier_level: f64,
        /// Coupon amount to pay on autocall.
        coupon_amount: f64,
        /// Principal to return on autocall.
        principal_return: f64,
    },
    /// TARF forward settlement at this fixing.
    TarfAccrual {
        /// Forward strike for the fixing.
        strike: f64,
        /// Notional per fixing.
        notional_per_fixing: f64,
        /// Leverage ratio on the downside (e.g., 2.0 for 2x).
        leverage_ratio: f64,
    },
    /// Unconditional coupon payment.
    CouponPayment {
        /// Coupon amount.
        coupon_amount: f64,
    },
    /// Barrier monitoring (no payment).
    BarrierMonitor {
        /// Barrier level.
        barrier_level: f64,
        /// Barrier type (Up/Down, In/Out).
        barrier_type: BarrierType,
    },
    /// Final maturity payoff evaluation.
    FinalPayoff {
        /// Strike price.
        strike: f64,
        /// True for call, false for put.
        is_call: bool,
        /// Notional amount.
        notional: f64,
    },
    /// Snowball coupon with memory feature.
    SnowballCoupon {
        /// Coupon amount per period.
        coupon_amount: f64,
        /// Barrier level for coupon activation.
        barrier_level: f64,
    },
    /// Accumulator Forward fixing.
    AccumulatorFixing {
        /// Forward strike.
        strike: f64,
        /// Quantity accumulated per fixing.
        quantity_per_fixing: f64,
    },
}

/// Target accrual configuration (for TARF products).
#[derive(Clone, Debug)]
pub struct TargetConfig {
    /// Maximum accumulated profit before early termination.
    pub target_level: f64,
    /// Whether to cap the final settlement at remaining target.
    pub cap_final_settlement: bool,
}

/// Downside protection configuration (for Autocallable products).
#[derive(Clone, Debug)]
pub struct DownsideProtection {
    /// Put barrier level (absolute).
    pub barrier_level: f64,
    /// Barrier type (typically DownIn for reverse convertible).
    pub barrier_type: BarrierType,
    /// Strike for the embedded put.
    pub put_strike: f64,
}

/// Memory coupon configuration (for Snowball products).
#[derive(Clone, Debug)]
pub struct MemoryCouponConfig {
    /// Coupon amount per period.
    pub coupon_per_period: f64,
    /// Barrier level for memory coupon activation.
    pub barrier_level: f64,
}

/// Generic script product configuration.
///
/// This struct provides a data-driven definition for structured exotic
/// products. It is compiled into a `ScriptKernel` via `ExoticCompiler`.
#[derive(Clone, Debug)]
pub struct ScriptProduct {
    /// Product type identifier.
    pub product_type: ScriptProductType,
    /// Trade identifier.
    pub trade_id: String,
    /// Underlying index ID (for spot price lookup).
    pub underlying_index: u16,
    /// Payment currency ID.
    pub currency_id: u8,
    /// Discount curve ID.
    pub discount_curve_id: u8,
    /// Notional amount.
    pub notional: f64,
    /// Observation schedule (sorted by time).
    pub observations: Vec<ObservationSchedule>,
    /// Target accrual config (for TARF/Accumulator).
    pub target: Option<TargetConfig>,
    /// Downside protection (for Autocallable).
    pub downside: Option<DownsideProtection>,
    /// Memory coupon config (for Snowball).
    pub memory_coupon: Option<MemoryCouponConfig>,
}

impl ScriptProduct {
    /// Creates a new TARF product configuration.
    pub fn tarf(
        trade_id: impl Into<String>,
        strike: f64,
        notional_per_fixing: f64,
        target_level: f64,
        fixing_times: Vec<f64>,
    ) -> Self {
        let observations = fixing_times
            .into_iter()
            .map(|t| ObservationSchedule {
                time: t,
                action: ObservationAction::TarfAccrual {
                    strike,
                    notional_per_fixing,
                    leverage_ratio: 1.0,
                },
            })
            .collect();

        Self {
            product_type: ScriptProductType::Tarf,
            trade_id: trade_id.into(),
            underlying_index: 1,
            currency_id: 0,
            discount_curve_id: 0,
            notional: notional_per_fixing,
            observations,
            target: Some(TargetConfig {
                target_level,
                cap_final_settlement: true,
            }),
            downside: None,
            memory_coupon: None,
        }
    }

    /// Creates a new Autocallable product configuration.
    pub fn autocallable(
        trade_id: impl Into<String>,
        notional: f64,
        autocall_barrier: f64,
        coupon_rate: f64,
        observation_times: Vec<f64>,
        ki_barrier: f64,
        put_strike: f64,
    ) -> Self {
        let coupon_amount = notional * coupon_rate;

        let observations: Vec<ObservationSchedule> = observation_times
            .iter()
            .map(|&t| ObservationSchedule {
                time: t,
                action: ObservationAction::AutocallCheck {
                    barrier_level: autocall_barrier,
                    coupon_amount,
                    principal_return: notional,
                },
            })
            .collect();

        Self {
            product_type: ScriptProductType::Autocallable,
            trade_id: trade_id.into(),
            underlying_index: 1,
            currency_id: 0,
            discount_curve_id: 0,
            notional,
            observations,
            target: None,
            downside: Some(DownsideProtection {
                barrier_level: ki_barrier,
                barrier_type: BarrierType::DownIn,
                put_strike,
            }),
            memory_coupon: None,
        }
    }

    /// Creates a new Accumulator Forward product configuration.
    pub fn accumulator(
        trade_id: impl Into<String>,
        strike: f64,
        quantity_per_fixing: f64,
        target_quantity: f64,
        fixing_times: Vec<f64>,
    ) -> Self {
        let observations = fixing_times
            .into_iter()
            .map(|t| ObservationSchedule {
                time: t,
                action: ObservationAction::AccumulatorFixing {
                    strike,
                    quantity_per_fixing,
                },
            })
            .collect();

        Self {
            product_type: ScriptProductType::AccumulatorForward,
            trade_id: trade_id.into(),
            underlying_index: 1,
            currency_id: 0,
            discount_curve_id: 0,
            notional: quantity_per_fixing,
            observations,
            target: Some(TargetConfig {
                target_level: target_quantity,
                cap_final_settlement: true,
            }),
            downside: None,
            memory_coupon: None,
        }
    }

    /// Creates a new Snowball Note product configuration.
    pub fn snowball(
        trade_id: impl Into<String>,
        notional: f64,
        coupon_per_period: f64,
        barrier_level: f64,
        observation_times: Vec<f64>,
        put_strike: f64,
    ) -> Self {
        let mut observations: Vec<ObservationSchedule> = observation_times
            .iter()
            .map(|&t| ObservationSchedule {
                time: t,
                action: ObservationAction::SnowballCoupon {
                    coupon_amount: coupon_per_period,
                    barrier_level,
                },
            })
            .collect();

        // Add final payoff at maturity (last observation time)
        if let Some(last_time) = observation_times.last() {
            observations.push(ObservationSchedule {
                time: *last_time,
                action: ObservationAction::FinalPayoff {
                    strike: put_strike,
                    is_call: false,
                    notional,
                },
            });
        }

        Self {
            product_type: ScriptProductType::SnowballNote,
            trade_id: trade_id.into(),
            underlying_index: 1,
            currency_id: 0,
            discount_curve_id: 0,
            notional,
            observations,
            target: None,
            downside: None,
            memory_coupon: Some(MemoryCouponConfig {
                coupon_per_period,
                barrier_level,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tarf_creation() {
        let product = ScriptProduct::tarf(
            "TARF001",
            100.0,
            1_000_000.0,
            50_000.0,
            vec![0.25, 0.5, 0.75, 1.0],
        );
        assert_eq!(product.product_type, ScriptProductType::Tarf);
        assert_eq!(product.observations.len(), 4);
        assert!(product.target.is_some());
        assert!(product.downside.is_none());
    }

    #[test]
    fn test_autocallable_creation() {
        let product = ScriptProduct::autocallable(
            "AUTO001",
            1_000_000.0,
            105.0,
            0.10,
            vec![0.25, 0.5, 0.75, 1.0],
            70.0,
            100.0,
        );
        assert_eq!(product.product_type, ScriptProductType::Autocallable);
        assert_eq!(product.observations.len(), 4);
        assert!(product.target.is_none());
        assert!(product.downside.is_some());
    }

    #[test]
    fn test_accumulator_creation() {
        let product = ScriptProduct::accumulator(
            "ACCUM001",
            100.0,
            1_000.0,
            50_000.0,
            vec![0.25, 0.5, 0.75, 1.0],
        );
        assert_eq!(product.product_type, ScriptProductType::AccumulatorForward);
        assert_eq!(product.observations.len(), 4);
        assert!(product.target.is_some());
        assert!(product.downside.is_none());
        assert!(product.memory_coupon.is_none());
    }

    #[test]
    fn test_snowball_creation() {
        let product = ScriptProduct::snowball(
            "SNOW001",
            1_000_000.0,
            10_000.0,
            105.0,
            vec![0.25, 0.5, 0.75, 1.0],
            90.0,
        );
        assert_eq!(product.product_type, ScriptProductType::SnowballNote);
        // 4 coupon observations + 1 final payoff
        assert_eq!(product.observations.len(), 5);
        assert!(product.target.is_none());
        assert!(product.downside.is_none());
        assert!(product.memory_coupon.is_some());
    }

    #[test]
    fn test_script_product_type_display() {
        assert_eq!(ScriptProductType::Tarf.to_string(), "TARF");
        assert_eq!(ScriptProductType::Autocallable.to_string(), "Autocallable");
        assert_eq!(
            ScriptProductType::AccumulatorForward.to_string(),
            "Accumulator Forward"
        );
        assert_eq!(ScriptProductType::SnowballNote.to_string(), "Snowball Note");
    }
}
