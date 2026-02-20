//! Observable definitions for market data construction.

use serde::{Deserialize, Serialize};

use crate::market::{CurrencyPair, FxIndex, RateIndex, SwapIndex};

// ---------------------------------------------------------------------------
// BondObservableSubtype
// ---------------------------------------------------------------------------

/// Output type for bond observables.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BondObservableSubtype {
    /// Bond price (normalised dirty/clean price).
    #[default]
    Price,
    /// Bond yield.
    Yield,
    /// Change in bond yield (delta).
    YieldDelta,
}

impl BondObservableSubtype {
    /// Returns the display name.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Price => "Price",
            Self::Yield => "Yield",
            Self::YieldDelta => "Yield Delta",
        }
    }
}

impl std::fmt::Display for BondObservableSubtype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

// ---------------------------------------------------------------------------
// FixingRequirement
// ---------------------------------------------------------------------------

/// Fixing configuration for an observable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FixingRequirement {
    /// Fixing source identifier (e.g., "ISDA", "Bloomberg", "ECB").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,

    /// Whether historical fixings must be provided (vs. purely forward-looking).
    #[serde(default)]
    pub requires_historical: bool,
}

impl FixingRequirement {
    /// Creates a new fixing requirement.
    #[must_use]
    pub fn new() -> Self {
        Self {
            source: None,
            requires_historical: false,
        }
    }

    /// Sets the fixing source.
    #[must_use]
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Sets whether historical fixings are required.
    #[must_use]
    pub fn with_requires_historical(mut self, required: bool) -> Self {
        self.requires_historical = required;
        self
    }
}

impl Default for FixingRequirement {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Variant definition structs
// ---------------------------------------------------------------------------

/// Cash rate observable definition (SOFR, EURIBOR, etc.).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CashObservableDef {
    /// Unique name for this observable.
    pub name: String,
    /// Name of the IR curve (references `CurveDefinition.name`).
    pub curve_name: String,
    /// Rate index for this cash observable.
    pub rate_index: RateIndex,
    /// Fixing configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixing: Option<FixingRequirement>,
}

impl CashObservableDef {
    /// Creates a new cash observable definition.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        curve_name: impl Into<String>,
        rate_index: RateIndex,
    ) -> Self {
        Self {
            name: name.into(),
            curve_name: curve_name.into(),
            rate_index,
            fixing: None,
        }
    }

    /// Sets the fixing requirement.
    #[must_use]
    pub fn with_fixing(mut self, fixing: FixingRequirement) -> Self {
        self.fixing = Some(fixing);
        self
    }
}

/// Swap rate observable definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwapObservableDef {
    /// Unique name for this observable.
    pub name: String,
    /// Name of the underlying cash observable definition.
    pub cash_observable_name: String,
    /// Name of the discount IR curve.
    pub discount_curve_name: String,
    /// Swap index.
    pub swap_index: SwapIndex,
    /// Fixing configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixing: Option<FixingRequirement>,
}

impl SwapObservableDef {
    /// Creates a new swap observable definition.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        cash_observable_name: impl Into<String>,
        discount_curve_name: impl Into<String>,
        swap_index: SwapIndex,
    ) -> Self {
        Self {
            name: name.into(),
            cash_observable_name: cash_observable_name.into(),
            discount_curve_name: discount_curve_name.into(),
            swap_index,
            fixing: None,
        }
    }

    /// Sets the fixing requirement.
    #[must_use]
    pub fn with_fixing(mut self, fixing: FixingRequirement) -> Self {
        self.fixing = Some(fixing);
        self
    }
}

/// Equity price observable definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EquityObservableDef {
    /// Unique name for this observable.
    pub name: String,
    /// Name of the equity forward curve.
    pub curve_name: String,
    /// Equity ticker or index identifier.
    pub ticker: String,
    /// Fixing configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixing: Option<FixingRequirement>,
}

impl EquityObservableDef {
    /// Creates a new equity observable definition.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        curve_name: impl Into<String>,
        ticker: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            curve_name: curve_name.into(),
            ticker: ticker.into(),
            fixing: None,
        }
    }

    /// Sets the fixing requirement.
    #[must_use]
    pub fn with_fixing(mut self, fixing: FixingRequirement) -> Self {
        self.fixing = Some(fixing);
        self
    }
}

