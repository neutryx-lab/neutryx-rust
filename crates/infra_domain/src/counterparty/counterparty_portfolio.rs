//! CounterpartyPortfolio hierarchy structure.
//!
//! This module provides the complete hierarchy for XVA calculation:
//! CounterpartyPortfolio -> IsdaMasterAgreement -> VariationMarginAgreement ->
//! Trade
//!
//! # Architecture
//!
//! ```text
//! CounterpartyPortfolio
//! ├── counterparty_id
//! ├── isda_agreements: Vec<IsdaMasterAgreement>
//! │   ├── variation_margin_agreements: Vec<VariationMarginAgreement>
//! │   │   └── trade_ids: Vec<TradeId>
//! │   └── non_csa_trade_ids: Vec<TradeId>
//! └── non_nettable_trades: NonNettableTrades
//! ```

use std::collections::{BTreeSet, HashSet};

use super::{
    CounterPartyError, CounterPartyId, EligibleCollateral, IsdaAgreementId,
    VariationMarginAgreementId,
};
use crate::{ids::TradeId, market::Currency, time::Date};

// ============================================================================
// IsdaPaymentMethod
// ============================================================================

/// ISDA payment method for collateral.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum IsdaPaymentMethod {
    /// Full bilateral exchange.
    #[default]
    Full,
    /// Limited recourse.
    Limited,
    /// One-way posting to counterparty.
    OnewayToCpty,
    /// One-way posting to self.
    OnewayToSelf,
}

// ============================================================================
// CollateralCallFrequency
// ============================================================================

/// Collateral call frequency for VM agreements.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CollateralCallFrequency {
    /// Daily margin calls (standard).
    #[default]
    Daily,
    /// Weekly margin calls.
    Weekly,
    /// Bi-weekly margin calls.
    Biweekly,
    /// Monthly margin calls.
    Monthly,
}

impl CollateralCallFrequency {
    /// Returns the default MPOR (Margin Period of Risk) in business days.
    #[must_use]
    pub fn default_mpor_days(&self) -> u32 {
        match self {
            CollateralCallFrequency::Daily => 10,
            CollateralCallFrequency::Weekly => 10,
            CollateralCallFrequency::Biweekly => 14,
            CollateralCallFrequency::Monthly => 20,
        }
    }
}

// ============================================================================
// IndependentAmountConfig
// ============================================================================

/// Dynamic Independent Amount configuration.
///
/// Supports asymmetric IA calculation:
/// - Counterparty IA: ia_cpty + k_cpty * max(PV, 0)
/// - Self IA: ia_self + k_self * min(PV, 0)
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct IndependentAmountConfig {
    /// Base IA for counterparty (positive).
    pub ia_cpty: f64,
    /// Coefficient for counterparty (multiplied by max(PV, 0)).
    pub k_cpty: f64,
    /// Base IA for self (negative).
    pub ia_self: f64,
    /// Coefficient for self (multiplied by min(PV, 0)).
    pub k_self: f64,
}

impl IndependentAmountConfig {
    /// Calculates the independent amount given a portfolio PV.
    #[must_use]
    pub fn calculate(&self, pv: f64) -> f64 {
        let cpty_ia = self.ia_cpty + self.k_cpty * pv.max(0.0);
        let self_ia = self.ia_self + self.k_self * pv.min(0.0);
        cpty_ia + self_ia
    }
}

// ============================================================================
// VariationMarginAgreement
// ============================================================================

/// Variation Margin Agreement with asymmetric conditions.
///
/// Supports:
/// - Asymmetric thresholds (counterparty vs self)
/// - Asymmetric MTA (Minimum Transfer Amount)
/// - Dynamic Independent Amount
/// - Asymmetric haircuts
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct VariationMarginAgreement {
    vma_id: VariationMarginAgreementId,
    name: String,
    base_currency: Currency,
    call_frequency: CollateralCallFrequency,

    // Asymmetric thresholds
    threshold_cpty: f64,
    threshold_self: f64,

    // Asymmetric MTA
    mta_cpty: f64,
    mta_self: f64,

    // Dynamic Independent Amount
    independent_amount: IndependentAmountConfig,

    // Asymmetric haircuts
    haircut_cpty: f64,
    haircut_self: f64,

    // Collateral (reserved for future collateral management)
    #[allow(dead_code)]
    eligible_collaterals: Vec<EligibleCollateral>,
    #[allow(dead_code)]
    current_collateral_balances: Vec<f64>,

    // Trades
    trade_ids: Vec<TradeId>,

    // Pre-calculated exposure (optional)
    precalc_exposure: Option<PreCalculatedExposurePath>,
}

impl VariationMarginAgreement {
    /// Creates a new VMA builder.
    #[must_use]
    pub fn builder(
        id: impl Into<VariationMarginAgreementId>,
        name: impl Into<String>,
        base_currency: Currency,
    ) -> VariationMarginAgreementBuilder {
        VariationMarginAgreementBuilder::new(id, name, base_currency)
    }

    /// Returns the VMA ID.
    #[inline]
    #[must_use]
    pub fn vma_id(&self) -> &VariationMarginAgreementId { &self.vma_id }

