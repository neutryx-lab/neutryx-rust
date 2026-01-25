//! ボラティリティクォート構造体。
//!
//! # Requirements: 2.2, 2.6, 10.3
//!
//! マーケットボラティリティクォートを表現するデータ構造を提供する。
//! bid/ask/mid価格、strike表現の種類、クォートタイプを管理。

use chrono::NaiveDate;
use num_traits::Float;
use serde::{Deserialize, Serialize};

use super::types::InstrumentId;

/// Strike表現方式。
///
/// # Requirements: 2.2
///
/// マーケットデータのstrike表現を指定する。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Strike {
    /// 絶対strike値（K）。
    Absolute(f64),
    /// ATMからの相対値（bps単位）。
    RelativeToAtm(f64),
    /// Moneyness（K/F）。
    Moneyness(f64),
    /// Log-moneyness（ln(K/F)）。
    LogMoneyness(f64),
}

impl Strike {
    /// 絶対strikeへの変換。
    ///
    /// # Arguments
    /// * `forward` - Forward rate/price
    /// * `atm_strike` - ATM strike（RelativeToAtmの場合に使用）
    pub fn to_absolute(&self, forward: f64, atm_strike: Option<f64>) -> f64 {
        match *self {
            Strike::Absolute(k) => k,
            Strike::RelativeToAtm(bps) => {
                let atm = atm_strike.unwrap_or(forward);
                atm + bps * 0.0001 // bps to decimal
            }
            Strike::Moneyness(m) => m * forward,
            Strike::LogMoneyness(lm) => forward * lm.exp(),
        }
    }

    /// Strike値を取得（内部値）。
    pub fn value(&self) -> f64 {
        match *self {
            Strike::Absolute(v)
            | Strike::RelativeToAtm(v)
            | Strike::Moneyness(v)
            | Strike::LogMoneyness(v) => v,
        }
    }

    /// Strikeがゼロまたは負かどうかを検証。
    pub fn is_valid(&self) -> bool {
        match *self {
            Strike::Absolute(k) => k > 0.0,
            Strike::RelativeToAtm(_) => true, // relative can be negative
            Strike::Moneyness(m) => m > 0.0,
            Strike::LogMoneyness(_) => true, // log-moneyness can be any real
        }
    }
}

impl Default for Strike {
    fn default() -> Self { Strike::Absolute(0.0) }
}

/// クォートタイプ（ボラティリティ種別）。
///
/// # Requirements: 2.2
///
/// マーケットクォートのボラティリティ表現方式を指定する。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum QuoteType {
    /// 正規ボラティリティ（Normal vol）。
    Normal,
    /// 対数正規ボラティリティ（Black vol）。
    #[default]
    LogNormal,
    /// シフト付き対数正規ボラティリティ（負金利対応）。
    ShiftedLogNormal {
        /// シフト値（正の値で下方シフト）。
        shift: f64,
    },
}

impl QuoteType {
    /// シフト値を取得（LogNormalとNormalは0.0）。
    pub fn shift(&self) -> f64 {
        match *self {
            QuoteType::Normal | QuoteType::LogNormal => 0.0,
            QuoteType::ShiftedLogNormal { shift } => shift,
        }
    }

    /// シフト付き対数正規かどうか。
    pub fn is_shifted(&self) -> bool { matches!(self, QuoteType::ShiftedLogNormal { .. }) }

    /// 正規ボラティリティかどうか。
    pub fn is_normal(&self) -> bool { matches!(self, QuoteType::Normal) }
}

/// Tenor表現。
///
/// 年単位での期間を表す。
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Tenor(pub f64);

impl Tenor {
    /// 年単位で新しいTenorを作成。
    pub fn years(y: f64) -> Self { Tenor(y) }

    /// 月単位でTenorを作成。
    pub fn months(m: u32) -> Self { Tenor(m as f64 / 12.0) }

    /// 年単位の値を取得。
    pub fn as_years(&self) -> f64 { self.0 }

    /// 月単位の値を取得（概算）。
    pub fn as_months(&self) -> f64 { self.0 * 12.0 }
}

impl Default for Tenor {
    fn default() -> Self { Tenor(1.0) }
}

impl From<f64> for Tenor {
    fn from(years: f64) -> Self { Tenor(years) }
}