/// FX rate observable definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FxObservableDef {
    /// Unique name for this observable.
    pub name: String,
    /// Currency pair (e.g., EUR/USD).
    pub currency_pair: CurrencyPair,
    /// Name of the foreign (base) currency IR curve.
    pub foreign_curve_name: String,
    /// Name of the domestic (quote) currency IR curve.
    pub domestic_curve_name: String,
    /// Optional FX index for fixing source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fx_index: Option<FxIndex>,
    /// Fixing configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixing: Option<FixingRequirement>,
}

impl FxObservableDef {
    /// Creates a new FX observable definition.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        currency_pair: CurrencyPair,
        foreign_curve_name: impl Into<String>,
        domestic_curve_name: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            currency_pair,
            foreign_curve_name: foreign_curve_name.into(),
            domestic_curve_name: domestic_curve_name.into(),
            fx_index: None,
            fixing: None,
        }
    }

    /// Sets the FX index.
    #[must_use]
    pub fn with_fx_index(mut self, fx_index: FxIndex) -> Self {
        self.fx_index = Some(fx_index);
        self
    }

    /// Sets the fixing requirement.
    #[must_use]
    pub fn with_fixing(mut self, fixing: FixingRequirement) -> Self {
        self.fixing = Some(fixing);
        self
    }
}

/// Commodity price observable definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommodityObservableDef {
    /// Unique name for this observable.
    pub name: String,
    /// Commodity name (e.g., "WTI", "BRENT").
    pub commodity: String,
    /// Name of the commodity forward curve (optional if fixings cover all dates).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub curve_name: Option<String>,
    /// Fixing configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixing: Option<FixingRequirement>,
}

impl CommodityObservableDef {
    /// Creates a new commodity observable definition with a curve.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        commodity: impl Into<String>,
        curve_name: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            commodity: commodity.into(),
            curve_name: Some(curve_name.into()),
            fixing: None,
        }
    }

    /// Creates a commodity observable definition without a curve (fixing-only).
    #[must_use]
    pub fn fixing_only(name: impl Into<String>, commodity: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            commodity: commodity.into(),
            curve_name: None,
            fixing: None,
        }
    }

    /// Sets the fixing requirement.
    #[must_use]
    pub fn with_fixing(mut self, fixing: FixingRequirement) -> Self {
        self.fixing = Some(fixing);
        self
    }
}

/// Inflation index observable definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InflationObservableDef {
    /// Unique name for this observable.
    pub name: String,
    /// Name of the inflation curve.
    pub curve_name: String,
    /// Inflation index name (e.g., "CPI-US", "RPI-UK").
    pub inflation_index: String,
    /// Region or country code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    /// Fixing configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixing: Option<FixingRequirement>,
}

impl InflationObservableDef {
    /// Creates a new inflation observable definition.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        curve_name: impl Into<String>,
        inflation_index: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            curve_name: curve_name.into(),
            inflation_index: inflation_index.into(),
            region: None,
            fixing: None,
        }
    }

    /// Sets the region code.
    #[must_use]
    pub fn with_region(mut self, region: impl Into<String>) -> Self {
        self.region = Some(region.into());
        self
    }

    /// Sets the fixing requirement.
    #[must_use]
    pub fn with_fixing(mut self, fixing: FixingRequirement) -> Self {
        self.fixing = Some(fixing);
        self
    }
}

/// Credit spread observable definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreditObservableDef {
    /// Unique name for this observable.
    pub name: String,
    /// Name of the credit (survival probability) curve.
    pub curve_name: String,
    /// Reference entity name.
    pub reference_entity: String,
    /// Seniority tier (e.g., "SNRFOR", "SUBLT2").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seniority: Option<String>,
}

impl CreditObservableDef {
    /// Creates a new credit observable definition.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        curve_name: impl Into<String>,
        reference_entity: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            curve_name: curve_name.into(),
            reference_entity: reference_entity.into(),
            seniority: None,
        }
    }

    /// Sets the seniority tier.
    #[must_use]
    pub fn with_seniority(mut self, seniority: impl Into<String>) -> Self {
        self.seniority = Some(seniority.into());
        self
    }
}

/// Bond price/yield observable definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BondObservableDef {
    /// Unique name for this observable.
    pub name: String,
    /// Name of the bond discount curve.
    pub discount_curve_name: String,
    /// Name of the repo curve.
    pub repo_curve_name: String,
    /// Bond instrument definition ID.
    pub bond_instrument_id: String,
    /// Output type (price, yield, or yield delta).
    #[serde(default)]
    pub subtype: BondObservableSubtype,
}