    /// Returns the VMA name.
    #[inline]
    #[must_use]
    pub fn name(&self) -> &str { &self.name }

    /// Returns the base currency.
    #[inline]
    #[must_use]
    pub fn base_currency(&self) -> Currency { self.base_currency }

    /// Returns the call frequency.
    #[inline]
    #[must_use]
    pub fn call_frequency(&self) -> CollateralCallFrequency { self.call_frequency }

    /// Returns the counterparty threshold (positive).
    #[inline]
    #[must_use]
    pub fn threshold_cpty(&self) -> f64 { self.threshold_cpty }

    /// Returns the self threshold (negative).
    #[inline]
    #[must_use]
    pub fn threshold_self(&self) -> f64 { self.threshold_self }

    /// Returns the counterparty MTA.
    #[inline]
    #[must_use]
    pub fn mta_cpty(&self) -> f64 { self.mta_cpty }

    /// Returns the self MTA.
    #[inline]
    #[must_use]
    pub fn mta_self(&self) -> f64 { self.mta_self }

    /// Returns the independent amount configuration.
    #[inline]
    #[must_use]
    pub fn independent_amount(&self) -> &IndependentAmountConfig { &self.independent_amount }

    /// Returns the counterparty haircut.
    #[inline]
    #[must_use]
    pub fn haircut_cpty(&self) -> f64 { self.haircut_cpty }

    /// Returns the self haircut.
    #[inline]
    #[must_use]
    pub fn haircut_self(&self) -> f64 { self.haircut_self }

    /// Returns the trade IDs.
    #[inline]
    #[must_use]
    pub fn trade_ids(&self) -> &[TradeId] { &self.trade_ids }

    /// Returns the default MPOR based on call frequency.
    #[inline]
    #[must_use]
    pub fn mpor_days(&self) -> u32 { self.call_frequency.default_mpor_days() }

    /// Returns the pre-calculated exposure path if set.
    #[inline]
    #[must_use]
    pub fn precalc_exposure(&self) -> Option<&PreCalculatedExposurePath> {
        self.precalc_exposure.as_ref()
    }
}

// ============================================================================
// VariationMarginAgreementBuilder
// ============================================================================

/// Builder for VariationMarginAgreement.
#[derive(Clone, Debug)]
pub struct VariationMarginAgreementBuilder {
    vma_id: VariationMarginAgreementId,
    name: String,
    base_currency: Currency,
    call_frequency: CollateralCallFrequency,
    threshold_cpty: f64,
    threshold_self: f64,
    mta_cpty: f64,
    mta_self: f64,
    independent_amount: IndependentAmountConfig,
    haircut_cpty: f64,
    haircut_self: f64,
    eligible_collaterals: Vec<EligibleCollateral>,
    current_collateral_balances: Vec<f64>,
    trade_ids: Vec<TradeId>,
    precalc_exposure: Option<PreCalculatedExposurePath>,
}

impl VariationMarginAgreementBuilder {
    /// Creates a new builder.
    #[must_use]
    pub fn new(
        id: impl Into<VariationMarginAgreementId>,
        name: impl Into<String>,
        base_currency: Currency,
    ) -> Self {
        Self {
            vma_id: id.into(),
            name: name.into(),
            base_currency,
            call_frequency: CollateralCallFrequency::default(),
            threshold_cpty: 0.0,
            threshold_self: 0.0,
            mta_cpty: 0.0,
            mta_self: 0.0,
            independent_amount: IndependentAmountConfig::default(),
            haircut_cpty: 0.0,
            haircut_self: 0.0,
            eligible_collaterals: vec![EligibleCollateral::Cash],
            current_collateral_balances: Vec::new(),
            trade_ids: Vec::new(),
            precalc_exposure: None,
        }
    }

    /// Sets the call frequency.
    #[must_use]
    pub fn call_frequency(mut self, freq: CollateralCallFrequency) -> Self {
        self.call_frequency = freq;
        self
    }

    /// Sets asymmetric thresholds.
    #[must_use]
    pub fn asymmetric_threshold(mut self, cpty: f64, self_: f64) -> Self {
        self.threshold_cpty = cpty;
        self.threshold_self = self_;
        self
    }

    /// Sets asymmetric MTA.
    #[must_use]
    pub fn asymmetric_mta(mut self, cpty: f64, self_: f64) -> Self {
        self.mta_cpty = cpty;
        self.mta_self = self_;
        self
    }

    /// Sets the independent amount configuration.
    #[must_use]
    pub fn independent_amount(mut self, config: IndependentAmountConfig) -> Self {
        self.independent_amount = config;
        self
    }

    /// Sets asymmetric haircuts.
    #[must_use]
    pub fn asymmetric_haircut(mut self, cpty: f64, self_: f64) -> Self {
        self.haircut_cpty = cpty;
        self.haircut_self = self_;
        self
    }

    /// Adds a trade ID.
    #[must_use]
    pub fn add_trade(mut self, trade_id: impl Into<TradeId>) -> Self {
        let id = trade_id.into();
        if !self.trade_ids.contains(&id) {
            self.trade_ids.push(id);
        }
        self
    }

