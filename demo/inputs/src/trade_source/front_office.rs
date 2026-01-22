//! Front office trade booking simulation.
//!
//! Simulates a front office system that books trades throughout the day.
//! Reference data is loaded from CSV files in the data directory.

use std::path::{Path, PathBuf};

use chrono::{Days, NaiveDate, Utc};
use rand::Rng;
use serde::Deserialize;

use super::{InstrumentType, TradeParams, TradeRecord, TradeSource};

/// Front office trade booking system
pub struct FrontOffice {
    /// List of counterparties
    counterparties: Vec<CounterpartyInfo>,
    /// List of underlyings
    underlyings: Vec<UnderlyingInfo>,
    /// List of FX pairs
    fx_pairs: Vec<FxPairInfo>,
    /// List of CDS reference entities
    cds_references: Vec<CdsReferenceInfo>,
    /// Currency to index mapping
    currency_indices: Vec<CurrencyIndexInfo>,
    /// Trade date
    trade_date: NaiveDate,
}

/// Counterparty information
#[derive(Clone, Debug, Deserialize)]
struct CounterpartyInfo {
    counterparty_id: String,
    #[allow(dead_code)]
    name: String,
    #[serde(skip)]
    netting_sets: Vec<String>,
}

/// Netting set record from CSV
#[derive(Clone, Debug, Deserialize)]
struct NettingSetRecord {
    netting_set_id: String,
    counterparty_id: String,
}

/// Underlying information
#[derive(Clone, Debug, Deserialize)]
struct UnderlyingInfo {
    ticker: String,
    spot_price: f64,
    currency: String,
}

/// FX pair information
#[derive(Clone, Debug, Deserialize)]
struct FxPairInfo {
    pair: String,
    buy_currency: String,
    sell_currency: String,
    spot_rate: f64,
}

/// CDS reference entity information
#[derive(Clone, Debug, Deserialize)]
struct CdsReferenceInfo {
    #[allow(dead_code)]
    entity_id: String,
    #[allow(dead_code)]
    name: String,
    ticker: String,
    base_spread_bps: f64,
}

/// Currency to index mapping
#[derive(Clone, Debug, Deserialize)]
struct CurrencyIndexInfo {
    currency: String,
    index: String,
}

/// Error type for FrontOffice operations
#[derive(Debug, thiserror::Error)]
pub enum FrontOfficeError {
    #[error("Failed to read CSV file: {0}")]
    CsvError(#[from] csv::Error),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("No data loaded for {0}")]
    NoData(String),
}

impl FrontOffice {
    /// Create a new front office with default embedded data (for backwards compatibility)
    pub fn new() -> Self {
        Self {
            counterparties: Self::default_counterparties(),
            underlyings: Self::default_underlyings(),
            fx_pairs: Self::default_fx_pairs(),
            cds_references: Self::default_cds_references(),
            currency_indices: Self::default_currency_indices(),
            trade_date: Utc::now().date_naive(),
        }
    }

    /// Create a new front office loading data from the specified data directory
    pub fn from_data_dir(data_dir: impl AsRef<Path>) -> Result<Self, FrontOfficeError> {
        let data_dir = data_dir.as_ref();

        let counterparties = Self::load_counterparties(data_dir)?;
        let underlyings = Self::load_underlyings(data_dir)?;
        let fx_pairs = Self::load_fx_pairs(data_dir)?;
        let cds_references = Self::load_cds_references(data_dir)?;
        let currency_indices = Self::load_currency_indices(data_dir)?;

        Ok(Self {
            counterparties,
            underlyings,
            fx_pairs,
            cds_references,
            currency_indices,
            trade_date: Utc::now().date_naive(),
        })
    }

    /// Set trade date
    pub fn with_trade_date(mut self, date: NaiveDate) -> Self {
        self.trade_date = date;
        self
    }

    // === CSV Loading Functions ===

