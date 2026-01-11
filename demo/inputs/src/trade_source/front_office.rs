//! Front office trade booking simulation.
//!
//! Simulates a front office system that books trades throughout the day.

use super::{InstrumentType, TradeParams, TradeRecord, TradeSource};
use chrono::{Days, NaiveDate, Utc};
use rand::Rng;

/// Front office trade booking system
pub struct FrontOffice {
    /// List of counterparties
    counterparties: Vec<CounterpartyInfo>,
    /// List of underlyings
    underlyings: Vec<UnderlyingInfo>,
    /// Trade date
    trade_date: NaiveDate,
}

/// Counterparty information
#[derive(Clone)]
struct CounterpartyInfo {
    id: String,
    name: String,
    netting_sets: Vec<String>,
}

/// Underlying information
#[derive(Clone)]
struct UnderlyingInfo {
    ticker: String,
    spot_price: f64,
    currency: String,
}

impl FrontOffice {
    /// Create a new front office with default data
    pub fn new() -> Self {
        Self {
            counterparties: Self::default_counterparties(),
            underlyings: Self::default_underlyings(),
            trade_date: Utc::now().date_naive(),
        }
    }

    /// Set trade date
    pub fn with_trade_date(mut self, date: NaiveDate) -> Self {
        self.trade_date = date;
        self
    }