/// Underlying index（金利指標）。
///
/// # Requirements: 2.6
///
/// VolCubeの対象となる金利指標を指定する。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum UnderlyingIndex {
    /// SOFR（USD）。
    #[default]
    Sofr,
    /// ESTR（EUR）。
    Estr,
    /// TONA（JPY）。
    Tona,
    /// EURIBOR（EUR legacy）。
    Euribor,
    /// LIBOR（legacy）。
    Libor,
    /// その他。
    Other(u32),
}

impl UnderlyingIndex {
    /// 指標名を文字列で取得。
    pub fn as_str(&self) -> &'static str {
        match self {
            UnderlyingIndex::Sofr => "SOFR",
            UnderlyingIndex::Estr => "ESTR",
            UnderlyingIndex::Tona => "TONA",
            UnderlyingIndex::Euribor => "EURIBOR",
            UnderlyingIndex::Libor => "LIBOR",
            UnderlyingIndex::Other(_) => "OTHER",
        }
    }
}

/// 通貨。
///
/// # Requirements: 2.6
///
/// ISO 4217通貨コード。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Currency {
    /// 米ドル。
    #[default]
    Usd,
    /// ユーロ。
    Eur,
    /// 日本円。
    Jpy,
    /// 英ポンド。
    Gbp,
    /// スイスフラン。
    Chf,
    /// その他。
    Other(u32),
}

impl Currency {
    /// 通貨コードを文字列で取得。
    pub fn as_str(&self) -> &'static str {
        match self {
            Currency::Usd => "USD",
            Currency::Eur => "EUR",
            Currency::Jpy => "JPY",
            Currency::Gbp => "GBP",
            Currency::Chf => "CHF",
            Currency::Other(_) => "OTH",
        }
    }

    /// デフォルトの金利指標を取得。
    pub fn default_index(&self) -> UnderlyingIndex {
        match self {
            Currency::Usd => UnderlyingIndex::Sofr,
            Currency::Eur => UnderlyingIndex::Estr,
            Currency::Jpy => UnderlyingIndex::Tona,
            Currency::Gbp => UnderlyingIndex::Sofr, // SONIA similar
            Currency::Chf => UnderlyingIndex::Sofr, // SARON similar
            Currency::Other(_) => UnderlyingIndex::Libor,
        }
    }
}

/// マーケットボラティリティクォート。
///
/// # Requirements: 2.2, 10.3
///
/// 単一のボラティリティクォートを表す。
/// bid/ask/mid価格、strike表現、クォートタイプを保持。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VolQuote {
    /// Instrument識別子。
    pub instrument_id: InstrumentId,
    /// Expiry日。
    pub expiry: NaiveDate,
    /// Tenor（underlying期間）。
    pub tenor: Tenor,
    /// Strike。
    pub strike: Strike,
    /// Bid価格（Option: 存在しない場合あり）。
    pub bid: Option<f64>,
    /// Ask価格（Option: 存在しない場合あり）。
    pub ask: Option<f64>,
    /// Mid価格（必須）。
    pub mid: f64,
    /// クォートタイプ。
    pub quote_type: QuoteType,
}

impl VolQuote {
    /// 新しいVolQuoteを作成。
    pub fn new(
        instrument_id: impl Into<InstrumentId>,
        expiry: NaiveDate,
        tenor: Tenor,
        strike: Strike,
        mid: f64,
    ) -> Self {
        Self {
            instrument_id: instrument_id.into(),
            expiry,
            tenor,
            strike,
            bid: None,
            ask: None,
            mid,
            quote_type: QuoteType::default(),
        }
    }

    /// Bid価格を設定。
    pub fn with_bid(mut self, bid: f64) -> Self {
        self.bid = Some(bid);
        self
    }

    /// Ask価格を設定。
    pub fn with_ask(mut self, ask: f64) -> Self {
        self.ask = Some(ask);
        self
    }

    /// Bid/Ask両方を設定。
    pub fn with_bid_ask(mut self, bid: f64, ask: f64) -> Self {
        self.bid = Some(bid);
        self.ask = Some(ask);
        self
    }

    /// クォートタイプを設定。
    pub fn with_quote_type(mut self, quote_type: QuoteType) -> Self {
        self.quote_type = quote_type;
        self
    }

    /// Bid-Askスプレッドを計算。
    pub fn spread(&self) -> Option<f64> {
        match (self.bid, self.ask) {
            (Some(b), Some(a)) => Some(a - b),
            _ => None,
        }
    }

