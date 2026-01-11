//! CSV file generator.
//!
//! Generates CSV files for trades, market data, and counterparty information.

use super::FileGenerator;
use crate::trade_source::{InstrumentType, TradeParams, TradeRecord};
use std::fs;
use std::io::{self, Write};
use std::path::Path;

/// CSV file generator
pub struct CsvGenerator;

/// Bulk data generator for creating complete sample datasets.
pub struct BulkDataGenerator {
    /// Output directory path
    output_dir: std::path::PathBuf,
}

impl BulkDataGenerator {
    /// Create a new bulk data generator with specified output directory.
    pub fn new(output_dir: impl AsRef<Path>) -> Self {
        Self {
            output_dir: output_dir.as_ref().to_path_buf(),
        }
    }

    /// Generate all sample data files.
    ///
    /// Creates the following directory structure:
    /// ```text
    /// output_dir/
    /// ├── counterparties/
    /// │   ├── counterparties.csv
    /// │   └── netting_sets.csv
    /// ├── market_data/
    /// │   ├── spot_rates.csv
    /// │   ├── credit_spreads.csv
    /// │   └── currencies.csv
    /// ├── trades/
    /// │   └── trades.csv
    /// └── config/
    ///     ├── csa_agreements.csv
    ///     └── holidays.csv
    /// ```
    pub fn generate_all(&self, trades: &[TradeRecord]) -> io::Result<GenerationSummary> {
        let mut summary = GenerationSummary::default();

        // Create directories
        let cp_dir = self.output_dir.join("counterparties");
        let md_dir = self.output_dir.join("market_data");
        let trade_dir = self.output_dir.join("trades");
        let config_dir = self.output_dir.join("config");

        fs::create_dir_all(&cp_dir)?;
        fs::create_dir_all(&md_dir)?;
        fs::create_dir_all(&trade_dir)?;
        fs::create_dir_all(&config_dir)?;

        // Counterparty data
        Self::write_file(&cp_dir.join("counterparties.csv"), &CsvGenerator::counterparties_csv())?;
        summary.files_created += 1;

        Self::write_file(&cp_dir.join("netting_sets.csv"), &CsvGenerator::netting_sets_csv())?;
        summary.files_created += 1;

        // Market data
        Self::write_file(&md_dir.join("spot_rates.csv"), &CsvGenerator::spot_rates_csv())?;
        summary.files_created += 1;

        Self::write_file(&md_dir.join("credit_spreads.csv"), &CsvGenerator::credit_spreads_csv())?;
        summary.files_created += 1;

        Self::write_file(&md_dir.join("currencies.csv"), &CsvGenerator::currencies_csv())?;
        summary.files_created += 1;

        // Trades
        if !trades.is_empty() {
            Self::write_file(&trade_dir.join("trades.csv"), &CsvGenerator::trades_to_csv(trades))?;
            summary.files_created += 1;
            summary.trades_written = trades.len();
        }

        // Config
        Self::write_file(&config_dir.join("csa_agreements.csv"), &CsvGenerator::csa_agreements_csv())?;
        summary.files_created += 1;

        Self::write_file(&config_dir.join("holidays.csv"), &CsvGenerator::holidays_csv())?;
        summary.files_created += 1;

        Ok(summary)
    }

    /// Generate counterparty files only.
    pub fn generate_counterparty_files(&self) -> io::Result<usize> {
        let cp_dir = self.output_dir.join("counterparties");
        fs::create_dir_all(&cp_dir)?;

        Self::write_file(&cp_dir.join("counterparties.csv"), &CsvGenerator::counterparties_csv())?;
        Self::write_file(&cp_dir.join("netting_sets.csv"), &CsvGenerator::netting_sets_csv())?;

        Ok(2)
    }

    /// Generate market data files only.
    pub fn generate_market_data_files(&self, yield_curves: &[(&str, Vec<(f64, f64)>)]) -> io::Result<usize> {
        let md_dir = self.output_dir.join("market_data");
        fs::create_dir_all(&md_dir)?;

        Self::write_file(&md_dir.join("spot_rates.csv"), &CsvGenerator::spot_rates_csv())?;
        Self::write_file(&md_dir.join("credit_spreads.csv"), &CsvGenerator::credit_spreads_csv())?;
        Self::write_file(&md_dir.join("currencies.csv"), &CsvGenerator::currencies_csv())?;

        let mut count = 3;

        // Write yield curves
        for (currency, rates) in yield_curves {
            let filename = format!("yield_curve_{}.csv", currency.to_lowercase());
            Self::write_file(&md_dir.join(&filename), &CsvGenerator::yield_curve_csv(currency, rates))?;
            count += 1;
        }

        Ok(count)
    }

