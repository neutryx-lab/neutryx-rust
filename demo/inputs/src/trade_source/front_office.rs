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

        // Distribute trades across instrument types
        let eq_opt_count = count * 30 / 100;
        let irs_count = count * 30 / 100;
        let fx_count = count * 25 / 100;
        let cds_count = count - eq_opt_count - irs_count - fx_count;

        trades.extend(self.generate_equity_options(eq_opt_count));
        trades.extend(self.generate_irs_trades(irs_count));
        trades.extend(self.generate_fx_forwards(fx_count));
        trades.extend(self.generate_cds_trades(cds_count));

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
}