    /// クォートを検証。
    pub fn validate(&self) -> Result<(), String> {
        if self.mid <= 0.0 {
            return Err("Mid volatility must be positive".to_string());
        }
        if let Some(bid) = self.bid {
            if bid < 0.0 {
                return Err("Bid volatility must be non-negative".to_string());
            }
            if bid > self.mid {
                return Err("Bid must be less than or equal to mid".to_string());
            }
        }
        if let Some(ask) = self.ask {
            if ask < self.mid {
                return Err("Ask must be greater than or equal to mid".to_string());
            }
        }
        if let (Some(bid), Some(ask)) = (self.bid, self.ask) {
            if bid > ask {
                return Err("Bid must be less than or equal to ask".to_string());
            }
        }
        if !self.strike.is_valid() {
            if matches!(self.strike, Strike::Absolute(_) | Strike::Moneyness(_)) {
                return Err("Strike must be positive for Absolute/Moneyness types".to_string());
            }
        }
        Ok(())
    }
}

/// クォートセット（VolCubeBuilder入力）。
///
/// # Requirements: 2.1, 2.3, 2.6
///
/// 複数のボラティリティクォートを集約する。
/// 通貨、underlying index、基準日をメタデータとして保持。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VolQuoteSet {
    /// クォート一覧。
    pub quotes: Vec<VolQuote>,
    /// 通貨。
    pub currency: Currency,
    /// Underlying index。
    pub underlying_index: UnderlyingIndex,
    /// 基準日（as-of date）。
    pub as_of_date: NaiveDate,
}

impl VolQuoteSet {
    /// 新しいVolQuoteSetを作成。
    pub fn new(currency: Currency, underlying_index: UnderlyingIndex, as_of_date: NaiveDate) -> Self {
        Self {
            quotes: Vec::new(),
            currency,
            underlying_index,
            as_of_date,
        }
    }

    /// クォートを追加。
    pub fn add_quote(&mut self, quote: VolQuote) { self.quotes.push(quote); }

    /// クォートを追加（builder pattern）。
    pub fn with_quote(mut self, quote: VolQuote) -> Self {
        self.quotes.push(quote);
        self
    }

    /// 複数クォートを追加。
    pub fn with_quotes(mut self, quotes: impl IntoIterator<Item = VolQuote>) -> Self {
        self.quotes.extend(quotes);
        self
    }

    /// クォート数を取得。
    pub fn len(&self) -> usize { self.quotes.len() }

    /// クォートが空かどうか。
    pub fn is_empty(&self) -> bool { self.quotes.is_empty() }

    /// 全クォートを検証。
    pub fn validate(&self) -> Result<(), String> {
        for (i, quote) in self.quotes.iter().enumerate() {
            quote
                .validate()
                .map_err(|e| format!("Quote {}: {}", i, e))?;
        }
        Ok(())
    }

    /// Expiry一覧を取得（重複なし、ソート済み）。
    pub fn unique_expiries(&self) -> Vec<NaiveDate> {
        let mut expiries: Vec<_> = self.quotes.iter().map(|q| q.expiry).collect();
        expiries.sort();
        expiries.dedup();
        expiries
    }

    /// Tenor一覧を取得（重複なし、ソート済み）。
    pub fn unique_tenors(&self) -> Vec<Tenor> {
        let mut tenors: Vec<_> = self.quotes.iter().map(|q| q.tenor).collect();
        tenors.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        tenors.dedup_by(|a, b| (a.0 - b.0).abs() < 1e-10);
        tenors
    }

    /// 特定の(expiry, tenor)のクォートをフィルタ。
    pub fn filter_by_expiry_tenor(&self, expiry: NaiveDate, tenor: Tenor) -> Vec<&VolQuote> {
        self.quotes
            .iter()
            .filter(|q| q.expiry == expiry && (q.tenor.0 - tenor.0).abs() < 1e-10)
            .collect()
    }

    /// expiry/tenor/strikeでグループ化。
    pub fn group_by_slice(&self) -> std::collections::HashMap<(NaiveDate, i64), Vec<&VolQuote>> {
        let mut groups = std::collections::HashMap::new();
        for quote in &self.quotes {
            let tenor_key = (quote.tenor.0 * 1000.0) as i64; // milliyears for hashing
            groups
                .entry((quote.expiry, tenor_key))
                .or_insert_with(Vec::new)
                .push(quote);
        }
        groups
    }
}