    /// Generate trade files only.
    pub fn generate_trade_files(&self, trades: &[TradeRecord]) -> io::Result<usize> {
        let trade_dir = self.output_dir.join("trades");
        fs::create_dir_all(&trade_dir)?;

        Self::write_file(&trade_dir.join("trades.csv"), &CsvGenerator::trades_to_csv(trades))?;

        Ok(1)
    }

    fn write_file(path: &Path, content: &str) -> io::Result<()> {
        let mut file = fs::File::create(path)?;
        file.write_all(content.as_bytes())?;
        Ok(())
    }
}

/// Summary of bulk generation operation.
#[derive(Debug, Default)]
pub struct GenerationSummary {
    /// Number of files created
    pub files_created: usize,
    /// Number of trades written
    pub trades_written: usize,
}

impl CsvGenerator {
    /// Generate trades CSV from trade records
    pub fn trades_to_csv(trades: &[TradeRecord]) -> String {
        let mut csv = String::from("trade_id,instrument_type,counterparty_id,netting_set_id,notional,currency,trade_date,maturity_date,param1,param2,param3\n");

        for trade in trades {
            let (p1, p2, p3) = match &trade.params {
                TradeParams::EquityOption { underlying, strike, is_call } => {
                    (underlying.clone(), format!("{:.4}", strike), if *is_call { "CALL" } else { "PUT" }.to_string())
                }
                TradeParams::Forward { underlying, forward_price } => {
                    (underlying.clone(), format!("{:.4}", forward_price), String::new())
                }
                TradeParams::InterestRateSwap { fixed_rate, float_index, pay_fixed } => {
                    (format!("{:.6}", fixed_rate), float_index.clone(), if *pay_fixed { "PAY" } else { "RCV" }.to_string())
                }
                TradeParams::FxForward { buy_currency, sell_currency, rate } => {
                    (buy_currency.clone(), sell_currency.clone(), format!("{:.6}", rate))
                }
                TradeParams::FxOption { currency_pair, strike, is_call } => {
                    (currency_pair.clone(), format!("{:.4}", strike), if *is_call { "CALL" } else { "PUT" }.to_string())
                }
                TradeParams::CreditDefaultSwap { reference_entity, spread_bps, is_protection_buyer } => {
                    (reference_entity.clone(), format!("{:.2}", spread_bps), if *is_protection_buyer { "BUY" } else { "SELL" }.to_string())
                }
            };

            csv.push_str(&format!(
                "{},{:?},{},{},{:.2},{},{},{},{},{},{}\n",
                trade.trade_id,
                trade.instrument_type,
                trade.counterparty_id,
                trade.netting_set_id,
                trade.notional,
                trade.currency,
                trade.trade_date,
                trade.maturity_date,
                p1,
                p2,
                p3
            ));
        }

        csv
    }

    /// Generate counterparties CSV
    pub fn counterparties_csv() -> String {
        r#"counterparty_id,name,rating,sector,country,credit_spread_bps,recovery_rate
CP001,Goldman Sachs,AA-,Financial,US,45,0.40
CP002,JP Morgan,A+,Financial,US,50,0.40
CP003,Morgan Stanley,A,Financial,US,55,0.40
CP004,Deutsche Bank,BBB+,Financial,DE,85,0.40
CP005,BNP Paribas,A+,Financial,FR,60,0.40
CP006,Toyota Motor,A+,Auto,JP,40,0.35
CP007,Apple Inc,AA+,Tech,US,25,0.50
CP008,Berkshire Hathaway,AA,Diversified,US,30,0.45
"#.to_string()
    }

    /// Generate netting sets CSV
    pub fn netting_sets_csv() -> String {
        r#"netting_set_id,counterparty_id,agreement_type,threshold,mta,margin_period_of_risk
NS001,CP001,CSA,10000000,500000,10
NS002,CP001,CSA,5000000,250000,10
NS003,CP002,CSA,15000000,1000000,10
NS004,CP003,CSA,10000000,500000,10
NS005,CP003,ISDA,0,0,10
NS006,CP004,CSA,8000000,400000,10
NS007,CP005,CSA,12000000,600000,10
"#.to_string()
    }

    /// Generate CSA agreements CSV
    pub fn csa_agreements_csv() -> String {
        r#"csa_id,netting_set_id,collateral_currency,eligible_collateral,haircut,rounding
CSA001,NS001,USD,CASH;GOVT,0.02,10000
CSA002,NS002,USD,CASH,0.00,10000
CSA003,NS003,USD,CASH;GOVT;CORP,0.05,50000
CSA004,NS004,USD,CASH;GOVT,0.02,10000
CSA005,NS006,EUR,CASH;GOVT,0.02,10000
CSA006,NS007,EUR,CASH,0.00,10000
"#.to_string()
    }