    fn load_counterparties(data_dir: &Path) -> Result<Vec<CounterpartyInfo>, FrontOfficeError> {
        let cp_path = data_dir.join("input/counterparties/counterparties.csv");
        let ns_path = data_dir.join("input/counterparties/netting_sets.csv");

        // Load counterparties
        let mut counterparties: Vec<CounterpartyInfo> = if cp_path.exists() {
            let mut rdr = csv::Reader::from_path(&cp_path)?;
            rdr.deserialize().collect::<Result<Vec<_>, _>>()?
        } else {
            return Ok(Self::default_counterparties());
        };

        // Load netting sets and associate with counterparties
        if ns_path.exists() {
            let mut rdr = csv::Reader::from_path(&ns_path)?;
            let netting_sets: Vec<NettingSetRecord> =
                rdr.deserialize().collect::<Result<Vec<_>, _>>()?;

            for cp in &mut counterparties {
                cp.netting_sets = netting_sets
                    .iter()
                    .filter(|ns| ns.counterparty_id == cp.counterparty_id)
                    .map(|ns| ns.netting_set_id.clone())
                    .collect();
            }
        }

        // Filter out counterparties without netting sets
        counterparties.retain(|cp| !cp.netting_sets.is_empty());

        if counterparties.is_empty() {
            return Ok(Self::default_counterparties());
        }

        Ok(counterparties)
    }

    fn load_underlyings(data_dir: &Path) -> Result<Vec<UnderlyingInfo>, FrontOfficeError> {
        let path = data_dir.join("input/market_data/reference/underlyings.csv");
        if path.exists() {
            let mut rdr = csv::Reader::from_path(&path)?;
            let underlyings: Vec<UnderlyingInfo> =
                rdr.deserialize().collect::<Result<Vec<_>, _>>()?;
            if !underlyings.is_empty() {
                return Ok(underlyings);
            }
        }
        Ok(Self::default_underlyings())
    }

    fn load_fx_pairs(data_dir: &Path) -> Result<Vec<FxPairInfo>, FrontOfficeError> {
        let path = data_dir.join("input/market_data/reference/fx_pairs.csv");
        if path.exists() {
            let mut rdr = csv::Reader::from_path(&path)?;
            let fx_pairs: Vec<FxPairInfo> = rdr.deserialize().collect::<Result<Vec<_>, _>>()?;
            if !fx_pairs.is_empty() {
                return Ok(fx_pairs);
            }
        }
        Ok(Self::default_fx_pairs())
    }

    fn load_cds_references(data_dir: &Path) -> Result<Vec<CdsReferenceInfo>, FrontOfficeError> {
        let path = data_dir.join("input/market_data/reference/cds_reference_entities.csv");
        if path.exists() {
            let mut rdr = csv::Reader::from_path(&path)?;
            let refs: Vec<CdsReferenceInfo> = rdr.deserialize().collect::<Result<Vec<_>, _>>()?;
            if !refs.is_empty() {
                return Ok(refs);
            }
        }
        Ok(Self::default_cds_references())
    }

    fn load_currency_indices(data_dir: &Path) -> Result<Vec<CurrencyIndexInfo>, FrontOfficeError> {
        let path = data_dir.join("input/market_data/reference/currency_indices.csv");
        if path.exists() {
            let mut rdr = csv::Reader::from_path(&path)?;
            let indices: Vec<CurrencyIndexInfo> =
                rdr.deserialize().collect::<Result<Vec<_>, _>>()?;
            if !indices.is_empty() {
                return Ok(indices);
            }
        }
        Ok(Self::default_currency_indices())
    }

    // === Default Data (fallback when CSVs not available) ===