impl BondObservableDef {
    /// Creates a new bond observable definition.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        discount_curve_name: impl Into<String>,
        repo_curve_name: impl Into<String>,
        bond_instrument_id: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            discount_curve_name: discount_curve_name.into(),
            repo_curve_name: repo_curve_name.into(),
            bond_instrument_id: bond_instrument_id.into(),
            subtype: BondObservableSubtype::default(),
        }
    }

    /// Sets the bond observable subtype.
    #[must_use]
    pub fn with_subtype(mut self, subtype: BondObservableSubtype) -> Self {
        self.subtype = subtype;
        self
    }
}

/// Bond future observable definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BondFutureObservableDef {
    /// Unique name for this observable.
    pub name: String,
    /// Name of the base IR curve.
    pub base_curve_name: String,
    /// Name of the spread IR curve.
    pub spread_curve_name: String,
    /// Bond future instrument definition ID.
    pub bond_future_instrument_id: String,
}

impl BondFutureObservableDef {
    /// Creates a new bond future observable definition.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        base_curve_name: impl Into<String>,
        spread_curve_name: impl Into<String>,
        bond_future_instrument_id: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            base_curve_name: base_curve_name.into(),
            spread_curve_name: spread_curve_name.into(),
            bond_future_instrument_id: bond_future_instrument_id.into(),
        }
    }
}

/// IR future observable definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IrFutureObservableDef {
    /// Unique name for this observable.
    pub name: String,
    /// Name of the base IR curve.
    pub base_curve_name: String,
    /// Name of the spread IR curve.
    pub spread_curve_name: String,
    /// IR future instrument definition ID.
    pub ir_future_instrument_id: String,
}

impl IrFutureObservableDef {
    /// Creates a new IR future observable definition.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        base_curve_name: impl Into<String>,
        spread_curve_name: impl Into<String>,
        ir_future_instrument_id: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            base_curve_name: base_curve_name.into(),
            spread_curve_name: spread_curve_name.into(),
            ir_future_instrument_id: ir_future_instrument_id.into(),
        }
    }
}

// ---------------------------------------------------------------------------
// ObservableDefinition
// ---------------------------------------------------------------------------

/// Observable definition — the recipe for constructing market observables.
///
/// Each variant specifies the curve dependencies and fixing requirements
/// needed to construct a live observable at runtime in the pricer layer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
#[non_exhaustive]
pub enum ObservableDefinition {
    /// Cash (money market / overnight rate) observable.
    Cash(CashObservableDef),
    /// Swap rate observable.
    Swap(SwapObservableDef),
    /// Equity spot/forward observable.
    Equity(EquityObservableDef),
    /// FX rate observable.
    Fx(FxObservableDef),
    /// Commodity observable.
    Commodity(CommodityObservableDef),
    /// Inflation index observable.
    Inflation(InflationObservableDef),
    /// Credit (survival curve) observable.
    Credit(CreditObservableDef),
    /// Bond observable (price, yield, or yield delta).
    Bond(BondObservableDef),
    /// Bond future observable.
    BondFuture(BondFutureObservableDef),
    /// IR future observable.
    IrFuture(IrFutureObservableDef),
}