    /// Generate yield curve CSV
    pub fn yield_curve_csv(currency: &str, rates: &[(f64, f64)]) -> String {
        let mut csv = String::from("currency,tenor_years,rate_pct\n");
        for (tenor, rate) in rates {
            csv.push_str(&format!("{},{:.4},{:.4}\n", currency, tenor, rate));
        }
        csv
    }

    /// Generate spot rates CSV
    pub fn spot_rates_csv() -> String {
        r#"ticker,spot_price,currency,timestamp
AAPL,185.25,USD,2026-01-09T10:00:00Z
GOOGL,140.50,USD,2026-01-09T10:00:00Z
MSFT,380.75,USD,2026-01-09T10:00:00Z
7203.T,2815.00,JPY,2026-01-09T10:00:00Z
DBK.DE,15.45,EUR,2026-01-09T10:00:00Z
VOW3.DE,98.50,EUR,2026-01-09T10:00:00Z
HSBA.L,625.40,GBP,2026-01-09T10:00:00Z
"#.to_string()
    }

    /// Generate volatility surface CSV
    pub fn volatility_surface_csv(ticker: &str, surface: &[(f64, f64, f64)]) -> String {
        let mut csv = String::from("ticker,strike_pct,expiry_years,implied_vol\n");
        for (strike, expiry, vol) in surface {
            csv.push_str(&format!("{},{:.2},{:.4},{:.4}\n", ticker, strike, expiry, vol));
        }
        csv
    }

    /// Generate credit spreads CSV
    pub fn credit_spreads_csv() -> String {
        r#"reference_entity,rating,tenor_years,spread_bps
FORD,BB+,1,125
FORD,BB+,3,145
FORD,BB+,5,160
FORD,BB+,10,185
GM,BBB-,1,95
GM,BBB-,3,115
GM,BBB-,5,130
GM,BBB-,10,155
BOEING,BBB,1,70
BOEING,BBB,3,85
BOEING,BBB,5,95
BOEING,BBB,10,115
ATT,BBB,1,85
ATT,BBB,3,100
ATT,BBB,5,115
ATT,BBB,10,140
"#.to_string()
    }

    /// Generate holidays CSV for a calendar
    pub fn holidays_csv() -> String {
        r#"calendar_id,date,name
TARGET,2026-01-01,New Year's Day
TARGET,2026-04-03,Good Friday
TARGET,2026-04-06,Easter Monday
TARGET,2026-05-01,Labour Day
TARGET,2026-12-25,Christmas Day
TARGET,2026-12-26,Boxing Day
USNY,2026-01-01,New Year's Day
USNY,2026-01-19,MLK Day
USNY,2026-02-16,Presidents Day
USNY,2026-05-25,Memorial Day
USNY,2026-07-03,Independence Day (Observed)
USNY,2026-09-07,Labor Day
USNY,2026-11-26,Thanksgiving
USNY,2026-12-25,Christmas Day
JP,2026-01-01,New Year's Day
JP,2026-01-02,Bank Holiday
JP,2026-01-03,Bank Holiday
JP,2026-01-12,Coming of Age Day
JP,2026-02-11,National Foundation Day
JP,2026-02-23,Emperor's Birthday
JP,2026-03-20,Vernal Equinox Day
JP,2026-04-29,Showa Day
JP,2026-05-03,Constitution Memorial Day
JP,2026-05-04,Greenery Day
JP,2026-05-05,Children's Day
JP,2026-07-20,Marine Day
JP,2026-08-11,Mountain Day
JP,2026-09-21,Respect for the Aged Day
JP,2026-09-22,Autumnal Equinox Day
JP,2026-10-12,Sports Day
JP,2026-11-03,Culture Day
JP,2026-11-23,Labor Thanksgiving Day
"#.to_string()
    }

    /// Generate currencies master CSV
    pub fn currencies_csv() -> String {
        r#"code,name,symbol,decimal_places,calendar_id
USD,US Dollar,$,2,USNY
EUR,Euro,€,2,TARGET
JPY,Japanese Yen,¥,0,JP
GBP,British Pound,£,2,GBLO
CHF,Swiss Franc,CHF,2,CHZU
AUD,Australian Dollar,A$,2,AUSY
CAD,Canadian Dollar,C$,2,CATO
CNY,Chinese Yuan,¥,2,CNSH
HKD,Hong Kong Dollar,HK$,2,HKHK
SGD,Singapore Dollar,S$,2,SGSG
"#.to_string()
    }
}