    /// Sets trade IDs.
    #[must_use]
    pub fn trade_ids(mut self, ids: impl IntoIterator<Item = impl Into<TradeId>>) -> Self {
        self.trade_ids = ids.into_iter().map(Into::into).collect();
        self
    }

    /// Sets pre-calculated exposure path.
    #[must_use]
    pub fn precalc_exposure(mut self, exposure: PreCalculatedExposurePath) -> Self {
        self.precalc_exposure = Some(exposure);
        self
    }

    /// Builds the VariationMarginAgreement.
    #[must_use]
    pub fn build(self) -> VariationMarginAgreement {
        VariationMarginAgreement {
            vma_id: self.vma_id,
            name: self.name,
            base_currency: self.base_currency,
            call_frequency: self.call_frequency,
            threshold_cpty: self.threshold_cpty,
            threshold_self: self.threshold_self,
            mta_cpty: self.mta_cpty,
            mta_self: self.mta_self,
            independent_amount: self.independent_amount,
            haircut_cpty: self.haircut_cpty,
            haircut_self: self.haircut_self,
            eligible_collaterals: self.eligible_collaterals,
            current_collateral_balances: self.current_collateral_balances,
            trade_ids: self.trade_ids,
            precalc_exposure: self.precalc_exposure,
        }
    }
}

// ============================================================================
// IsdaInitialMargin
// ============================================================================

/// ISDA-level Initial Margin configuration.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct IsdaInitialMargin {
    /// IM amount posted to counterparty.
    pub im_post: f64,
    /// IM amount received from counterparty.
    pub im_recv: f64,
    /// IM currency.
    pub im_currency: Currency,
    /// IM rate curve ID (for discounting).
    pub im_rate_curve_id: Option<String>,
}

// ============================================================================
// IsdaMasterAgreement
// ============================================================================

/// ISDA Master Agreement.
///
/// Contains one or more VariationMarginAgreements and tracks
/// trades without CSA (non_csa_trade_ids).
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct IsdaMasterAgreement {
    isda_id: IsdaAgreementId,
    name: String,
    counterparty_id: CounterPartyId,
    agreement_date: Option<Date>,
    payment_method: IsdaPaymentMethod,
    variation_margin_agreements: Vec<VariationMarginAgreement>,
    non_csa_trade_ids: Vec<TradeId>,
    initial_margin: Option<IsdaInitialMargin>,
    /// Pre-calculated exposure path for non-CSA trades (reserved for future
    /// use).
    #[allow(dead_code)]
    precalc_non_csa_exposure: Option<PreCalculatedExposurePath>,
}

impl IsdaMasterAgreement {
    /// Creates a new ISDA builder.
    #[must_use]
    pub fn builder(
        id: impl Into<IsdaAgreementId>,
        name: impl Into<String>,
        counterparty_id: impl Into<CounterPartyId>,
    ) -> IsdaMasterAgreementBuilder {
        IsdaMasterAgreementBuilder::new(id, name, counterparty_id)
    }

    /// Returns the ISDA ID.
    #[inline]
    #[must_use]
    pub fn isda_id(&self) -> &IsdaAgreementId { &self.isda_id }

    /// Returns the name.
    #[inline]
    #[must_use]
    pub fn name(&self) -> &str { &self.name }

    /// Returns the counterparty ID.
    #[inline]
    #[must_use]
    pub fn counterparty_id(&self) -> &CounterPartyId { &self.counterparty_id }

    /// Returns the agreement date.
    #[inline]
    #[must_use]
    pub fn agreement_date(&self) -> Option<Date> { self.agreement_date }

    /// Returns the payment method.
    #[inline]
    #[must_use]
    pub fn payment_method(&self) -> IsdaPaymentMethod { self.payment_method }

    /// Returns the VM agreements.
    #[inline]
    #[must_use]
    pub fn variation_margin_agreements(&self) -> &[VariationMarginAgreement] {
        &self.variation_margin_agreements
    }

    /// Returns non-CSA trade IDs.
    #[inline]
    #[must_use]
    pub fn non_csa_trade_ids(&self) -> &[TradeId] { &self.non_csa_trade_ids }

    /// Returns the initial margin configuration.
    #[inline]
    #[must_use]
    pub fn initial_margin(&self) -> Option<&IsdaInitialMargin> { self.initial_margin.as_ref() }

    /// Iterates over all trade IDs in this ISDA.
    pub fn iter_all_trades(&self) -> impl Iterator<Item = &TradeId> {
        let vma_trades = self
            .variation_margin_agreements
            .iter()
            .flat_map(|vma| vma.trade_ids.iter());
        let non_csa_trades = self.non_csa_trade_ids.iter();
        vma_trades.chain(non_csa_trades)
    }
}

// ============================================================================
// IsdaMasterAgreementBuilder
// ============================================================================