/// VolQuoteからVolInstrument<f64>への変換ヘルパー。
///
/// 基準日からのexpiry年数とforward rateを計算してVolInstrumentに変換する。
pub fn vol_quote_to_instrument<T: Float>(
    quote: &VolQuote,
    as_of_date: NaiveDate,
    forward: T,
) -> super::types::VolInstrument<T> {
    let expiry_days = (quote.expiry - as_of_date).num_days();
    let expiry_years = T::from(expiry_days).unwrap() / T::from(365.0).unwrap();
    let tenor_years = T::from(quote.tenor.0).unwrap();
    let strike_abs = T::from(quote.strike.to_absolute(forward.to_f64().unwrap(), None)).unwrap();
    let implied_vol = T::from(quote.mid).unwrap();

    super::types::VolInstrument::new(
        quote.instrument_id.clone(),
        expiry_years,
        tenor_years,
        strike_abs,
        implied_vol,
        forward,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Strike Tests
    // =========================================================================

    #[test]
    fn test_strike_absolute() {
        let strike = Strike::Absolute(0.03);
        assert_eq!(strike.value(), 0.03);
        assert!(strike.is_valid());
        assert_eq!(strike.to_absolute(0.025, None), 0.03);
    }

    #[test]
    fn test_strike_absolute_invalid() {
        let strike = Strike::Absolute(-0.01);
        assert!(!strike.is_valid());
    }

    #[test]
    fn test_strike_relative_to_atm() {
        let strike = Strike::RelativeToAtm(50.0); // +50 bps
        assert_eq!(strike.value(), 50.0);
        assert!(strike.is_valid());
        // ATM = 0.03, +50bps = 0.03 + 0.005 = 0.035
        assert!((strike.to_absolute(0.03, Some(0.03)) - 0.035).abs() < 1e-10);
    }

    #[test]
    fn test_strike_relative_negative() {
        let strike = Strike::RelativeToAtm(-25.0); // -25 bps
        assert!(strike.is_valid()); // relative can be negative
        assert!((strike.to_absolute(0.03, Some(0.03)) - 0.0275).abs() < 1e-10);
    }

    #[test]
    fn test_strike_moneyness() {
        let strike = Strike::Moneyness(1.1); // K = 1.1 * F
        assert_eq!(strike.value(), 1.1);
        assert!(strike.is_valid());
        assert!((strike.to_absolute(0.03, None) - 0.033).abs() < 1e-10);
    }

    #[test]
    fn test_strike_moneyness_invalid() {
        let strike = Strike::Moneyness(0.0);
        assert!(!strike.is_valid());
    }

    #[test]
    fn test_strike_log_moneyness() {
        let strike = Strike::LogMoneyness(0.1); // ln(K/F) = 0.1 => K = F * exp(0.1)
        assert_eq!(strike.value(), 0.1);
        assert!(strike.is_valid());
        let expected = 0.03 * (0.1_f64).exp();
        assert!((strike.to_absolute(0.03, None) - expected).abs() < 1e-10);
    }

    #[test]
    fn test_strike_log_moneyness_negative() {
        let strike = Strike::LogMoneyness(-0.1); // OTM put
        assert!(strike.is_valid()); // log-moneyness can be negative
    }

    #[test]
    fn test_strike_default() {
        let strike = Strike::default();
        assert!(matches!(strike, Strike::Absolute(0.0)));
    }

    #[test]
    fn test_strike_serde() {
        let strike = Strike::Moneyness(1.05);
        let json = serde_json::to_string(&strike).unwrap();
        let deserialized: Strike = serde_json::from_str(&json).unwrap();
        assert_eq!(strike, deserialized);
    }

    // =========================================================================
    // QuoteType Tests
    // =========================================================================

    #[test]
    fn test_quote_type_default() {
        let qt = QuoteType::default();
        assert!(matches!(qt, QuoteType::LogNormal));
    }

    #[test]
    fn test_quote_type_shift() {
        assert_eq!(QuoteType::Normal.shift(), 0.0);
        assert_eq!(QuoteType::LogNormal.shift(), 0.0);
        assert_eq!(QuoteType::ShiftedLogNormal { shift: 0.03 }.shift(), 0.03);
    }

    #[test]
    fn test_quote_type_is_shifted() {
        assert!(!QuoteType::Normal.is_shifted());
        assert!(!QuoteType::LogNormal.is_shifted());
        assert!(QuoteType::ShiftedLogNormal { shift: 0.03 }.is_shifted());
    }

    #[test]
    fn test_quote_type_is_normal() {
        assert!(QuoteType::Normal.is_normal());
        assert!(!QuoteType::LogNormal.is_normal());
        assert!(!QuoteType::ShiftedLogNormal { shift: 0.03 }.is_normal());
    }

    #[test]
    fn test_quote_type_serde() {
        let qt = QuoteType::ShiftedLogNormal { shift: 0.02 };
        let json = serde_json::to_string(&qt).unwrap();
        let deserialized: QuoteType = serde_json::from_str(&json).unwrap();
        assert_eq!(qt, deserialized);
    }

    // =========================================================================
    // Tenor Tests
    // =========================================================================

    #[test]
    fn test_tenor_years() {
        let tenor = Tenor::years(5.0);
        assert_eq!(tenor.as_years(), 5.0);
        assert!((tenor.as_months() - 60.0).abs() < 1e-10);
    }

    #[test]
    fn test_tenor_months() {
        let tenor = Tenor::months(6);
        assert!((tenor.as_years() - 0.5).abs() < 1e-10);
        assert!((tenor.as_months() - 6.0).abs() < 1e-10);
    }

    #[test]
    fn test_tenor_from_f64() {
        let tenor: Tenor = 2.5.into();
        assert_eq!(tenor.as_years(), 2.5);
    }

    #[test]
    fn test_tenor_default() {
        let tenor = Tenor::default();
        assert_eq!(tenor.as_years(), 1.0);
    }

    #[test]
    fn test_tenor_ord() {
        let t1 = Tenor::years(1.0);
        let t2 = Tenor::years(2.0);
        assert!(t1 < t2);
    }

    // =========================================================================
    // UnderlyingIndex Tests
    // =========================================================================

    #[test]
    fn test_underlying_index_default() {
        let idx = UnderlyingIndex::default();
        assert!(matches!(idx, UnderlyingIndex::Sofr));
    }

    #[test]
    fn test_underlying_index_as_str() {
        assert_eq!(UnderlyingIndex::Sofr.as_str(), "SOFR");
        assert_eq!(UnderlyingIndex::Estr.as_str(), "ESTR");
        assert_eq!(UnderlyingIndex::Tona.as_str(), "TONA");
        assert_eq!(UnderlyingIndex::Euribor.as_str(), "EURIBOR");
        assert_eq!(UnderlyingIndex::Libor.as_str(), "LIBOR");
        assert_eq!(UnderlyingIndex::Other(99).as_str(), "OTHER");
    }

    // =========================================================================
    // Currency Tests
    // =========================================================================

    #[test]
    fn test_currency_default() {
        let ccy = Currency::default();
        assert!(matches!(ccy, Currency::Usd));
    }

    #[test]
    fn test_currency_as_str() {
        assert_eq!(Currency::Usd.as_str(), "USD");
        assert_eq!(Currency::Eur.as_str(), "EUR");
        assert_eq!(Currency::Jpy.as_str(), "JPY");
        assert_eq!(Currency::Gbp.as_str(), "GBP");
        assert_eq!(Currency::Chf.as_str(), "CHF");
        assert_eq!(Currency::Other(99).as_str(), "OTH");
    }

    #[test]
    fn test_currency_default_index() {
        assert!(matches!(Currency::Usd.default_index(), UnderlyingIndex::Sofr));
        assert!(matches!(Currency::Eur.default_index(), UnderlyingIndex::Estr));
        assert!(matches!(Currency::Jpy.default_index(), UnderlyingIndex::Tona));
    }

    // =========================================================================
    // VolQuote Tests
    // =========================================================================

    #[test]
    fn test_vol_quote_new() {
        let expiry = NaiveDate::from_ymd_opt(2027, 1, 25).unwrap();
        let quote = VolQuote::new("TEST-1", expiry, Tenor::years(5.0), Strike::Absolute(0.03), 0.20);

        assert_eq!(quote.instrument_id.as_str(), "TEST-1");
        assert_eq!(quote.expiry, expiry);
        assert_eq!(quote.tenor.as_years(), 5.0);
        assert_eq!(quote.mid, 0.20);
        assert!(quote.bid.is_none());
        assert!(quote.ask.is_none());
        assert!(matches!(quote.quote_type, QuoteType::LogNormal));
    }

    #[test]
    fn test_vol_quote_with_bid_ask() {
        let expiry = NaiveDate::from_ymd_opt(2027, 1, 25).unwrap();
        let quote = VolQuote::new("TEST-1", expiry, Tenor::years(5.0), Strike::Absolute(0.03), 0.20)
            .with_bid(0.19)
            .with_ask(0.21);

        assert_eq!(quote.bid, Some(0.19));
        assert_eq!(quote.ask, Some(0.21));
        let spread = quote.spread().unwrap();
        assert!((spread - 0.02).abs() < 1e-10);
    }

    #[test]
    fn test_vol_quote_with_bid_ask_combined() {
        let expiry = NaiveDate::from_ymd_opt(2027, 1, 25).unwrap();
        let quote = VolQuote::new("TEST-1", expiry, Tenor::years(5.0), Strike::Absolute(0.03), 0.20)
            .with_bid_ask(0.18, 0.22);

        assert_eq!(quote.bid, Some(0.18));
        assert_eq!(quote.ask, Some(0.22));
    }

    #[test]
    fn test_vol_quote_with_quote_type() {
        let expiry = NaiveDate::from_ymd_opt(2027, 1, 25).unwrap();
        let quote = VolQuote::new("TEST-1", expiry, Tenor::years(5.0), Strike::Absolute(0.03), 0.20)
            .with_quote_type(QuoteType::Normal);

        assert!(matches!(quote.quote_type, QuoteType::Normal));
    }

    #[test]
    fn test_vol_quote_spread_none() {
        let expiry = NaiveDate::from_ymd_opt(2027, 1, 25).unwrap();
        let quote = VolQuote::new("TEST-1", expiry, Tenor::years(5.0), Strike::Absolute(0.03), 0.20);

        assert!(quote.spread().is_none());
    }

    #[test]
    fn test_vol_quote_validate_valid() {
        let expiry = NaiveDate::from_ymd_opt(2027, 1, 25).unwrap();
        let quote = VolQuote::new("TEST-1", expiry, Tenor::years(5.0), Strike::Absolute(0.03), 0.20)
            .with_bid_ask(0.19, 0.21);

        assert!(quote.validate().is_ok());
    }

    #[test]
    fn test_vol_quote_validate_negative_mid() {
        let expiry = NaiveDate::from_ymd_opt(2027, 1, 25).unwrap();
        let quote = VolQuote::new("TEST-1", expiry, Tenor::years(5.0), Strike::Absolute(0.03), -0.20);

        let result = quote.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Mid"));
    }

    #[test]
    fn test_vol_quote_validate_bid_greater_than_mid() {
        let expiry = NaiveDate::from_ymd_opt(2027, 1, 25).unwrap();
        let quote = VolQuote::new("TEST-1", expiry, Tenor::years(5.0), Strike::Absolute(0.03), 0.20)
            .with_bid(0.25);

        let result = quote.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Bid"));
    }

    #[test]
    fn test_vol_quote_validate_ask_less_than_mid() {
        let expiry = NaiveDate::from_ymd_opt(2027, 1, 25).unwrap();
        let quote = VolQuote::new("TEST-1", expiry, Tenor::years(5.0), Strike::Absolute(0.03), 0.20)
            .with_ask(0.15);

        let result = quote.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Ask"));
    }

    #[test]
    fn test_vol_quote_validate_bid_greater_than_ask() {
        let expiry = NaiveDate::from_ymd_opt(2027, 1, 25).unwrap();
        let quote = VolQuote::new("TEST-1", expiry, Tenor::years(5.0), Strike::Absolute(0.03), 0.20)
            .with_bid_ask(0.21, 0.19);

        let result = quote.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_vol_quote_serde() {
        let expiry = NaiveDate::from_ymd_opt(2027, 1, 25).unwrap();
        let quote = VolQuote::new("TEST-1", expiry, Tenor::years(5.0), Strike::Absolute(0.03), 0.20)
            .with_bid_ask(0.19, 0.21)
            .with_quote_type(QuoteType::ShiftedLogNormal { shift: 0.02 });

        let json = serde_json::to_string(&quote).unwrap();
        let deserialized: VolQuote = serde_json::from_str(&json).unwrap();
        assert_eq!(quote.instrument_id, deserialized.instrument_id);
        assert_eq!(quote.mid, deserialized.mid);
        assert_eq!(quote.bid, deserialized.bid);
        assert_eq!(quote.ask, deserialized.ask);
    }

    // =========================================================================
    // VolQuoteSet Tests
    // =========================================================================

    #[test]
    fn test_vol_quote_set_new() {
        let as_of = NaiveDate::from_ymd_opt(2026, 1, 25).unwrap();
        let qs = VolQuoteSet::new(Currency::Usd, UnderlyingIndex::Sofr, as_of);

        assert!(qs.is_empty());
        assert_eq!(qs.len(), 0);
        assert!(matches!(qs.currency, Currency::Usd));
        assert!(matches!(qs.underlying_index, UnderlyingIndex::Sofr));
        assert_eq!(qs.as_of_date, as_of);
    }

    #[test]
    fn test_vol_quote_set_add_quote() {
        let as_of = NaiveDate::from_ymd_opt(2026, 1, 25).unwrap();
        let expiry = NaiveDate::from_ymd_opt(2027, 1, 25).unwrap();
        let mut qs = VolQuoteSet::new(Currency::Usd, UnderlyingIndex::Sofr, as_of);

        let quote = VolQuote::new("TEST-1", expiry, Tenor::years(5.0), Strike::Absolute(0.03), 0.20);
        qs.add_quote(quote);

        assert_eq!(qs.len(), 1);
        assert!(!qs.is_empty());
    }

    #[test]
    fn test_vol_quote_set_with_quotes() {
        let as_of = NaiveDate::from_ymd_opt(2026, 1, 25).unwrap();
        let expiry1 = NaiveDate::from_ymd_opt(2027, 1, 25).unwrap();
        let expiry2 = NaiveDate::from_ymd_opt(2028, 1, 25).unwrap();

        let quotes = vec![
            VolQuote::new("TEST-1", expiry1, Tenor::years(5.0), Strike::Absolute(0.03), 0.20),
            VolQuote::new("TEST-2", expiry2, Tenor::years(10.0), Strike::Absolute(0.035), 0.22),
        ];

        let qs = VolQuoteSet::new(Currency::Usd, UnderlyingIndex::Sofr, as_of).with_quotes(quotes);

        assert_eq!(qs.len(), 2);
    }

    #[test]
    fn test_vol_quote_set_validate() {
        let as_of = NaiveDate::from_ymd_opt(2026, 1, 25).unwrap();
        let expiry = NaiveDate::from_ymd_opt(2027, 1, 25).unwrap();

        let quotes = vec![
            VolQuote::new("TEST-1", expiry, Tenor::years(5.0), Strike::Absolute(0.03), 0.20),
            VolQuote::new("TEST-2", expiry, Tenor::years(10.0), Strike::Absolute(0.035), -0.22), // invalid
        ];

        let qs = VolQuoteSet::new(Currency::Usd, UnderlyingIndex::Sofr, as_of).with_quotes(quotes);

        let result = qs.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Quote 1"));
    }

    #[test]
    fn test_vol_quote_set_unique_expiries() {
        let as_of = NaiveDate::from_ymd_opt(2026, 1, 25).unwrap();
        let expiry1 = NaiveDate::from_ymd_opt(2027, 1, 25).unwrap();
        let expiry2 = NaiveDate::from_ymd_opt(2028, 1, 25).unwrap();

        let quotes = vec![
            VolQuote::new("TEST-1", expiry1, Tenor::years(5.0), Strike::Absolute(0.03), 0.20),
            VolQuote::new("TEST-2", expiry1, Tenor::years(10.0), Strike::Absolute(0.035), 0.22),
            VolQuote::new("TEST-3", expiry2, Tenor::years(5.0), Strike::Absolute(0.03), 0.21),
        ];

        let qs = VolQuoteSet::new(Currency::Usd, UnderlyingIndex::Sofr, as_of).with_quotes(quotes);

        let expiries = qs.unique_expiries();
        assert_eq!(expiries.len(), 2);
        assert_eq!(expiries[0], expiry1);
        assert_eq!(expiries[1], expiry2);
    }

    #[test]
    fn test_vol_quote_set_unique_tenors() {
        let as_of = NaiveDate::from_ymd_opt(2026, 1, 25).unwrap();
        let expiry = NaiveDate::from_ymd_opt(2027, 1, 25).unwrap();

        let quotes = vec![
            VolQuote::new("TEST-1", expiry, Tenor::years(5.0), Strike::Absolute(0.03), 0.20),
            VolQuote::new("TEST-2", expiry, Tenor::years(10.0), Strike::Absolute(0.035), 0.22),
            VolQuote::new("TEST-3", expiry, Tenor::years(5.0), Strike::Absolute(0.04), 0.21),
        ];

        let qs = VolQuoteSet::new(Currency::Usd, UnderlyingIndex::Sofr, as_of).with_quotes(quotes);

        let tenors = qs.unique_tenors();
        assert_eq!(tenors.len(), 2);
        assert_eq!(tenors[0].as_years(), 5.0);
        assert_eq!(tenors[1].as_years(), 10.0);
    }

    #[test]
    fn test_vol_quote_set_filter_by_expiry_tenor() {
        let as_of = NaiveDate::from_ymd_opt(2026, 1, 25).unwrap();
        let expiry1 = NaiveDate::from_ymd_opt(2027, 1, 25).unwrap();
        let expiry2 = NaiveDate::from_ymd_opt(2028, 1, 25).unwrap();

        let quotes = vec![
            VolQuote::new("TEST-1", expiry1, Tenor::years(5.0), Strike::Absolute(0.03), 0.20),
            VolQuote::new("TEST-2", expiry1, Tenor::years(5.0), Strike::Absolute(0.035), 0.22),
            VolQuote::new("TEST-3", expiry1, Tenor::years(10.0), Strike::Absolute(0.03), 0.21),
            VolQuote::new("TEST-4", expiry2, Tenor::years(5.0), Strike::Absolute(0.03), 0.19),
        ];

        let qs = VolQuoteSet::new(Currency::Usd, UnderlyingIndex::Sofr, as_of).with_quotes(quotes);

        let filtered = qs.filter_by_expiry_tenor(expiry1, Tenor::years(5.0));
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_vol_quote_set_group_by_slice() {
        let as_of = NaiveDate::from_ymd_opt(2026, 1, 25).unwrap();
        let expiry1 = NaiveDate::from_ymd_opt(2027, 1, 25).unwrap();
        let expiry2 = NaiveDate::from_ymd_opt(2028, 1, 25).unwrap();

        let quotes = vec![
            VolQuote::new("TEST-1", expiry1, Tenor::years(5.0), Strike::Absolute(0.03), 0.20),
            VolQuote::new("TEST-2", expiry1, Tenor::years(5.0), Strike::Absolute(0.035), 0.22),
            VolQuote::new("TEST-3", expiry1, Tenor::years(10.0), Strike::Absolute(0.03), 0.21),
            VolQuote::new("TEST-4", expiry2, Tenor::years(5.0), Strike::Absolute(0.03), 0.19),
        ];

        let qs = VolQuoteSet::new(Currency::Usd, UnderlyingIndex::Sofr, as_of).with_quotes(quotes);

        let groups = qs.group_by_slice();
        assert_eq!(groups.len(), 3); // (expiry1, 5Y), (expiry1, 10Y), (expiry2, 5Y)
    }

    #[test]
    fn test_vol_quote_set_serde() {
        let as_of = NaiveDate::from_ymd_opt(2026, 1, 25).unwrap();
        let expiry = NaiveDate::from_ymd_opt(2027, 1, 25).unwrap();

        let qs = VolQuoteSet::new(Currency::Usd, UnderlyingIndex::Sofr, as_of).with_quote(
            VolQuote::new("TEST-1", expiry, Tenor::years(5.0), Strike::Absolute(0.03), 0.20),
        );

        let json = serde_json::to_string(&qs).unwrap();
        let deserialized: VolQuoteSet = serde_json::from_str(&json).unwrap();
        assert_eq!(qs.len(), deserialized.len());
        assert_eq!(qs.currency, deserialized.currency);
    }

    // =========================================================================
    // vol_quote_to_instrument Tests
    // =========================================================================

    #[test]
    fn test_vol_quote_to_instrument() {
        let as_of = NaiveDate::from_ymd_opt(2026, 1, 25).unwrap();
        let expiry = NaiveDate::from_ymd_opt(2027, 1, 25).unwrap(); // ~1 year

        let quote = VolQuote::new("TEST-1", expiry, Tenor::years(5.0), Strike::Absolute(0.03), 0.20);

        let instrument = vol_quote_to_instrument(&quote, as_of, 0.025_f64);

        assert_eq!(instrument.instrument_id.as_str(), "TEST-1");
        assert!((instrument.expiry - 1.0).abs() < 0.01); // ~1 year
        assert_eq!(instrument.tenor, 5.0);
        assert_eq!(instrument.strike, 0.03);
        assert_eq!(instrument.implied_vol, 0.20);
        assert_eq!(instrument.forward, 0.025);
    }
}