impl ObservableDefinition {
    /// Returns the observable name.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Cash(d) => &d.name,
            Self::Swap(d) => &d.name,
            Self::Equity(d) => &d.name,
            Self::Fx(d) => &d.name,
            Self::Commodity(d) => &d.name,
            Self::Inflation(d) => &d.name,
            Self::Credit(d) => &d.name,
            Self::Bond(d) => &d.name,
            Self::BondFuture(d) => &d.name,
            Self::IrFuture(d) => &d.name,
        }
    }

    /// Returns the observable type as a string label.
    #[must_use]
    pub fn observable_type(&self) -> &'static str {
        match self {
            Self::Cash(_) => "cash",
            Self::Swap(_) => "swap",
            Self::Equity(_) => "equity",
            Self::Fx(_) => "fx",
            Self::Commodity(_) => "commodity",
            Self::Inflation(_) => "inflation",
            Self::Credit(_) => "credit",
            Self::Bond(_) => "bond",
            Self::BondFuture(_) => "bond_future",
            Self::IrFuture(_) => "ir_future",
        }
    }

    /// Returns the names of all curves this observable depends on.
    #[must_use]
    pub fn curve_dependencies(&self) -> Vec<&str> {
        match self {
            Self::Cash(d) => vec![&d.curve_name],
            Self::Swap(d) => vec![&d.discount_curve_name],
            Self::Equity(d) => vec![&d.curve_name],
            Self::Fx(d) => vec![&d.foreign_curve_name, &d.domestic_curve_name],
            Self::Commodity(d) => d.curve_name.as_deref().into_iter().collect(),
            Self::Inflation(d) => vec![&d.curve_name],
            Self::Credit(d) => vec![&d.curve_name],
            Self::Bond(d) => vec![&d.discount_curve_name, &d.repo_curve_name],
            Self::BondFuture(d) => vec![&d.base_curve_name, &d.spread_curve_name],
            Self::IrFuture(d) => vec![&d.base_curve_name, &d.spread_curve_name],
        }
    }

    /// Returns `true` if this observable type requires fixing data.
    #[must_use]
    pub fn requires_fixing(&self) -> bool {
        matches!(
            self,
            Self::Cash(_)
                | Self::Swap(_)
                | Self::Equity(_)
                | Self::Fx(_)
                | Self::Commodity(_)
                | Self::Inflation(_)
        )
    }

    /// Validates the observable definition.
    pub fn validate(&self) -> Result<(), ObservableDefError> {
        match self {
            Self::Cash(d) => {
                check_non_empty(&d.name, "name")?;
                check_non_empty(&d.curve_name, "curve_name")?;
            }
            Self::Swap(d) => {
                check_non_empty(&d.name, "name")?;
                check_non_empty(&d.cash_observable_name, "cash_observable_name")?;
                check_non_empty(&d.discount_curve_name, "discount_curve_name")?;
            }
            Self::Equity(d) => {
                check_non_empty(&d.name, "name")?;
                check_non_empty(&d.curve_name, "curve_name")?;
                check_non_empty(&d.ticker, "ticker")?;
            }
            Self::Fx(d) => {
                check_non_empty(&d.name, "name")?;
                check_non_empty(&d.foreign_curve_name, "foreign_curve_name")?;
                check_non_empty(&d.domestic_curve_name, "domestic_curve_name")?;
            }
            Self::Commodity(d) => {
                check_non_empty(&d.name, "name")?;
                check_non_empty(&d.commodity, "commodity")?;
                if d.curve_name.is_none() && d.fixing.is_none() {
                    return Err(ObservableDefError::CommodityRequiresCurveOrFixing(
                        d.name.clone(),
                    ));
                }
            }
            Self::Inflation(d) => {
                check_non_empty(&d.name, "name")?;
                check_non_empty(&d.curve_name, "curve_name")?;
                check_non_empty(&d.inflation_index, "inflation_index")?;
            }
            Self::Credit(d) => {
                check_non_empty(&d.name, "name")?;
                check_non_empty(&d.curve_name, "curve_name")?;
                check_non_empty(&d.reference_entity, "reference_entity")?;
            }
            Self::Bond(d) => {
                check_non_empty(&d.name, "name")?;
                check_non_empty(&d.discount_curve_name, "discount_curve_name")?;
                check_non_empty(&d.repo_curve_name, "repo_curve_name")?;
                check_non_empty(&d.bond_instrument_id, "bond_instrument_id")?;
            }
            Self::BondFuture(d) => {
                check_non_empty(&d.name, "name")?;
                check_non_empty(&d.base_curve_name, "base_curve_name")?;
                check_non_empty(&d.spread_curve_name, "spread_curve_name")?;
                check_non_empty(&d.bond_future_instrument_id, "bond_future_instrument_id")?;
            }
            Self::IrFuture(d) => {
                check_non_empty(&d.name, "name")?;
                check_non_empty(&d.base_curve_name, "base_curve_name")?;
                check_non_empty(&d.spread_curve_name, "spread_curve_name")?;
                check_non_empty(&d.ir_future_instrument_id, "ir_future_instrument_id")?;
            }
        }
        Ok(())
    }
}

fn check_non_empty(value: &str, field: &'static str) -> Result<(), ObservableDefError> {
    if value.is_empty() {
        return Err(ObservableDefError::EmptyField(field));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// ObservableDefError
// ---------------------------------------------------------------------------

/// Error type for observable definition validation.
#[derive(Debug, Clone, PartialEq)]
pub enum ObservableDefError {
    /// A required field is empty.
    EmptyField(&'static str),
    /// Commodity observable has neither curve nor fixings configured.
    CommodityRequiresCurveOrFixing(String),
}

impl std::fmt::Display for ObservableDefError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyField(field) => write!(f, "Required field is empty: {field}"),
            Self::CommodityRequiresCurveOrFixing(name) => {
                write!(
                    f,
                    "Commodity observable '{name}' requires either a curve or fixing configuration"
                )
            }
        }
    }
}