/// Builder for IsdaMasterAgreement.
#[derive(Clone, Debug)]
pub struct IsdaMasterAgreementBuilder {
    isda_id: IsdaAgreementId,
    name: String,
    counterparty_id: CounterPartyId,
    agreement_date: Option<Date>,
    payment_method: IsdaPaymentMethod,
    variation_margin_agreements: Vec<VariationMarginAgreement>,
    non_csa_trade_ids: Vec<TradeId>,
    initial_margin: Option<IsdaInitialMargin>,
    precalc_non_csa_exposure: Option<PreCalculatedExposurePath>,
}

impl IsdaMasterAgreementBuilder {
    /// Creates a new builder.
    #[must_use]
    pub fn new(
        id: impl Into<IsdaAgreementId>,
        name: impl Into<String>,
        counterparty_id: impl Into<CounterPartyId>,
    ) -> Self {
        Self {
            isda_id: id.into(),
            name: name.into(),
            counterparty_id: counterparty_id.into(),
            agreement_date: None,
            payment_method: IsdaPaymentMethod::default(),
            variation_margin_agreements: Vec::new(),
            non_csa_trade_ids: Vec::new(),
            initial_margin: None,
            precalc_non_csa_exposure: None,
        }
    }

    /// Sets the agreement date.
    #[must_use]
    pub fn agreement_date(mut self, date: Date) -> Self {
        self.agreement_date = Some(date);
        self
    }

    /// Sets the payment method.
    #[must_use]
    pub fn payment_method(mut self, method: IsdaPaymentMethod) -> Self {
        self.payment_method = method;
        self
    }

    /// Adds a VM agreement.
    #[must_use]
    pub fn add_vma(mut self, vma: VariationMarginAgreement) -> Self {
        self.variation_margin_agreements.push(vma);
        self
    }

    /// Adds a non-CSA trade ID.
    #[must_use]
    pub fn add_non_csa_trade(mut self, trade_id: impl Into<TradeId>) -> Self {
        let id = trade_id.into();
        if !self.non_csa_trade_ids.contains(&id) {
            self.non_csa_trade_ids.push(id);
        }
        self
    }

    /// Sets the initial margin.
    #[must_use]
    pub fn initial_margin(mut self, im: IsdaInitialMargin) -> Self {
        self.initial_margin = Some(im);
        self
    }

    /// Builds the IsdaMasterAgreement.
    #[must_use]
    pub fn build(self) -> IsdaMasterAgreement {
        IsdaMasterAgreement {
            isda_id: self.isda_id,
            name: self.name,
            counterparty_id: self.counterparty_id,
            agreement_date: self.agreement_date,
            payment_method: self.payment_method,
            variation_margin_agreements: self.variation_margin_agreements,
            non_csa_trade_ids: self.non_csa_trade_ids,
            initial_margin: self.initial_margin,
            precalc_non_csa_exposure: self.precalc_non_csa_exposure,
        }
    }
}

// ============================================================================
// NettingEligibility
// ============================================================================

/// Netting eligibility classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum NettingEligibility {
    /// Full netting with CSA collateral.
    FullNetting,
    /// ISDA netting only (no CSA).
    IsdaOnly,
    /// Non-nettable (gross exposure).
    NonNettable,
}

// ============================================================================
// NonNettableTrades
// ============================================================================

/// Non-nettable trades group.
///
/// Holds trades that cannot be netted with any netting agreement.
/// Exposure is calculated gross (PE + NE separately).
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NonNettableTrades {
    trade_ids: Vec<TradeId>,
    precalc_positive_exposure: Option<PreCalculatedExposurePath>,
    precalc_negative_exposure: Option<PreCalculatedExposurePath>,
}

impl NonNettableTrades {
    /// Creates a new empty non-nettable trades group.
    #[must_use]
    pub fn new() -> Self { Self::default() }

    /// Adds a trade ID.
    pub fn add_trade(&mut self, trade_id: impl Into<TradeId>) {
        let id = trade_id.into();
        if !self.trade_ids.contains(&id) {
            self.trade_ids.push(id);
        }
    }

    /// Returns the trade IDs.
    #[inline]
    #[must_use]
    pub fn trade_ids(&self) -> &[TradeId] { &self.trade_ids }

    /// Returns true if empty.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool { self.trade_ids.is_empty() }

    /// Returns the number of trades.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize { self.trade_ids.len() }

    /// Sets pre-calculated positive exposure.
    pub fn set_precalc_positive(&mut self, exposure: PreCalculatedExposurePath) {
        self.precalc_positive_exposure = Some(exposure);
    }

    /// Sets pre-calculated negative exposure.
    pub fn set_precalc_negative(&mut self, exposure: PreCalculatedExposurePath) {
        self.precalc_negative_exposure = Some(exposure);
    }
}

// ============================================================================
// PreCalculatedExposurePath
// ============================================================================

/// Pre-calculated exposure path.
///
/// Stores exposure values by date for Monte Carlo paths.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PreCalculatedExposurePath {
    /// exposure_by_date[date][path_index] = exposure value
    exposure_by_date: std::collections::BTreeMap<Date, Vec<f64>>,
    currency: Currency,
}

impl PreCalculatedExposurePath {
    /// Creates a new pre-calculated exposure path.
    #[must_use]
    pub fn new(currency: Currency) -> Self {
        Self {
            exposure_by_date: std::collections::BTreeMap::new(),
            currency,
        }
    }