    fn default_counterparties() -> Vec<CounterpartyInfo> {
        vec![
            CounterpartyInfo {
                counterparty_id: "CP001".to_string(),
                name: "Goldman Sachs".to_string(),
                netting_sets: vec!["NS001".to_string(), "NS002".to_string()],
            },
            CounterpartyInfo {
                counterparty_id: "CP002".to_string(),
                name: "JP Morgan".to_string(),
                netting_sets: vec!["NS003".to_string()],
            },
            CounterpartyInfo {
                counterparty_id: "CP003".to_string(),
                name: "Morgan Stanley".to_string(),
                netting_sets: vec!["NS004".to_string(), "NS005".to_string()],
            },
            CounterpartyInfo {
                counterparty_id: "CP004".to_string(),
                name: "Deutsche Bank".to_string(),
                netting_sets: vec!["NS006".to_string()],
            },
            CounterpartyInfo {
                counterparty_id: "CP005".to_string(),
                name: "BNP Paribas".to_string(),
                netting_sets: vec!["NS007".to_string()],
            },
        ]
    }

    fn default_underlyings() -> Vec<UnderlyingInfo> {
        vec![
            UnderlyingInfo {
                ticker: "AAPL".to_string(),
                spot_price: 185.0,
                currency: "USD".to_string(),
            },
            UnderlyingInfo {
                ticker: "GOOGL".to_string(),
                spot_price: 140.0,
                currency: "USD".to_string(),
            },
            UnderlyingInfo {
                ticker: "MSFT".to_string(),
                spot_price: 380.0,
                currency: "USD".to_string(),
            },
            UnderlyingInfo {
                ticker: "7203.T".to_string(),
                spot_price: 2800.0,
                currency: "JPY".to_string(),
            },
            UnderlyingInfo {
                ticker: "DBK.DE".to_string(),
                spot_price: 15.50,
                currency: "EUR".to_string(),
            },
        ]
    }

    fn default_fx_pairs() -> Vec<FxPairInfo> {
        vec![
            FxPairInfo {
                pair: "USDJPY".to_string(),
                buy_currency: "USD".to_string(),
                sell_currency: "JPY".to_string(),
                spot_rate: 150.25,
            },
            FxPairInfo {
                pair: "EURUSD".to_string(),
                buy_currency: "EUR".to_string(),
                sell_currency: "USD".to_string(),
                spot_rate: 1.085,
            },
            FxPairInfo {
                pair: "GBPUSD".to_string(),
                buy_currency: "GBP".to_string(),
                sell_currency: "USD".to_string(),
                spot_rate: 1.265,
            },
            FxPairInfo {
                pair: "USDCHF".to_string(),
                buy_currency: "USD".to_string(),
                sell_currency: "CHF".to_string(),
                spot_rate: 0.882,
            },
            FxPairInfo {
                pair: "EURJPY".to_string(),
                buy_currency: "EUR".to_string(),
                sell_currency: "JPY".to_string(),
                spot_rate: 163.0,
            },
        ]
    }

    fn default_cds_references() -> Vec<CdsReferenceInfo> {
        vec![
            CdsReferenceInfo {
                entity_id: "REF001".to_string(),
                name: "Ford Motor Company".to_string(),
                ticker: "FORD".to_string(),
                base_spread_bps: 150.0,
            },
            CdsReferenceInfo {
                entity_id: "REF002".to_string(),
                name: "General Motors Company".to_string(),
                ticker: "GM".to_string(),
                base_spread_bps: 120.0,
            },
            CdsReferenceInfo {
                entity_id: "REF003".to_string(),
                name: "Boeing Company".to_string(),
                ticker: "BOEING".to_string(),
                base_spread_bps: 80.0,
            },
            CdsReferenceInfo {
                entity_id: "REF004".to_string(),
                name: "AT&T Inc.".to_string(),
                ticker: "ATT".to_string(),
                base_spread_bps: 100.0,
            },
            CdsReferenceInfo {
                entity_id: "REF005".to_string(),
                name: "Verizon Communications".to_string(),
                ticker: "VERIZON".to_string(),
                base_spread_bps: 75.0,
            },
        ]
    }

    fn default_currency_indices() -> Vec<CurrencyIndexInfo> {
        vec![
            CurrencyIndexInfo {
                currency: "USD".to_string(),
                index: "SOFR".to_string(),
            },
            CurrencyIndexInfo {
                currency: "EUR".to_string(),
                index: "EURIBOR".to_string(),
            },
            CurrencyIndexInfo {
                currency: "JPY".to_string(),
                index: "TONAR".to_string(),
            },
            CurrencyIndexInfo {
                currency: "GBP".to_string(),
                index: "SONIA".to_string(),
            },
        ]
    }