    fn default_counterparties() -> Vec<CounterpartyInfo> {
        vec![
            CounterpartyInfo {
                id: "CP001".to_string(),
                name: "Goldman Sachs".to_string(),
                netting_sets: vec!["NS001".to_string(), "NS002".to_string()],
            },
            CounterpartyInfo {
                id: "CP002".to_string(),
                name: "JP Morgan".to_string(),
                netting_sets: vec!["NS003".to_string()],
            },
            CounterpartyInfo {
                id: "CP003".to_string(),
                name: "Morgan Stanley".to_string(),
                netting_sets: vec!["NS004".to_string(), "NS005".to_string()],
            },
            CounterpartyInfo {
                id: "CP004".to_string(),
                name: "Deutsche Bank".to_string(),
                netting_sets: vec!["NS006".to_string()],
            },
            CounterpartyInfo {
                id: "CP005".to_string(),
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
            let maturity = self.trade_date.checked_add_days(Days::new(months * 30)).unwrap();

            // Notional
            let notional: f64 = rng.gen_range(1_000_000.0..50_000_000.0);

            trades.push(TradeRecord {
                trade_id: format!("EQ-OPT-{:06}", i + 1),
                instrument_type: InstrumentType::EquityOption,
                counterparty_id: cp.id.clone(),
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

        let currencies = vec!["USD", "EUR", "JPY", "GBP"];
        let indices = vec!["SOFR", "EURIBOR", "TONAR", "SONIA"];

        for i in 0..count {
            let cp = &self.counterparties[rng.gen_range(0..self.counterparties.len())];
            let ns = &cp.netting_sets[rng.gen_range(0..cp.netting_sets.len())];
            let ccy_idx = rng.gen_range(0..currencies.len());

            // Maturity 1-30 years
            let years: u64 = rng.gen_range(1..31);
            let maturity = self.trade_date.checked_add_days(Days::new(years * 365)).unwrap();

            // Fixed rate around 4%
            let fixed_rate: f64 = rng.gen_range(0.02..0.06);

            // Notional
            let notional: f64 = rng.gen_range(10_000_000.0..500_000_000.0);

            trades.push(TradeRecord {
                trade_id: format!("IRS-{:06}", i + 1),
                instrument_type: InstrumentType::InterestRateSwap,
                counterparty_id: cp.id.clone(),
                netting_set_id: ns.clone(),
                notional,
                currency: currencies[ccy_idx].to_string(),
                trade_date: self.trade_date.to_string(),
                maturity_date: maturity.to_string(),
                params: TradeParams::InterestRateSwap {
                    fixed_rate,
                    float_index: indices[ccy_idx].to_string(),
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

        let fx_pairs = vec![
            ("USD", "JPY", 150.25),
            ("EUR", "USD", 1.085),
            ("GBP", "USD", 1.265),
            ("USD", "CHF", 0.882),
            ("EUR", "JPY", 163.0),
        ];

        for i in 0..count {
            let cp = &self.counterparties[rng.gen_range(0..self.counterparties.len())];
            let ns = &cp.netting_sets[rng.gen_range(0..cp.netting_sets.len())];
            let (buy, sell, spot) = &fx_pairs[rng.gen_range(0..fx_pairs.len())];

            // Maturity 1-12 months
            let months: u64 = rng.gen_range(1..13);
            let maturity = self.trade_date.checked_add_days(Days::new(months * 30)).unwrap();

            // Forward rate with small premium/discount
            let fwd_pts: f64 = rng.gen_range(-0.02..0.02);
            let rate = spot * (1.0 + fwd_pts);

            // Notional
            let notional: f64 = rng.gen_range(1_000_000.0..100_000_000.0);

            trades.push(TradeRecord {
                trade_id: format!("FX-FWD-{:06}", i + 1),
                instrument_type: InstrumentType::FxForward,
                counterparty_id: cp.id.clone(),
                netting_set_id: ns.clone(),
                notional,
                currency: buy.to_string(),
                trade_date: self.trade_date.to_string(),
                maturity_date: maturity.to_string(),
                params: TradeParams::FxForward {
                    buy_currency: buy.to_string(),
                    sell_currency: sell.to_string(),
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

        let reference_entities = vec![
            ("FORD", 150.0),
            ("GM", 120.0),
            ("BOEING", 80.0),
            ("ATT", 100.0),
            ("VERIZON", 75.0),
        ];

        for i in 0..count {
            let cp = &self.counterparties[rng.gen_range(0..self.counterparties.len())];
            let ns = &cp.netting_sets[rng.gen_range(0..cp.netting_sets.len())];
            let (entity, base_spread) = &reference_entities[rng.gen_range(0..reference_entities.len())];

            // Standard CDS maturities
            let years: u64 = *[1, 2, 3, 5, 7, 10].iter().collect::<Vec<_>>()[rng.gen_range(0..6)];
            let maturity = self.trade_date.checked_add_days(Days::new(years * 365)).unwrap();

            // Spread with noise
            let spread: f64 = base_spread * rng.gen_range(0.8..1.2);

            // Notional
            let notional: f64 = rng.gen_range(5_000_000.0..100_000_000.0);

            trades.push(TradeRecord {
                trade_id: format!("CDS-{:06}", i + 1),
                instrument_type: InstrumentType::CreditDefaultSwap,
                counterparty_id: cp.id.clone(),
                netting_set_id: ns.clone(),
                notional,
                currency: "USD".to_string(),
                trade_date: self.trade_date.to_string(),
                maturity_date: maturity.to_string(),
                params: TradeParams::CreditDefaultSwap {
                    reference_entity: entity.to_string(),
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

        let fx_pairs = vec![
            ("USDJPY", "USD", 150.25),
            ("EURUSD", "EUR", 1.085),
            ("GBPUSD", "GBP", 1.265),
            ("USDCHF", "USD", 0.882),
            ("EURJPY", "EUR", 163.0),
        ];

        for i in 0..count {
            let cp = &self.counterparties[rng.gen_range(0..self.counterparties.len())];
            let ns = &cp.netting_sets[rng.gen_range(0..cp.netting_sets.len())];
            let (pair, base_ccy, spot) = &fx_pairs[rng.gen_range(0..fx_pairs.len())];

            // Maturity 1-12 months
            let months: u64 = rng.gen_range(1..13);
            let maturity = self.trade_date.checked_add_days(Days::new(months * 30)).unwrap();

            // Strike around spot
            let strike_pct: f64 = rng.gen_range(0.90..1.10);
            let strike = spot * strike_pct;

            // Notional
            let notional: f64 = rng.gen_range(1_000_000.0..50_000_000.0);

            trades.push(TradeRecord {
                trade_id: format!("FX-OPT-{:06}", i + 1),
                instrument_type: InstrumentType::FxOption,
                counterparty_id: cp.id.clone(),
                netting_set_id: ns.clone(),
                notional,
                currency: base_ccy.to_string(),
                trade_date: self.trade_date.to_string(),
                maturity_date: maturity.to_string(),
                params: TradeParams::FxOption {
                    currency_pair: pair.to_string(),
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
            let maturity = self.trade_date.checked_add_days(Days::new(months * 30)).unwrap();

            // Notional
            let notional: f64 = rng.gen_range(1_000_000.0..20_000_000.0);

            trades.push(TradeRecord {
                trade_id: format!("EQ-FWD-{:06}", i + 1),
                instrument_type: InstrumentType::EquityForward,
                counterparty_id: cp.id.clone(),
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
        let eq_fwd_count = count - eq_opt_count - irs_count - fx_fwd_count - cds_count - fx_opt_count;

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
        assert!(eq_options.iter().all(|t| t.instrument_type == InstrumentType::EquityOption));

        let irs = fo.generate_irs_trades(5);
        assert_eq!(irs.len(), 5);
        assert!(irs.iter().all(|t| t.instrument_type == InstrumentType::InterestRateSwap));

        let fx_fwd = fo.generate_fx_forwards(5);
        assert_eq!(fx_fwd.len(), 5);
        assert!(fx_fwd.iter().all(|t| t.instrument_type == InstrumentType::FxForward));

        let cds = fo.generate_cds_trades(5);
        assert_eq!(cds.len(), 5);
        assert!(cds.iter().all(|t| t.instrument_type == InstrumentType::CreditDefaultSwap));

        let fx_opt = fo.generate_fx_options(5);
        assert_eq!(fx_opt.len(), 5);
        assert!(fx_opt.iter().all(|t| t.instrument_type == InstrumentType::FxOption));

        let eq_fwd = fo.generate_equity_forwards(5);
        assert_eq!(eq_fwd.len(), 5);
        assert!(eq_fwd.iter().all(|t| t.instrument_type == InstrumentType::EquityForward));
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

        let eq_opt = trades.iter().filter(|t| t.instrument_type == InstrumentType::EquityOption).count();
        let irs = trades.iter().filter(|t| t.instrument_type == InstrumentType::InterestRateSwap).count();
        let fx_fwd = trades.iter().filter(|t| t.instrument_type == InstrumentType::FxForward).count();
        let cds = trades.iter().filter(|t| t.instrument_type == InstrumentType::CreditDefaultSwap).count();
        let fx_opt = trades.iter().filter(|t| t.instrument_type == InstrumentType::FxOption).count();
        let eq_fwd = trades.iter().filter(|t| t.instrument_type == InstrumentType::EquityForward).count();

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
}
