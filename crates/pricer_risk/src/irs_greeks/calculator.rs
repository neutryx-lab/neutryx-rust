//! IRS Greeks calculator implementation.
//!
//! Provides AAD and bump-and-revalue Greeks calculation for Interest Rate
//! Swaps.

use std::marker::PhantomData;

use num_traits::Float;

// TODO: l1l2-integration feature disabled pending refactoring to use
// infra_master::trade::Trade
#[allow(unused_imports)]
use super::{
    config::IrsGreeksConfig,
    error::IrsGreeksError,
    result::{IrsDeltaResult, IrsGreeksResult},
};
#[allow(unused_imports)]
use crate::greeks::GreeksMode;

/// IRS Greeks calculator.
///
/// Computes NPV, DV01, and tenor Deltas for Interest Rate Swaps using
/// either AAD (Adjoint Algorithmic Differentiation) or bump-and-revalue
/// methods.
///
/// # Type Parameters
///
/// * `T` - Floating-point type implementing `Float` (e.g., `f64`, `Dual64`)
///
/// # Examples
///
/// ```rust,ignore
/// use pricer_risk::irs_greeks::{IrsGreeksCalculator, IrsGreeksConfig};
///
/// let config = IrsGreeksConfig::default();
/// let calculator = IrsGreeksCalculator::<f64>::new(config);
///
/// let npv = calculator.compute_npv(&swap, &curves, valuation_date)?;
/// ```
pub struct IrsGreeksCalculator<T: Float> {
    config: IrsGreeksConfig,
    _phantom: PhantomData<T>,
}

impl<T: Float> IrsGreeksCalculator<T> {
    /// Creates a new IRS Greeks calculator with the given configuration.
    pub fn new(config: IrsGreeksConfig) -> Self {
        Self {
            config,
            _phantom: PhantomData,
        }
    }

    /// Returns a reference to the configuration.
    pub fn config(&self) -> &IrsGreeksConfig { &self.config }
}

// TODO: l1l2-integration feature implementation pending refactoring
// The IRS Greeks calculation methods are disabled until infra_master::trade::Trade
// refactoring is complete. See pricer_pricing::irs_greeks for reference.