impl FileGenerator for CsvGenerator {
    fn generate(&self) -> String {
        // Default: generate counterparties
        Self::counterparties_csv()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trade_source::{FrontOffice, TradeSource};
    use tempfile::TempDir;

    #[test]
    fn test_counterparties_csv() {
        let csv = CsvGenerator::counterparties_csv();
        assert!(csv.contains("CP001"));
        assert!(csv.contains("Goldman Sachs"));
    }

    #[test]
    fn test_netting_sets_csv() {
        let csv = CsvGenerator::netting_sets_csv();
        assert!(csv.contains("NS001"));
        assert!(csv.contains("threshold"));
    }

    #[test]
    fn test_trades_to_csv() {
        let fo = FrontOffice::new();
        let trades = fo.generate_trades(10);
        let csv = CsvGenerator::trades_to_csv(&trades);

        assert!(csv.starts_with("trade_id,"));
        let lines: Vec<_> = csv.lines().collect();
        assert_eq!(lines.len(), 11); // Header + 10 trades
    }

    #[test]
    fn test_yield_curve_csv() {
        let rates = vec![(0.25, 4.5), (0.5, 4.6), (1.0, 4.7)];
        let csv = CsvGenerator::yield_curve_csv("USD", &rates);

        assert!(csv.contains("USD"));
        assert!(csv.contains("0.2500"));
        let lines: Vec<_> = csv.lines().collect();
        assert_eq!(lines.len(), 4); // Header + 3 rates
    }

    #[test]
    fn test_spot_rates_csv() {
        let csv = CsvGenerator::spot_rates_csv();
        assert!(csv.contains("AAPL"));
        assert!(csv.contains("ticker,spot_price"));
    }

    #[test]
    fn test_bulk_data_generator() {
        let temp_dir = TempDir::new().unwrap();
        let generator = BulkDataGenerator::new(temp_dir.path());

        let fo = FrontOffice::new();
        let trades = fo.generate_trades(50);

        let summary = generator.generate_all(&trades).unwrap();

        assert_eq!(summary.files_created, 8);
        assert_eq!(summary.trades_written, 50);

        // Verify directory structure
        assert!(temp_dir.path().join("counterparties/counterparties.csv").exists());
        assert!(temp_dir.path().join("counterparties/netting_sets.csv").exists());
        assert!(temp_dir.path().join("market_data/spot_rates.csv").exists());
        assert!(temp_dir.path().join("market_data/credit_spreads.csv").exists());
        assert!(temp_dir.path().join("trades/trades.csv").exists());
        assert!(temp_dir.path().join("config/csa_agreements.csv").exists());
        assert!(temp_dir.path().join("config/holidays.csv").exists());
    }

    #[test]
    fn test_bulk_counterparty_files() {
        let temp_dir = TempDir::new().unwrap();
        let generator = BulkDataGenerator::new(temp_dir.path());

        let count = generator.generate_counterparty_files().unwrap();

        assert_eq!(count, 2);
        assert!(temp_dir.path().join("counterparties/counterparties.csv").exists());
        assert!(temp_dir.path().join("counterparties/netting_sets.csv").exists());
    }

    #[test]
    fn test_bulk_market_data_files() {
        let temp_dir = TempDir::new().unwrap();
        let generator = BulkDataGenerator::new(temp_dir.path());

        let yield_curves = vec![
            ("USD", vec![(1.0, 4.5), (5.0, 4.7)]),
            ("EUR", vec![(1.0, 3.0), (5.0, 3.2)]),
        ];
        let count = generator.generate_market_data_files(&yield_curves).unwrap();

        assert_eq!(count, 5); // 3 base files + 2 yield curves
        assert!(temp_dir.path().join("market_data/spot_rates.csv").exists());
        assert!(temp_dir.path().join("market_data/yield_curve_usd.csv").exists());
        assert!(temp_dir.path().join("market_data/yield_curve_eur.csv").exists());
    }

    #[test]
    fn test_bulk_trade_files() {
        let temp_dir = TempDir::new().unwrap();
        let generator = BulkDataGenerator::new(temp_dir.path());

        let fo = FrontOffice::new();
        let trades = fo.generate_trades(20);

        let count = generator.generate_trade_files(&trades).unwrap();

        assert_eq!(count, 1);
        assert!(temp_dir.path().join("trades/trades.csv").exists());

        // Verify content
        let content = std::fs::read_to_string(temp_dir.path().join("trades/trades.csv")).unwrap();
        let lines: Vec<_> = content.lines().collect();
        assert_eq!(lines.len(), 21); // Header + 20 trades
    }
}