    /// Adds exposure values for a date.
    pub fn add_exposure(&mut self, date: Date, exposures: Vec<f64>) {
        self.exposure_by_date.insert(date, exposures);
    }

    /// Returns exposure values at a date.
    #[must_use]
    pub fn exposure_at(&self, date: &Date) -> Option<&Vec<f64>> { self.exposure_by_date.get(date) }

    /// Returns the currency.
    #[inline]
    #[must_use]
    pub fn currency(&self) -> Currency { self.currency }

    /// Returns an iterator over dates.
    pub fn dates(&self) -> impl Iterator<Item = &Date> { self.exposure_by_date.keys() }

    /// Returns the number of dates.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize { self.exposure_by_date.len() }

    /// Returns true if empty.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool { self.exposure_by_date.is_empty() }

    /// Creates a new builder for PreCalculatedExposurePath.
    #[must_use]
    pub fn builder(currency: Currency, num_paths: usize) -> ExposurePathBuilder {
        ExposurePathBuilder::new(currency, num_paths)
    }
}

// ============================================================================
// ExposurePathBuilder
// ============================================================================

/// Builder for [`PreCalculatedExposurePath`].
///
/// Provides a validated construction API for pre-calculated exposure paths,
/// ensuring path count consistency across all dates.
///
/// # Examples
///
/// ```
/// use infra_master::counterparty::ExposurePathBuilder;
/// use infra_master::market::Currency;
/// use infra_master::time::Date;
///
/// let date1 = Date::from_ymd(2025, 6, 30).unwrap();
/// let date2 = Date::from_ymd(2025, 12, 31).unwrap();
///
/// let path = ExposurePathBuilder::new(Currency::USD, 3)
///     .add_date_exposure(date1, vec![100.0, 200.0, 150.0])
///     .add_date_exposure(date2, vec![120.0, 180.0, 160.0])
///     .build()
///     .unwrap();
///
/// assert_eq!(path.len(), 2);
/// ```
#[derive(Clone, Debug)]
pub struct ExposurePathBuilder {
    currency: Currency,
    num_paths: usize,
    exposures_by_date: std::collections::BTreeMap<Date, Vec<f64>>,
    validation_errors: Vec<String>,
}

impl ExposurePathBuilder {
    /// Creates a new builder with the specified currency and number of paths.
    #[must_use]
    pub fn new(currency: Currency, num_paths: usize) -> Self {
        Self {
            currency,
            num_paths,
            exposures_by_date: std::collections::BTreeMap::new(),
            validation_errors: Vec::new(),
        }
    }

    /// Adds exposure values for a specific date.
    ///
    /// The number of exposure values must match the `num_paths` specified
    /// in the builder constructor.
    #[must_use]
    pub fn add_date_exposure(mut self, date: Date, exposures: Vec<f64>) -> Self {
        if exposures.len() != self.num_paths {
            self.validation_errors.push(format!(
                "Date {}: expected {} paths, got {}",
                date,
                self.num_paths,
                exposures.len()
            ));
        }
        self.exposures_by_date.insert(date, exposures);
        self
    }

    /// Sets the complete time grid with exposures.
    ///
    /// The `dates` and `exposures` vectors must have the same length.
    /// Each exposure vector must have `num_paths` elements.
    #[must_use]
    pub fn with_time_grid(mut self, dates: Vec<Date>, exposures: Vec<Vec<f64>>) -> Self {
        if dates.len() != exposures.len() {
            self.validation_errors.push(format!(
                "Dates/exposures length mismatch: {} dates, {} exposure vectors",
                dates.len(),
                exposures.len()
            ));
            return self;
        }

        for (date, exp) in dates.into_iter().zip(exposures.into_iter()) {
            if exp.len() != self.num_paths {
                self.validation_errors.push(format!(
                    "Date {}: expected {} paths, got {}",
                    date,
                    self.num_paths,
                    exp.len()
                ));
            }
            self.exposures_by_date.insert(date, exp);
        }

        self
    }

    /// Builds the PreCalculatedExposurePath.
    ///
    /// # Errors
    ///
    /// Returns [`CounterPartyError::InvalidCreditParams`] if any validation
    /// errors occurred during construction.
    pub fn build(self) -> Result<PreCalculatedExposurePath, CounterPartyError> {
        if !self.validation_errors.is_empty() {
            return Err(CounterPartyError::InvalidCreditParams(format!(
                "Exposure path validation failed: {}",
                self.validation_errors.join("; ")
            )));
        }

        Ok(PreCalculatedExposurePath {
            exposure_by_date: self.exposures_by_date,
            currency: self.currency,
        })
    }
}

// ============================================================================
// CounterpartyPortfolio
// ============================================================================

/// Counterparty-level portfolio hierarchy.
///
/// Contains the complete hierarchy for XVA calculation:
/// - ISDA agreements with VM agreements and non-CSA trades
/// - Non-nettable trades (gross exposure)
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CounterpartyPortfolio {
    counterparty_id: CounterPartyId,
    credit_index_id: Option<String>,
    isda_agreements: Vec<IsdaMasterAgreement>,
    non_nettable_trades: NonNettableTrades,
}