    // === Helper Functions ===

    fn get_index_for_currency(&self, currency: &str) -> String {
        self.currency_indices
            .iter()
            .find(|ci| ci.currency == currency)
            .map(|ci| ci.index.clone())
            .unwrap_or_else(|| "SOFR".to_string())
    }

    /// Get data directory path
    pub fn data_dir() -> PathBuf {
        PathBuf::from("demo/data")
    }

    // === Trade Generation Functions ===

    /// Generate equity option trades
    pub fn generate_equity_options(&self, count: usize) -> Vec<TradeRecord> {
        let mut rng = rand::thread_rng();
        let mut trades = Vec::with_capacity(count);

        for i in 0..count {
            let cp = &self.counterparties[rng.gen_range(0..self.counterparties.len())];
            let underlying = &self.underlyings[rng.gen_range(0..self.underlyings.len())];
            let ns = &cp.netting_sets[rng.gen_range(0..cp.netting_sets.len())];

            // Strike around spot
            let strike_pct: f64 = rng.gen_range(0.85..1.15);
            let strike = underlying.spot_price * strike_pct;

            // Maturity 1-24 months
            let months: u64 = rng.gen_range(1..25);
            let maturity = self
                .trade_date
                .checked_add_days(Days::new(months * 30))
                .unwrap();

            // Notional
            let notional: f64 = rng.gen_range(1_000_000.0..50_000_000.0);

            trades.push(TradeRecord {
                trade_id: format!("EQ-OPT-{:06}", i + 1),
                instrument_type: InstrumentType::EquityOption,
                counterparty_id: cp.counterparty_id.clone(),
                netting_set_id: ns.clone(),
                notional,
                currency: underlying.currency.clone(),
                trade_date: self.trade_date.to_string(),
                maturity_date: maturity.to_string(),
                params: TradeParams::EquityOption {
                    underlying: underlying.ticker.clone(),
                    strike,
                    is_call: rng.gen_bool(0.5),
                },
            });
        }

        trades
    }

    /// Generate interest rate swap trades
    pub fn generate_irs_trades(&self, count: usize) -> Vec<TradeRecord> {
        let mut rng = rand::thread_rng();
        let mut trades = Vec::with_capacity(count);

        // Get unique currencies from currency_indices
        let currencies: Vec<&str> = self
            .currency_indices
            .iter()
            .map(|ci| ci.currency.as_str())
            .collect();

        for i in 0..count {
            let cp = &self.counterparties[rng.gen_range(0..self.counterparties.len())];
            let ns = &cp.netting_sets[rng.gen_range(0..cp.netting_sets.len())];
            let currency = currencies[rng.gen_range(0..currencies.len())];
            let float_index = self.get_index_for_currency(currency);

            // Maturity 1-30 years
            let years: u64 = rng.gen_range(1..31);
            let maturity = self
                .trade_date
                .checked_add_days(Days::new(years * 365))
                .unwrap();

            // Fixed rate around 4%
            let fixed_rate: f64 = rng.gen_range(0.02..0.06);

            // Notional
            let notional: f64 = rng.gen_range(10_000_000.0..500_000_000.0);

            trades.push(TradeRecord {
                trade_id: format!("IRS-{:06}", i + 1),
                instrument_type: InstrumentType::InterestRateSwap,
                counterparty_id: cp.counterparty_id.clone(),
                netting_set_id: ns.clone(),
                notional,
                currency: currency.to_string(),
                trade_date: self.trade_date.to_string(),
                maturity_date: maturity.to_string(),
                params: TradeParams::InterestRateSwap {
                    fixed_rate,
                    float_index,
                    pay_fixed: rng.gen_bool(0.5),
                },
            });
        }

        trades
    }