impl std::error::Error for ObservableDefError {}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market::Currency;

    // -- BondObservableSubtype -----------------------------------------------

    #[test]
    fn test_bond_observable_subtype_default() {
        assert_eq!(BondObservableSubtype::default(), BondObservableSubtype::Price);
    }

    #[test]
    fn test_bond_observable_subtype_display() {
        assert_eq!(BondObservableSubtype::Price.to_string(), "Price");
        assert_eq!(BondObservableSubtype::Yield.to_string(), "Yield");
        assert_eq!(BondObservableSubtype::YieldDelta.to_string(), "Yield Delta");
    }

    #[test]
    fn test_bond_observable_subtype_serde() {
        let json = serde_json::to_string(&BondObservableSubtype::YieldDelta).unwrap();
        assert_eq!(json, "\"yield_delta\"");
        let parsed: BondObservableSubtype = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, BondObservableSubtype::YieldDelta);
    }

    // -- FixingRequirement ---------------------------------------------------

    #[test]
    fn test_fixing_requirement_default() {
        let f = FixingRequirement::default();
        assert!(f.source.is_none());
        assert!(!f.requires_historical);
    }

    #[test]
    fn test_fixing_requirement_builder() {
        let f = FixingRequirement::new()
            .with_source("Bloomberg")
            .with_requires_historical(true);
        assert_eq!(f.source.as_deref(), Some("Bloomberg"));
        assert!(f.requires_historical);
    }

    #[test]
    fn test_fixing_requirement_serde() {
        let f = FixingRequirement::new().with_source("ECB");
        let json = serde_json::to_string(&f).unwrap();
        let parsed: FixingRequirement = serde_json::from_str(&json).unwrap();
        assert_eq!(f, parsed);
    }

    // -- CashObservableDef ---------------------------------------------------

    #[test]
    fn test_cash_observable_new() {
        let def = CashObservableDef::new("USD-SOFR-Cash", "USD-SOFR", RateIndex::Sofr);
        assert_eq!(def.name, "USD-SOFR-Cash");
        assert_eq!(def.curve_name, "USD-SOFR");
        assert_eq!(def.rate_index, RateIndex::Sofr);
        assert!(def.fixing.is_none());
    }

    #[test]
    fn test_cash_observable_with_fixing() {
        let def = CashObservableDef::new("EUR-ESTR-Cash", "EUR-ESTR", RateIndex::Estr)
            .with_fixing(FixingRequirement::new().with_source("ECB"));
        assert!(def.fixing.is_some());
        assert_eq!(def.fixing.unwrap().source.as_deref(), Some("ECB"));
    }

    // -- SwapObservableDef ---------------------------------------------------

    #[test]
    fn test_swap_observable_new() {
        let def = SwapObservableDef::new(
            "USD-CMS10Y",
            "USD-SOFR-Cash",
            "USD-SOFR-Discount",
            SwapIndex::UsdCms10Y,
        );
        assert_eq!(def.name, "USD-CMS10Y");
        assert_eq!(def.cash_observable_name, "USD-SOFR-Cash");
        assert_eq!(def.discount_curve_name, "USD-SOFR-Discount");
        assert_eq!(def.swap_index, SwapIndex::UsdCms10Y);
    }

    // -- EquityObservableDef -------------------------------------------------

    #[test]
    fn test_equity_observable_new() {
        let def = EquityObservableDef::new("SPX-Obs", "SPX-Forward", "SPX");
        assert_eq!(def.name, "SPX-Obs");
        assert_eq!(def.curve_name, "SPX-Forward");
        assert_eq!(def.ticker, "SPX");
    }

    // -- FxObservableDef -----------------------------------------------------

    #[test]
    fn test_fx_observable_new() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let def = FxObservableDef::new("EURUSD-Spot", pair, "EUR-ESTR", "USD-SOFR");
        assert_eq!(def.name, "EURUSD-Spot");
        assert_eq!(def.currency_pair, pair);
        assert_eq!(def.foreign_curve_name, "EUR-ESTR");
        assert_eq!(def.domestic_curve_name, "USD-SOFR");
        assert!(def.fx_index.is_none());
    }

    #[test]
    fn test_fx_observable_with_index() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let def = FxObservableDef::new("EURUSD-Spot", pair, "EUR-ESTR", "USD-SOFR")
            .with_fx_index(FxIndex::EcbEurUsd);
        assert_eq!(def.fx_index, Some(FxIndex::EcbEurUsd));
    }

    // -- CommodityObservableDef ----------------------------------------------

    #[test]
    fn test_commodity_observable_with_curve() {
        let def = CommodityObservableDef::new("WTI-Obs", "WTI", "WTI-Forward");
        assert_eq!(def.commodity, "WTI");
        assert_eq!(def.curve_name.as_deref(), Some("WTI-Forward"));
    }

    #[test]
    fn test_commodity_observable_fixing_only() {
        let def = CommodityObservableDef::fixing_only("BRENT-Obs", "BRENT")
            .with_fixing(FixingRequirement::new());
        assert!(def.curve_name.is_none());
        assert!(def.fixing.is_some());
    }

    // -- InflationObservableDef ----------------------------------------------

    #[test]
    fn test_inflation_observable_new() {
        let def = InflationObservableDef::new("US-CPI-Obs", "US-CPI-Curve", "CPI-US")
            .with_region("US");
        assert_eq!(def.inflation_index, "CPI-US");
        assert_eq!(def.region.as_deref(), Some("US"));
    }

    // -- CreditObservableDef -------------------------------------------------

    #[test]
    fn test_credit_observable_new() {
        let def =
            CreditObservableDef::new("ACME-Credit", "ACME-Surv", "ACME Corp").with_seniority("SNRFOR");
        assert_eq!(def.reference_entity, "ACME Corp");
        assert_eq!(def.seniority.as_deref(), Some("SNRFOR"));
    }

    // -- BondObservableDef ---------------------------------------------------

    #[test]
    fn test_bond_observable_new() {
        let def = BondObservableDef::new("UST-10Y", "USD-SOFR", "USD-Repo", "UST-10Y-Bond");
        assert_eq!(def.subtype, BondObservableSubtype::Price);
    }

    #[test]
    fn test_bond_observable_with_subtype() {
        let def = BondObservableDef::new("UST-10Y", "USD-SOFR", "USD-Repo", "UST-10Y-Bond")
            .with_subtype(BondObservableSubtype::Yield);
        assert_eq!(def.subtype, BondObservableSubtype::Yield);
    }

    // -- BondFutureObservableDef ---------------------------------------------

    #[test]
    fn test_bond_future_observable_new() {
        let def = BondFutureObservableDef::new("TY-Future", "USD-SOFR", "USD-Spread", "TY-Mar25");
        assert_eq!(def.bond_future_instrument_id, "TY-Mar25");
    }

    // -- IrFutureObservableDef -----------------------------------------------

    #[test]
    fn test_ir_future_observable_new() {
        let def = IrFutureObservableDef::new("SR3-Future", "USD-SOFR", "USD-Spread", "SR3-Mar25");
        assert_eq!(def.ir_future_instrument_id, "SR3-Mar25");
    }

    // -- ObservableDefinition enum -------------------------------------------

    #[test]
    fn test_name() {
        let obs = ObservableDefinition::Cash(CashObservableDef::new(
            "USD-SOFR-Cash",
            "USD-SOFR",
            RateIndex::Sofr,
        ));
        assert_eq!(obs.name(), "USD-SOFR-Cash");
    }

    #[test]
    fn test_observable_type() {
        let cases: Vec<(ObservableDefinition, &str)> = vec![
            (
                ObservableDefinition::Cash(CashObservableDef::new("c", "c", RateIndex::Sofr)),
                "cash",
            ),
            (
                ObservableDefinition::Swap(SwapObservableDef::new(
                    "s",
                    "c",
                    "d",
                    SwapIndex::UsdCms10Y,
                )),
                "swap",
            ),
            (
                ObservableDefinition::Equity(EquityObservableDef::new("e", "c", "SPX")),
                "equity",
            ),
            (
                ObservableDefinition::Fx(FxObservableDef::new(
                    "f",
                    CurrencyPair::new(Currency::EUR, Currency::USD),
                    "fc",
                    "dc",
                )),
                "fx",
            ),
            (
                ObservableDefinition::Commodity(CommodityObservableDef::new("m", "WTI", "c")),
                "commodity",
            ),
            (
                ObservableDefinition::Inflation(InflationObservableDef::new("i", "c", "CPI")),
                "inflation",
            ),
            (
                ObservableDefinition::Credit(CreditObservableDef::new("cr", "c", "ACME")),
                "credit",
            ),
            (
                ObservableDefinition::Bond(BondObservableDef::new("b", "d", "r", "inst")),
                "bond",
            ),
            (
                ObservableDefinition::BondFuture(BondFutureObservableDef::new(
                    "bf", "b", "s", "inst",
                )),
                "bond_future",
            ),
            (
                ObservableDefinition::IrFuture(IrFutureObservableDef::new("ir", "b", "s", "inst")),
                "ir_future",
            ),
        ];

        for (obs, expected) in cases {
            assert_eq!(obs.observable_type(), expected);
        }
    }

    #[test]
    fn test_curve_dependencies_single() {
        let obs = ObservableDefinition::Cash(CashObservableDef::new(
            "USD-SOFR-Cash",
            "USD-SOFR",
            RateIndex::Sofr,
        ));
        assert_eq!(obs.curve_dependencies(), vec!["USD-SOFR"]);
    }

    #[test]
    fn test_curve_dependencies_dual() {
        let obs = ObservableDefinition::Fx(FxObservableDef::new(
            "EURUSD",
            CurrencyPair::new(Currency::EUR, Currency::USD),
            "EUR-ESTR",
            "USD-SOFR",
        ));
        assert_eq!(obs.curve_dependencies(), vec!["EUR-ESTR", "USD-SOFR"]);
    }

    #[test]
    fn test_curve_dependencies_commodity_none() {
        let obs = ObservableDefinition::Commodity(
            CommodityObservableDef::fixing_only("BRENT", "BRENT")
                .with_fixing(FixingRequirement::new()),
        );
        assert!(obs.curve_dependencies().is_empty());
    }

    #[test]
    fn test_requires_fixing() {
        assert!(
            ObservableDefinition::Cash(CashObservableDef::new("c", "c", RateIndex::Sofr))
                .requires_fixing()
        );
        assert!(ObservableDefinition::Fx(FxObservableDef::new(
            "f",
            CurrencyPair::new(Currency::EUR, Currency::USD),
            "fc",
            "dc",
        ))
        .requires_fixing());
        assert!(
            !ObservableDefinition::Credit(CreditObservableDef::new("cr", "c", "ACME"))
                .requires_fixing()
        );
        assert!(
            !ObservableDefinition::Bond(BondObservableDef::new("b", "d", "r", "i"))
                .requires_fixing()
        );
    }

    // -- Validation ----------------------------------------------------------

    #[test]
    fn test_validate_success() {
        let obs = ObservableDefinition::Cash(CashObservableDef::new(
            "USD-SOFR-Cash",
            "USD-SOFR",
            RateIndex::Sofr,
        ));
        assert!(obs.validate().is_ok());
    }

    #[test]
    fn test_validate_empty_name() {
        let obs =
            ObservableDefinition::Cash(CashObservableDef::new("", "USD-SOFR", RateIndex::Sofr));
        assert!(matches!(
            obs.validate(),
            Err(ObservableDefError::EmptyField("name"))
        ));
    }

    #[test]
    fn test_validate_empty_curve_name() {
        let obs = ObservableDefinition::Cash(CashObservableDef::new(
            "USD-SOFR-Cash",
            "",
            RateIndex::Sofr,
        ));
        assert!(matches!(
            obs.validate(),
            Err(ObservableDefError::EmptyField("curve_name"))
        ));
    }

    #[test]
    fn test_validate_commodity_requires_curve_or_fixing() {
        let obs = ObservableDefinition::Commodity(CommodityObservableDef::fixing_only(
            "BRENT-Obs", "BRENT",
        ));
        assert!(matches!(
            obs.validate(),
            Err(ObservableDefError::CommodityRequiresCurveOrFixing(_))
        ));
    }

    #[test]
    fn test_validate_commodity_with_curve_ok() {
        let obs =
            ObservableDefinition::Commodity(CommodityObservableDef::new("WTI", "WTI", "WTI-Fwd"));
        assert!(obs.validate().is_ok());
    }

    #[test]
    fn test_validate_commodity_with_fixing_ok() {
        let obs = ObservableDefinition::Commodity(
            CommodityObservableDef::fixing_only("BRENT", "BRENT")
                .with_fixing(FixingRequirement::new()),
        );
        assert!(obs.validate().is_ok());
    }

    #[test]
    fn test_validate_fx_empty_foreign_curve() {
        let obs = ObservableDefinition::Fx(FxObservableDef::new(
            "EURUSD",
            CurrencyPair::new(Currency::EUR, Currency::USD),
            "",
            "USD-SOFR",
        ));
        assert!(matches!(
            obs.validate(),
            Err(ObservableDefError::EmptyField("foreign_curve_name"))
        ));
    }

    #[test]
    fn test_validate_bond_empty_instrument() {
        let obs = ObservableDefinition::Bond(BondObservableDef::new(
            "UST-10Y", "USD-SOFR", "USD-Repo", "",
        ));
        assert!(matches!(
            obs.validate(),
            Err(ObservableDefError::EmptyField("bond_instrument_id"))
        ));
    }

    #[test]
    fn test_validate_all_variants_happy_path() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let cases: Vec<ObservableDefinition> = vec![
            ObservableDefinition::Cash(CashObservableDef::new("c", "c", RateIndex::Sofr)),
            ObservableDefinition::Swap(SwapObservableDef::new(
                "s",
                "c",
                "d",
                SwapIndex::UsdCms10Y,
            )),
            ObservableDefinition::Equity(EquityObservableDef::new("e", "c", "SPX")),
            ObservableDefinition::Fx(FxObservableDef::new("f", pair, "fc", "dc")),
            ObservableDefinition::Commodity(CommodityObservableDef::new("m", "WTI", "c")),
            ObservableDefinition::Inflation(InflationObservableDef::new("i", "c", "CPI")),
            ObservableDefinition::Credit(CreditObservableDef::new("cr", "c", "ACME")),
            ObservableDefinition::Bond(BondObservableDef::new("b", "d", "r", "i")),
            ObservableDefinition::BondFuture(BondFutureObservableDef::new("bf", "b", "s", "i")),
            ObservableDefinition::IrFuture(IrFutureObservableDef::new("ir", "b", "s", "i")),
        ];

        for obs in cases {
            assert!(obs.validate().is_ok(), "failed for {}", obs.observable_type());
        }
    }

    // -- Serde ---------------------------------------------------------------

    #[test]
    fn test_serde_roundtrip_cash() {
        let obs = ObservableDefinition::Cash(
            CashObservableDef::new("USD-SOFR-Cash", "USD-SOFR", RateIndex::Sofr)
                .with_fixing(FixingRequirement::new().with_source("Fed")),
        );
        let json = serde_json::to_string(&obs).unwrap();
        let parsed: ObservableDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(obs, parsed);
    }

    #[test]
    fn test_serde_roundtrip_fx() {
        let obs = ObservableDefinition::Fx(
            FxObservableDef::new(
                "EURUSD-Spot",
                CurrencyPair::new(Currency::EUR, Currency::USD),
                "EUR-ESTR",
                "USD-SOFR",
            )
            .with_fx_index(FxIndex::EcbEurUsd),
        );
        let json = serde_json::to_string(&obs).unwrap();
        let parsed: ObservableDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(obs, parsed);
    }

    #[test]
    fn test_serde_roundtrip_bond() {
        let obs = ObservableDefinition::Bond(
            BondObservableDef::new("UST-10Y", "USD-SOFR", "USD-Repo", "UST-10Y-Bond")
                .with_subtype(BondObservableSubtype::Yield),
        );
        let json = serde_json::to_string(&obs).unwrap();
        let parsed: ObservableDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(obs, parsed);
    }

    #[test]
    fn test_serde_tagged_enum() {
        let obs = ObservableDefinition::Cash(CashObservableDef::new(
            "USD-SOFR-Cash",
            "USD-SOFR",
            RateIndex::Sofr,
        ));
        let json = serde_json::to_string(&obs).unwrap();
        assert!(json.contains("\"type\":\"cash\""));
    }

    #[test]
    fn test_serde_optional_fields_omitted() {
        let obs = ObservableDefinition::Cash(CashObservableDef::new(
            "USD-SOFR-Cash",
            "USD-SOFR",
            RateIndex::Sofr,
        ));
        let json = serde_json::to_string(&obs).unwrap();
        assert!(!json.contains("fixing"));
    }

    #[test]
    fn test_serde_from_json() {
        let json = r#"{
            "type": "credit",
            "name": "ACME-Credit",
            "curveName": "ACME-Surv",
            "referenceEntity": "ACME Corp",
            "seniority": "SNRFOR"
        }"#;
        let obs: ObservableDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(obs.name(), "ACME-Credit");
        assert_eq!(obs.observable_type(), "credit");
    }

    // -- Error Display -------------------------------------------------------

    #[test]
    fn test_error_display() {
        let e = ObservableDefError::EmptyField("name");
        assert!(e.to_string().contains("name"));

        let e = ObservableDefError::CommodityRequiresCurveOrFixing("WTI".into());
        assert!(e.to_string().contains("WTI"));
    }
}