impl CounterpartyPortfolio {
    /// Creates a new builder.
    #[must_use]
    pub fn builder(counterparty_id: impl Into<CounterPartyId>) -> CounterpartyPortfolioBuilder {
        CounterpartyPortfolioBuilder::new(counterparty_id)
    }

    /// Returns the counterparty ID.
    #[inline]
    #[must_use]
    pub fn counterparty_id(&self) -> &CounterPartyId { &self.counterparty_id }

    /// Returns the credit index ID.
    #[inline]
    #[must_use]
    pub fn credit_index_id(&self) -> Option<&str> { self.credit_index_id.as_deref() }

    /// Returns the ISDA agreements.
    #[inline]
    #[must_use]
    pub fn isda_agreements(&self) -> &[IsdaMasterAgreement] { &self.isda_agreements }

    /// Returns the non-nettable trades.
    #[inline]
    #[must_use]
    pub fn non_nettable_trades(&self) -> &NonNettableTrades { &self.non_nettable_trades }

    /// Iterates over all trade IDs in this portfolio.
    pub fn iter_all_trades(&self) -> impl Iterator<Item = &TradeId> {
        let isda_trades = self
            .isda_agreements
            .iter()
            .flat_map(|isda| isda.iter_all_trades());
        let non_nettable = self.non_nettable_trades.trade_ids.iter();
        isda_trades.chain(non_nettable)
    }

    /// Collects all trade IDs into a set.
    #[must_use]
    pub fn all_trade_ids(&self) -> HashSet<TradeId> { self.iter_all_trades().cloned().collect() }

    /// Gets all currencies from trades using a lookup function.
    pub fn get_all_currencies<F>(&self, trade_currency_fn: F) -> HashSet<Currency>
    where
        F: Fn(&TradeId) -> Option<Currency>,
    {
        self.iter_all_trades()
            .filter_map(trade_currency_fn)
            .collect()
    }

    /// Gets all payment dates from trades using a lookup function.
    pub fn get_all_payment_dates<F>(&self, trade_dates_fn: F) -> BTreeSet<Date>
    where
        F: Fn(&TradeId) -> Vec<Date>,
    {
        self.iter_all_trades().flat_map(trade_dates_fn).collect()
    }
}

// ============================================================================
// CounterpartyPortfolioBuilder
// ============================================================================

/// Builder for CounterpartyPortfolio.
#[derive(Clone, Debug)]
pub struct CounterpartyPortfolioBuilder {
    counterparty_id: CounterPartyId,
    credit_index_id: Option<String>,
    isda_agreements: Vec<IsdaMasterAgreement>,
    non_nettable_trades: NonNettableTrades,
}

impl CounterpartyPortfolioBuilder {
    /// Creates a new builder.
    #[must_use]
    pub fn new(counterparty_id: impl Into<CounterPartyId>) -> Self {
        Self {
            counterparty_id: counterparty_id.into(),
            credit_index_id: None,
            isda_agreements: Vec::new(),
            non_nettable_trades: NonNettableTrades::new(),
        }
    }

    /// Sets the credit index ID.
    #[must_use]
    pub fn credit_index(mut self, index_id: impl Into<String>) -> Self {
        self.credit_index_id = Some(index_id.into());
        self
    }

    /// Adds an ISDA agreement.
    #[must_use]
    pub fn add_isda(mut self, isda: IsdaMasterAgreement) -> Self {
        self.isda_agreements.push(isda);
        self
    }

    /// Adds a non-nettable trade.
    #[must_use]
    pub fn add_non_nettable_trade(mut self, trade_id: impl Into<TradeId>) -> Self {
        self.non_nettable_trades.add_trade(trade_id);
        self
    }