    /// Generate FX forward trades
    pub fn generate_fx_forwards(&self, count: usize) -> Vec<TradeRecord> {
        let mut rng = rand::thread_rng();
        let mut trades = Vec::with_capacity(count);

        for i in 0..count {
            let cp = &self.counterparties[rng.gen_range(0..self.counterparties.len())];
            let ns = &cp.netting_sets[rng.gen_range(0..cp.netting_sets.len())];
            let fx_pair = &self.fx_pairs[rng.gen_range(0..self.fx_pairs.len())];

            // Maturity 1-12 months
            let months: u64 = rng.gen_range(1..13);
            let maturity = self
                .trade_date
                .checked_add_days(Days::new(months * 30))
                .unwrap();

            // Forward rate with small premium/discount
            let fwd_pts: f64 = rng.gen_range(-0.02..0.02);
            let rate = fx_pair.spot_rate * (1.0 + fwd_pts);

            // Notional
            let notional: f64 = rng.gen_range(1_000_000.0..100_000_000.0);

            trades.push(TradeRecord {
                trade_id: format!("FX-FWD-{:06}", i + 1),
                instrument_type: InstrumentType::FxForward,
                counterparty_id: cp.counterparty_id.clone(),
                netting_set_id: ns.clone(),
                notional,
                currency: fx_pair.buy_currency.clone(),
                trade_date: self.trade_date.to_string(),
                maturity_date: maturity.to_string(),
                params: TradeParams::FxForward {
                    buy_currency: fx_pair.buy_currency.clone(),
                    sell_currency: fx_pair.sell_currency.clone(),
                    rate,
                },
            });
        }

        trades
    }

    /// Generate CDS trades
    pub fn generate_cds_trades(&self, count: usize) -> Vec<TradeRecord> {
        let mut rng = rand::thread_rng();
        let mut trades = Vec::with_capacity(count);

        for i in 0..count {
            let cp = &self.counterparties[rng.gen_range(0..self.counterparties.len())];
            let ns = &cp.netting_sets[rng.gen_range(0..cp.netting_sets.len())];
            let cds_ref = &self.cds_references[rng.gen_range(0..self.cds_references.len())];

            // Standard CDS maturities
            let years: u64 = *[1, 2, 3, 5, 7, 10].iter().collect::<Vec<_>>()[rng.gen_range(0..6)];
            let maturity = self
                .trade_date
                .checked_add_days(Days::new(years * 365))
                .unwrap();

            // Spread with noise
            let spread: f64 = cds_ref.base_spread_bps * rng.gen_range(0.8..1.2);

            // Notional
            let notional: f64 = rng.gen_range(5_000_000.0..100_000_000.0);

            trades.push(TradeRecord {
                trade_id: format!("CDS-{:06}", i + 1),
                instrument_type: InstrumentType::CreditDefaultSwap,
                counterparty_id: cp.counterparty_id.clone(),
                netting_set_id: ns.clone(),
                notional,
                currency: "USD".to_string(),
                trade_date: self.trade_date.to_string(),
                maturity_date: maturity.to_string(),
                params: TradeParams::CreditDefaultSwap {
                    reference_entity: cds_ref.ticker.clone(),
                    spread_bps: spread,
                    is_protection_buyer: rng.gen_bool(0.5),
                },
            });
        }

        trades
    }

    /// Generate FX option trades
    pub fn generate_fx_options(&self, count: usize) -> Vec<TradeRecord> {
        let mut rng = rand::thread_rng();
        let mut trades = Vec::with_capacity(count);

        for i in 0..count {
            let cp = &self.counterparties[rng.gen_range(0..self.counterparties.len())];
            let ns = &cp.netting_sets[rng.gen_range(0..cp.netting_sets.len())];
            let fx_pair = &self.fx_pairs[rng.gen_range(0..self.fx_pairs.len())];

            // Maturity 1-12 months
            let months: u64 = rng.gen_range(1..13);
            let maturity = self
                .trade_date
                .checked_add_days(Days::new(months * 30))
                .unwrap();

            // Strike around spot
            let strike_pct: f64 = rng.gen_range(0.90..1.10);
            let strike = fx_pair.spot_rate * strike_pct;

            // Notional
            let notional: f64 = rng.gen_range(1_000_000.0..50_000_000.0);

            trades.push(TradeRecord {
                trade_id: format!("FX-OPT-{:06}", i + 1),
                instrument_type: InstrumentType::FxOption,
                counterparty_id: cp.counterparty_id.clone(),
                netting_set_id: ns.clone(),
                notional,
                currency: fx_pair.buy_currency.clone(),
                trade_date: self.trade_date.to_string(),
                maturity_date: maturity.to_string(),
                params: TradeParams::FxOption {
                    currency_pair: fx_pair.pair.clone(),
                    strike,
                    is_call: rng.gen_bool(0.5),
                },
            });
        }

        trades
    }

    /// Generate equity forward trades
    pub fn generate_equity_forwards(&self, count: usize) -> Vec<TradeRecord> {
        let mut rng = rand::thread_rng();
        let mut trades = Vec::with_capacity(count);

        for i in 0..count {
            let cp = &self.counterparties[rng.gen_range(0..self.counterparties.len())];
            let underlying = &self.underlyings[rng.gen_range(0..self.underlyings.len())];
            let ns = &cp.netting_sets[rng.gen_range(0..cp.netting_sets.len())];

            // Forward price with cost of carry
            let fwd_adj: f64 = rng.gen_range(0.98..1.05);
            let forward_price = underlying.spot_price * fwd_adj;

            // Maturity 1-12 months
            let months: u64 = rng.gen_range(1..13);
            let maturity = self
                .trade_date
                .checked_add_days(Days::new(months * 30))
                .unwrap();

            // Notional
            let notional: f64 = rng.gen_range(1_000_000.0..20_000_000.0);

            trades.push(TradeRecord {
                trade_id: format!("EQ-FWD-{:06}", i + 1),
                instrument_type: InstrumentType::EquityForward,
                counterparty_id: cp.counterparty_id.clone(),
                netting_set_id: ns.clone(),
                notional,
                currency: underlying.currency.clone(),
                trade_date: self.trade_date.to_string(),
                maturity_date: maturity.to_string(),
                params: TradeParams::Forward {
                    underlying: underlying.ticker.clone(),
                    forward_price,
                },
            });
        }

        trades
    }