    /// Builds the CounterpartyPortfolio.
    ///
    /// # Errors
    ///
    /// Returns error if validation fails (e.g., counterparty mismatch in ISDA).
    pub fn build(self) -> Result<CounterpartyPortfolio, CounterPartyError> {
        // Validate counterparty consistency in ISDA agreements
        for isda in &self.isda_agreements {
            if isda.counterparty_id != self.counterparty_id {
                return Err(CounterPartyError::CounterpartyMismatch {
                    expected: self.counterparty_id.to_string(),
                    actual: isda.counterparty_id.to_string(),
                });
            }
        }

        Ok(CounterpartyPortfolio {
            counterparty_id: self.counterparty_id,
            credit_index_id: self.credit_index_id,
            isda_agreements: self.isda_agreements,
            non_nettable_trades: self.non_nettable_trades,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // CollateralCallFrequency tests
    // ========================================================================

    #[test]
    fn test_call_frequency_default_mpor() {
        assert_eq!(CollateralCallFrequency::Daily.default_mpor_days(), 10);
        assert_eq!(CollateralCallFrequency::Weekly.default_mpor_days(), 10);
        assert_eq!(CollateralCallFrequency::Biweekly.default_mpor_days(), 14);
        assert_eq!(CollateralCallFrequency::Monthly.default_mpor_days(), 20);
    }

    // ========================================================================
    // IndependentAmountConfig tests
    // ========================================================================

    #[test]
    fn test_independent_amount_calculate() {
        let config = IndependentAmountConfig {
            ia_cpty: 100_000.0,
            k_cpty: 0.1,
            ia_self: -50_000.0,
            k_self: 0.05,
        };

        // Positive PV
        let ia = config.calculate(1_000_000.0);
        assert!((ia - (100_000.0 + 0.1 * 1_000_000.0 - 50_000.0)).abs() < 0.01);

        // Negative PV
        let ia = config.calculate(-500_000.0);
        assert!((ia - (100_000.0 - 50_000.0 + 0.05 * (-500_000.0))).abs() < 0.01);
    }

    // ========================================================================
    // VariationMarginAgreement tests
    // ========================================================================

    #[test]
    fn test_vma_builder_minimal() {
        let vma = VariationMarginAgreement::builder("VMA001", "Test VMA", Currency::USD).build();
        assert_eq!(vma.vma_id().as_str(), "VMA001");
        assert_eq!(vma.name(), "Test VMA");
        assert_eq!(vma.base_currency(), Currency::USD);
        assert_eq!(vma.call_frequency(), CollateralCallFrequency::Daily);
    }

    #[test]
    fn test_vma_builder_asymmetric() {
        let vma = VariationMarginAgreement::builder("VMA001", "Test VMA", Currency::EUR)
            .asymmetric_threshold(1_000_000.0, -500_000.0)
            .asymmetric_mta(50_000.0, -25_000.0)
            .asymmetric_haircut(-0.05, 0.05)
            .build();

        assert!((vma.threshold_cpty() - 1_000_000.0).abs() < 0.01);
        assert!((vma.threshold_self() - (-500_000.0)).abs() < 0.01);
        assert!((vma.mta_cpty() - 50_000.0).abs() < 0.01);
        assert!((vma.mta_self() - (-25_000.0)).abs() < 0.01);
    }

    #[test]
    fn test_vma_with_trades() {
        let vma = VariationMarginAgreement::builder("VMA001", "Test", Currency::USD)
            .add_trade("T001")
            .add_trade("T002")
            .add_trade("T001") // Duplicate
            .build();

        assert_eq!(vma.trade_ids().len(), 2);
    }

    // ========================================================================
    // IsdaMasterAgreement tests
    // ========================================================================

    #[test]
    fn test_isda_builder_minimal() {
        let isda = IsdaMasterAgreement::builder("ISDA001", "Test ISDA", "CP001").build();
        assert_eq!(isda.isda_id().as_str(), "ISDA001");
        assert_eq!(isda.name(), "Test ISDA");
        assert_eq!(isda.counterparty_id().as_str(), "CP001");
        assert_eq!(isda.payment_method(), IsdaPaymentMethod::Full);
    }

    #[test]
    fn test_isda_with_vma_and_non_csa() {
        let vma = VariationMarginAgreement::builder("VMA001", "Test VMA", Currency::USD)
            .add_trade("T001")
            .add_trade("T002")
            .build();

        let isda = IsdaMasterAgreement::builder("ISDA001", "Test ISDA", "CP001")
            .add_vma(vma)
            .add_non_csa_trade("T003")
            .add_non_csa_trade("T004")
            .build();

        assert_eq!(isda.variation_margin_agreements().len(), 1);
        assert_eq!(isda.non_csa_trade_ids().len(), 2);

        let all_trades: Vec<_> = isda.iter_all_trades().collect();
        assert_eq!(all_trades.len(), 4);
    }

    // ========================================================================
    // NonNettableTrades tests
    // ========================================================================

    #[test]
    fn test_non_nettable_trades() {
        let mut trades = NonNettableTrades::new();
        assert!(trades.is_empty());

        trades.add_trade("T001");
        trades.add_trade("T002");
        trades.add_trade("T001"); // Duplicate

        assert_eq!(trades.len(), 2);
        assert!(!trades.is_empty());
    }

    // ========================================================================
    // PreCalculatedExposurePath tests
    // ========================================================================

    #[test]
    fn test_precalc_exposure_path() {
        let mut path = PreCalculatedExposurePath::new(Currency::USD);
        assert!(path.is_empty());

        let date1 = Date::from_ymd(2025, 6, 30).unwrap();
        let date2 = Date::from_ymd(2025, 12, 31).unwrap();

        path.add_exposure(date1, vec![100.0, 200.0, 150.0]);
        path.add_exposure(date2, vec![120.0, 180.0, 160.0]);

        assert_eq!(path.len(), 2);
        assert_eq!(path.exposure_at(&date1).unwrap().len(), 3);
        assert_eq!(path.currency(), Currency::USD);
    }

    // ========================================================================
    // CounterpartyPortfolio tests
    // ========================================================================

    #[test]
    fn test_cp_portfolio_builder_minimal() {
        let portfolio = CounterpartyPortfolio::builder("CP001").build().unwrap();
        assert_eq!(portfolio.counterparty_id().as_str(), "CP001");
        assert!(portfolio.isda_agreements().is_empty());
        assert!(portfolio.non_nettable_trades().is_empty());
    }

    #[test]
    fn test_cp_portfolio_full_hierarchy() {
        let vma = VariationMarginAgreement::builder("VMA001", "Test VMA", Currency::USD)
            .add_trade("T001")
            .add_trade("T002")
            .build();

        let isda = IsdaMasterAgreement::builder("ISDA001", "Test ISDA", "CP001")
            .add_vma(vma)
            .add_non_csa_trade("T003")
            .build();

        let portfolio = CounterpartyPortfolio::builder("CP001")
            .credit_index("CDX.NA.IG")
            .add_isda(isda)
            .add_non_nettable_trade("T004")
            .build()
            .unwrap();

        assert_eq!(portfolio.counterparty_id().as_str(), "CP001");
        assert_eq!(portfolio.credit_index_id(), Some("CDX.NA.IG"));
        assert_eq!(portfolio.isda_agreements().len(), 1);
        assert_eq!(portfolio.non_nettable_trades().len(), 1);

        let all_trades = portfolio.all_trade_ids();
        assert_eq!(all_trades.len(), 4);
    }

    #[test]
    fn test_cp_portfolio_counterparty_mismatch() {
        let isda = IsdaMasterAgreement::builder("ISDA001", "Test ISDA", "CP002").build();

        let result = CounterpartyPortfolio::builder("CP001")
            .add_isda(isda)
            .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_cp_portfolio_get_all_currencies() {
        let vma = VariationMarginAgreement::builder("VMA001", "Test", Currency::USD)
            .add_trade("T001")
            .add_trade("T002")
            .build();

        let isda = IsdaMasterAgreement::builder("ISDA001", "Test", "CP001")
            .add_vma(vma)
            .build();

        let portfolio = CounterpartyPortfolio::builder("CP001")
            .add_isda(isda)
            .build()
            .unwrap();

        // Mock currency lookup
        let currencies = portfolio.get_all_currencies(|id| match id.as_str() {
            "T001" => Some(Currency::USD),
            "T002" => Some(Currency::EUR),
            _ => None,
        });

        assert!(currencies.contains(&Currency::USD));
        assert!(currencies.contains(&Currency::EUR));
    }

    // ========================================================================
    // ExposurePathBuilder tests
    // ========================================================================

    #[test]
    fn test_exposure_path_builder_basic() {
        let date1 = Date::from_ymd(2025, 6, 30).unwrap();
        let date2 = Date::from_ymd(2025, 12, 31).unwrap();

        let path = ExposurePathBuilder::new(Currency::USD, 3)
            .add_date_exposure(date1, vec![100.0, 200.0, 150.0])
            .add_date_exposure(date2, vec![120.0, 180.0, 160.0])
            .build()
            .unwrap();

        assert_eq!(path.len(), 2);
        assert_eq!(path.currency(), Currency::USD);
        assert_eq!(
            path.exposure_at(&date1).unwrap(),
            &vec![100.0, 200.0, 150.0]
        );
    }

    #[test]
    fn test_exposure_path_builder_path_count_mismatch() {
        let date1 = Date::from_ymd(2025, 6, 30).unwrap();
        let date2 = Date::from_ymd(2025, 12, 31).unwrap();

        let result = ExposurePathBuilder::new(Currency::USD, 3)
            .add_date_exposure(date1, vec![100.0, 200.0, 150.0])
            .add_date_exposure(date2, vec![120.0, 180.0]) // Only 2 paths, should be 3
            .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_exposure_path_builder_empty_dates() {
        let result = ExposurePathBuilder::new(Currency::EUR, 3).build();

        // Building with no dates should succeed (empty path is valid)
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_exposure_path_builder_with_time_grid() {
        let dates = vec![
            Date::from_ymd(2025, 3, 31).unwrap(),
            Date::from_ymd(2025, 6, 30).unwrap(),
            Date::from_ymd(2025, 12, 31).unwrap(),
        ];

        let exposures = vec![vec![100.0, 110.0], vec![120.0, 130.0], vec![140.0, 150.0]];

        let path = ExposurePathBuilder::new(Currency::JPY, 2)
            .with_time_grid(dates.clone(), exposures.clone())
            .build()
            .unwrap();

        assert_eq!(path.len(), 3);
        for (date, exp) in dates.iter().zip(exposures.iter()) {
            assert_eq!(path.exposure_at(date).unwrap(), exp);
        }
    }

    #[test]
    fn test_exposure_path_builder_date_exposure_mismatch() {
        let dates = vec![
            Date::from_ymd(2025, 3, 31).unwrap(),
            Date::from_ymd(2025, 6, 30).unwrap(),
        ];

        let exposures = vec![
            vec![100.0, 110.0],
            vec![120.0, 130.0],
            vec![140.0, 150.0], // 3 exposure vectors, but only 2 dates
        ];

        let result = ExposurePathBuilder::new(Currency::JPY, 2)
            .with_time_grid(dates, exposures)
            .build();

        assert!(result.is_err());
    }
}