    /// Generate a single random trade (for streaming scenarios)
    pub fn generate_single_trade(&self) -> TradeRecord {
        let mut rng = rand::thread_rng();
        let trade_type = rng.gen_range(0..6);

        match trade_type {
            0 => self.generate_equity_options(1).pop().unwrap(),
            1 => self.generate_irs_trades(1).pop().unwrap(),
            2 => self.generate_fx_forwards(1).pop().unwrap(),
            3 => self.generate_cds_trades(1).pop().unwrap(),
            4 => self.generate_fx_options(1).pop().unwrap(),
            _ => self.generate_equity_forwards(1).pop().unwrap(),
        }
    }
}

impl Default for FrontOffice {
    fn default() -> Self {
        Self::new()
    }
}

impl TradeSource for FrontOffice {
    fn generate_trades(&self, count: usize) -> Vec<TradeRecord> {
        let mut rng = rand::thread_rng();
        let mut trades = Vec::new();

        // Distribute trades across instrument types (6 types now)
        let eq_opt_count = count * 20 / 100;
        let irs_count = count * 20 / 100;
        let fx_fwd_count = count * 15 / 100;
        let cds_count = count * 15 / 100;
        let fx_opt_count = count * 15 / 100;
        let eq_fwd_count =
            count - eq_opt_count - irs_count - fx_fwd_count - cds_count - fx_opt_count;

        trades.extend(self.generate_equity_options(eq_opt_count));
        trades.extend(self.generate_irs_trades(irs_count));
        trades.extend(self.generate_fx_forwards(fx_fwd_count));
        trades.extend(self.generate_cds_trades(cds_count));
        trades.extend(self.generate_fx_options(fx_opt_count));
        trades.extend(self.generate_equity_forwards(eq_fwd_count));

        // Shuffle
        use rand::seq::SliceRandom;
        trades.shuffle(&mut rng);

        trades
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_front_office_generate() {
        let fo = FrontOffice::new();
        let trades = fo.generate_trades(100);
        assert_eq!(trades.len(), 100);
    }

    #[test]
    fn test_generate_all_trade_types() {
        let fo = FrontOffice::new();

        let eq_options = fo.generate_equity_options(5);
        assert_eq!(eq_options.len(), 5);
        assert!(eq_options
            .iter()
            .all(|t| t.instrument_type == InstrumentType::EquityOption));

        let irs = fo.generate_irs_trades(5);
        assert_eq!(irs.len(), 5);
        assert!(irs
            .iter()
            .all(|t| t.instrument_type == InstrumentType::InterestRateSwap));

        let fx_fwd = fo.generate_fx_forwards(5);
        assert_eq!(fx_fwd.len(), 5);
        assert!(fx_fwd
            .iter()
            .all(|t| t.instrument_type == InstrumentType::FxForward));

        let cds = fo.generate_cds_trades(5);
        assert_eq!(cds.len(), 5);
        assert!(cds
            .iter()
            .all(|t| t.instrument_type == InstrumentType::CreditDefaultSwap));

        let fx_opt = fo.generate_fx_options(5);
        assert_eq!(fx_opt.len(), 5);
        assert!(fx_opt
            .iter()
            .all(|t| t.instrument_type == InstrumentType::FxOption));

        let eq_fwd = fo.generate_equity_forwards(5);
        assert_eq!(eq_fwd.len(), 5);
        assert!(eq_fwd
            .iter()
            .all(|t| t.instrument_type == InstrumentType::EquityForward));
    }

    #[test]
    fn test_generate_single_trade() {
        let fo = FrontOffice::new();

        // Generate multiple single trades to cover all types
        for _ in 0..20 {
            let trade = fo.generate_single_trade();
            assert!(!trade.trade_id.is_empty());
            assert!(!trade.counterparty_id.is_empty());
            assert!(trade.notional > 0.0);
        }
    }

    #[test]
    fn test_trade_distribution() {
        let fo = FrontOffice::new();
        let trades = fo.generate_trades(100);

        let eq_opt = trades
            .iter()
            .filter(|t| t.instrument_type == InstrumentType::EquityOption)
            .count();
        let irs = trades
            .iter()
            .filter(|t| t.instrument_type == InstrumentType::InterestRateSwap)
            .count();
        let fx_fwd = trades
            .iter()
            .filter(|t| t.instrument_type == InstrumentType::FxForward)
            .count();
        let cds = trades
            .iter()
            .filter(|t| t.instrument_type == InstrumentType::CreditDefaultSwap)
            .count();
        let fx_opt = trades
            .iter()
            .filter(|t| t.instrument_type == InstrumentType::FxOption)
            .count();
        let eq_fwd = trades
            .iter()
            .filter(|t| t.instrument_type == InstrumentType::EquityForward)
            .count();

        // Verify all 6 types are present
        assert!(eq_opt > 0, "Should have equity options");
        assert!(irs > 0, "Should have IRS");
        assert!(fx_fwd > 0, "Should have FX forwards");
        assert!(cds > 0, "Should have CDS");
        assert!(fx_opt > 0, "Should have FX options");
        assert!(eq_fwd > 0, "Should have equity forwards");

        // Total should match
        assert_eq!(eq_opt + irs + fx_fwd + cds + fx_opt + eq_fwd, 100);
    }

    #[test]
    fn test_from_data_dir() {
        // Test that from_data_dir gracefully falls back to defaults
        // when CSV files don't exist
        let result = FrontOffice::from_data_dir("nonexistent/path");
        assert!(result.is_ok());

        let fo = result.unwrap();
        let trades = fo.generate_trades(10);
        assert_eq!(trades.len(), 10);
    }
}
